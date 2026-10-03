#![deny(unsafe_code)]
#![cfg_attr(
    not(test),
    deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)
)]

use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use indexer::{AdaptiveProviderHealth, JsonRpcTransport, ProviderError};
use serde_json::{Value, json};
use settlement_client::{
    COMPUTE_BUDGET_PROGRAM_ID, CompiledRevenueStreamInstruction,
    CompiledSettlementAdjustmentInstruction, CompiledSettlementInstruction,
    CompiledSettlementRefundCancellationInstruction, CompiledSettlementRefundInstruction,
    ComputeBudgetPolicy, MAX_SETTLEMENT_SIGNED_BYTES,
    estimate_legacy_settlement_wire_bytes_with_compute_budget,
};
use sha2::{Digest, Sha256};
use solana_address_lookup_table_interface::{
    program as address_lookup_program, state::AddressLookupTable,
};
use solana_transaction::Address;
use std::{sync::Arc, time::Duration};
use thiserror::Error;

pub const MAX_SIGNED_TRANSACTION_BYTES: usize = 1232;
pub const MAX_PROVIDER_ATTEMPTS: usize = 3;
pub const DEFAULT_ATTEMPT_TIMEOUT: Duration = Duration::from_secs(2);
pub const MAX_SIGNATURE_BYTES: usize = 128;
pub const MAX_ADDRESS_LOOKUP_TABLES: usize = 2;
pub const MAX_LOADED_ADDRESSES_PER_TABLE: usize = 32;
pub const MAX_ADDRESS_LOOKUP_TABLE_ADDRESSES: usize = 256;
pub const ADJUSTMENT_REGISTRY_ACCOUNT_DATA_LEN: usize = 8 + (5 * 32) + 2;
pub const MAX_ADJUSTMENT_REGISTRY_RPC_RESULT_BYTES: usize = 16 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdjustmentRegistryVerificationRequest {
    pub merchant: String,
    pub mint: String,
    pub proposer: String,
    pub approver: String,
    /// When present, finalized account data must also bind the governance
    /// authority used by governed cancellation. Existing adjustment evidence
    /// callers may omit this until they have a governance binding contract.
    pub governance: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdjustmentRegistryVerification {
    pub registry: String,
    pub merchant: String,
    pub mint: String,
    pub proposer: String,
    pub approver: String,
    pub governance: Option<String>,
    pub finalized_slot: u64,
    pub account_digest: [u8; 32],
    pub commitment: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedAddressLookupTable {
    pub table: String,
    pub addresses: Vec<String>,
}

#[async_trait]
pub trait AddressLookupTableResolver: Send + Sync {
    async fn resolve(&self, table: &str) -> Result<ResolvedAddressLookupTable, CoordinatorError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedTransaction {
    canonical_base64: String,
    bytes: Vec<u8>,
}

impl SignedTransaction {
    pub fn parse(value: &str) -> Result<Self, CoordinatorError> {
        if value.is_empty() || value.len() > 4 * MAX_SIGNED_TRANSACTION_BYTES {
            return Err(CoordinatorError::InvalidSignedTransaction(
                "signed transaction encoding is empty or too large".to_string(),
            ));
        }
        let bytes = BASE64
            .decode(value)
            .map_err(|_| CoordinatorError::InvalidSignedTransaction("invalid base64".into()))?;
        if bytes.is_empty() || bytes.len() > MAX_SIGNED_TRANSACTION_BYTES {
            return Err(CoordinatorError::InvalidSignedTransaction(
                "signed transaction bytes exceed the bounded packet size".to_string(),
            ));
        }
        let canonical_base64 = BASE64.encode(&bytes);
        if canonical_base64 != value {
            return Err(CoordinatorError::InvalidSignedTransaction(
                "signed transaction must use canonical base64".to_string(),
            ));
        }
        Ok(Self {
            canonical_base64,
            bytes,
        })
    }

    pub fn canonical_base64(&self) -> &str {
        &self.canonical_base64
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Returns the transaction's first verified Ed25519 signature in Solana's
    /// canonical base58 representation. This is the durable reconciliation
    /// identifier; the signed message bytes themselves are never persisted.
    pub fn signature(&self) -> Result<String, CoordinatorError> {
        let transaction = wincode::deserialize_exact::<
            solana_transaction::versioned::VersionedTransaction,
        >(self.bytes())
        .map_err(|error| {
            CoordinatorError::InvalidSignedTransaction(format!(
                "unable to decode versioned transaction: {error}"
            ))
        })?;
        transaction.verify_and_hash_message().map_err(|error| {
            CoordinatorError::InvalidSignedTransaction(format!(
                "transaction signatures or message are invalid: {error}"
            ))
        })?;
        transaction
            .signatures
            .first()
            .map(ToString::to_string)
            .ok_or_else(|| {
                CoordinatorError::InvalidSignedTransaction(
                    "signed transaction does not contain a payer signature".into(),
                )
            })
    }

    /// Proves that the wallet signed the exact settlement instruction emitted
    /// by the compiler. The transaction is deliberately parsed and verified at
    /// this boundary so no caller-controlled digest can substitute for the
    /// actual Solana message.
    pub fn verify_settlement_instruction(
        &self,
        expected: &CompiledSettlementInstruction,
    ) -> Result<(), CoordinatorError> {
        self.verify_settlement_instruction_with_lookup_tables(expected, &[])
    }

    /// Proves that a wallet signed exactly the server-compiled record-only
    /// adjustment instruction. Adjustment recording intentionally has a
    /// different wire contract from settlement: one bounded instruction and
    /// no caller-selected compute-budget or account additions.
    pub fn verify_adjustment_instruction(
        &self,
        expected: &CompiledSettlementAdjustmentInstruction,
    ) -> Result<(), CoordinatorError> {
        if expected.status != "prepared_unsubmitted" {
            return Err(CoordinatorError::InvalidSignedTransaction(
                "adjustment instruction is not prepared for submission".into(),
            ));
        }
        let transaction = self.decode_verified_transaction()?;
        if self.bytes.len() > MAX_SIGNED_TRANSACTION_BYTES {
            return Err(CoordinatorError::InvalidSignedTransaction(
                "signed adjustment exceeds the bounded packet size".into(),
            ));
        }
        let message = &transaction.message;
        if !lookup_table_keys(message)?.is_empty() {
            return Err(CoordinatorError::InvalidSignedTransaction(
                "adjustment recording does not permit address lookup tables".into(),
            ));
        }
        let instructions = message.instructions();
        if instructions.len() != 1 {
            return Err(CoordinatorError::InvalidSignedTransaction(
                "adjustment transaction must contain exactly one instruction".into(),
            ));
        }
        let static_accounts = message.static_account_keys();
        let instruction = &instructions[0];
        let program_index = usize::from(instruction.program_id_index);
        let program_id = static_accounts.get(program_index).ok_or_else(|| {
            CoordinatorError::InvalidSignedTransaction(
                "adjustment program index is out of bounds".into(),
            )
        })?;
        if program_id.to_string() != expected.program_id {
            return Err(CoordinatorError::InvalidSignedTransaction(
                "signed adjustment program id does not match the compiled instruction".into(),
            ));
        }
        let expected_data = BASE64
            .decode(&expected.instruction_data_base64)
            .map_err(|_| {
                CoordinatorError::InvalidSignedTransaction(
                    "compiled adjustment instruction data is not valid base64".into(),
                )
            })?;
        if instruction.data != expected_data {
            return Err(CoordinatorError::InvalidSignedTransaction(
                "signed adjustment instruction data does not match the compiled instruction".into(),
            ));
        }
        if instruction.accounts.len() != expected.accounts.len() {
            return Err(CoordinatorError::InvalidSignedTransaction(
                "signed adjustment account count does not match the compiled instruction".into(),
            ));
        }
        if static_accounts.len() != expected.accounts.len() + 1
            && static_accounts.len() != expected.accounts.len() + 2
        {
            return Err(CoordinatorError::InvalidSignedTransaction(
                "signed adjustment contains unexpected static account keys".into(),
            ));
        }
        let expected_keys = expected
            .accounts
            .iter()
            .map(|account| account.pubkey.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        if expected_keys.len() != expected.accounts.len() {
            return Err(CoordinatorError::InvalidSignedTransaction(
                "compiled adjustment contains duplicate account keys".into(),
            ));
        }
        let fee_payer_key = static_accounts.first().map(ToString::to_string);
        if static_accounts.len() == expected.accounts.len() + 1
            && fee_payer_key
                .as_deref()
                .is_some_and(|payer| !expected_keys.contains(payer))
        {
            return Err(CoordinatorError::InvalidSignedTransaction(
                "unexpected fee payer account was added to the adjustment message".into(),
            ));
        }
        if static_accounts.len() == expected.accounts.len() + 2
            && fee_payer_key
                .as_deref()
                .is_some_and(|payer| expected_keys.contains(payer))
        {
            return Err(CoordinatorError::InvalidSignedTransaction(
                "fee payer must not be an unexpected duplicate adjustment account".into(),
            ));
        }
        let program_occurrences = static_accounts
            .iter()
            .filter(|account| account.to_string() == expected.program_id)
            .count();
        if program_occurrences != 1 {
            return Err(CoordinatorError::InvalidSignedTransaction(
                "adjustment program id must occur exactly once in static accounts".into(),
            ));
        }
        for (index, account) in static_accounts.iter().enumerate() {
            let key = account.to_string();
            if !expected_keys.contains(key.as_str()) && key != expected.program_id && index != 0 {
                return Err(CoordinatorError::InvalidSignedTransaction(
                    "signed adjustment contains an unused unexpected static account".into(),
                ));
            }
        }
        let mut unique_accounts = std::collections::BTreeSet::new();
        for (position, (actual_index, expected_account)) in instruction
            .accounts
            .iter()
            .zip(&expected.accounts)
            .enumerate()
        {
            let actual_index = usize::from(*actual_index);
            let actual_key = static_accounts.get(actual_index).ok_or_else(|| {
                CoordinatorError::InvalidSignedTransaction(format!(
                    "signed adjustment account index is out of bounds at position {position}"
                ))
            })?;
            let actual_is_signer =
                actual_index < static_accounts.len() && message.is_signer(actual_index);
            let reserved_addresses =
                std::collections::BTreeSet::<solana_transaction::Address>::new();
            let actual_is_writable = message
                .is_maybe_writable_with_reserved_addresses(actual_index, Some(&reserved_addresses));
            if actual_key.to_string() != expected_account.pubkey
                || actual_is_signer != expected_account.is_signer
                || actual_is_writable != expected_account.is_writable
            {
                return Err(CoordinatorError::InvalidSignedTransaction(format!(
                    "signed adjustment account policy does not match the compiled instruction at position {position}"
                )));
            }
            if !unique_accounts.insert(actual_key) {
                return Err(CoordinatorError::InvalidSignedTransaction(
                    "signed adjustment contains duplicate instruction accounts".into(),
                ));
            }
        }
        Ok(())
    }

    /// Verifies a wallet-signed refund claim or execute instruction against
    /// the server-owned compiler artifact. Refund movement does not permit
    /// address lookup tables, compute-budget additions, extra instructions,
    /// account substitution, or client-selected signer topology.
    pub fn verify_refund_instruction(
        &self,
        expected: &CompiledSettlementRefundInstruction,
    ) -> Result<(), CoordinatorError> {
        if expected.status != "prepared_unsubmitted" {
            return Err(CoordinatorError::InvalidSignedTransaction(
                "refund instruction is not prepared for submission".into(),
            ));
        }
        let transaction = self.decode_verified_transaction()?;
        if self.bytes.len() > MAX_SIGNED_TRANSACTION_BYTES {
            return Err(CoordinatorError::InvalidSignedTransaction(
                "signed refund exceeds the bounded packet size".into(),
            ));
        }
        let message = &transaction.message;
        if !lookup_table_keys(message)?.is_empty() {
            return Err(CoordinatorError::InvalidSignedTransaction(
                "refund movement does not permit address lookup tables".into(),
            ));
        }
        let instructions = message.instructions();
        if instructions.len() != 1 {
            return Err(CoordinatorError::InvalidSignedTransaction(
                "refund transaction must contain exactly one instruction".into(),
            ));
        }
        let static_accounts = message.static_account_keys();
        let instruction = &instructions[0];
        let program_index = usize::from(instruction.program_id_index);
        let program_id = static_accounts.get(program_index).ok_or_else(|| {
            CoordinatorError::InvalidSignedTransaction(
                "refund program index is out of bounds".into(),
            )
        })?;
        if program_id.to_string() != expected.program_id {
            return Err(CoordinatorError::InvalidSignedTransaction(
                "signed refund program id does not match the compiled instruction".into(),
            ));
        }
        let expected_data = BASE64
            .decode(&expected.instruction_data_base64)
            .map_err(|_| {
                CoordinatorError::InvalidSignedTransaction(
                    "compiled refund instruction data is not valid base64".into(),
                )
            })?;
        if instruction.data != expected_data {
            return Err(CoordinatorError::InvalidSignedTransaction(
                "signed refund instruction data does not match the compiled instruction".into(),
            ));
        }
        if instruction.accounts.len() != expected.accounts.len()
            || (static_accounts.len() != expected.accounts.len() + 1
                && static_accounts.len() != expected.accounts.len() + 2)
        {
            return Err(CoordinatorError::InvalidSignedTransaction(
                "signed refund account shape does not match the compiled instruction".into(),
            ));
        }
        let expected_keys = expected
            .accounts
            .iter()
            .map(|account| account.pubkey.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        if expected_keys.len() != expected.accounts.len() {
            return Err(CoordinatorError::InvalidSignedTransaction(
                "compiled refund contains duplicate account keys".into(),
            ));
        }
        if static_accounts.len() == expected.accounts.len() + 2
            && expected_keys.contains(static_accounts[0].to_string().as_str())
        {
            return Err(CoordinatorError::InvalidSignedTransaction(
                "refund fee payer must not duplicate a movement account".into(),
            ));
        }
        if static_accounts
            .iter()
            .filter(|account| account.to_string() == expected.program_id)
            .count()
            != 1
        {
            return Err(CoordinatorError::InvalidSignedTransaction(
                "refund program id must occur exactly once in static accounts".into(),
            ));
        }
        for (index, account) in static_accounts.iter().enumerate() {
            let key = account.to_string();
            if !expected_keys.contains(key.as_str()) && key != expected.program_id && index != 0 {
                return Err(CoordinatorError::InvalidSignedTransaction(
                    "signed refund contains an unused unexpected static account".into(),
                ));
            }
        }
        let reserved_addresses = std::collections::BTreeSet::<solana_transaction::Address>::new();
        let mut unique_accounts = std::collections::BTreeSet::new();
        for (position, (actual_index, expected_account)) in instruction
            .accounts
            .iter()
            .zip(&expected.accounts)
            .enumerate()
        {
            let actual_index = usize::from(*actual_index);
            let actual_key = static_accounts.get(actual_index).ok_or_else(|| {
                CoordinatorError::InvalidSignedTransaction(format!(
                    "signed refund account index is out of bounds at position {position}"
                ))
            })?;
            let actual_is_signer = message.is_signer(actual_index);
            let actual_is_writable = message
                .is_maybe_writable_with_reserved_addresses(actual_index, Some(&reserved_addresses));
            if actual_key.to_string() != expected_account.pubkey
                || actual_is_signer != expected_account.is_signer
                || actual_is_writable != expected_account.is_writable
            {
                return Err(CoordinatorError::InvalidSignedTransaction(format!(
                    "signed refund account policy does not match the compiled instruction at position {position}"
                )));
            }
            if !unique_accounts.insert(actual_key) {
                return Err(CoordinatorError::InvalidSignedTransaction(
                    "signed refund contains duplicate instruction accounts".into(),
                ));
            }
        }
        Ok(())
    }

    /// Verifies the fixed five-account governance cancellation instruction
    /// against the server-owned artifact. Cancellation has no movement
    /// accounts, permits no address lookup table, extra instruction, or
    /// client-selected account topology, and remains bounded by the legacy
    /// packet limit.
    pub fn verify_refund_cancellation_instruction(
        &self,
        expected: &CompiledSettlementRefundCancellationInstruction,
    ) -> Result<(), CoordinatorError> {
        if expected.status != "prepared_unsubmitted" {
            return Err(CoordinatorError::InvalidSignedTransaction(
                "refund cancellation is not prepared for submission".into(),
            ));
        }
        let transaction = self.decode_verified_transaction()?;
        if self.bytes.len() > MAX_SIGNED_TRANSACTION_BYTES {
            return Err(CoordinatorError::InvalidSignedTransaction(
                "signed refund cancellation exceeds the bounded packet size".into(),
            ));
        }
        let message = &transaction.message;
        if !lookup_table_keys(message)?.is_empty() {
            return Err(CoordinatorError::InvalidSignedTransaction(
                "refund cancellation does not permit address lookup tables".into(),
            ));
        }
        let instructions = message.instructions();
        if instructions.len() != 1 {
            return Err(CoordinatorError::InvalidSignedTransaction(
                "refund cancellation must contain exactly one instruction".into(),
            ));
        }
        let static_accounts = message.static_account_keys();
        let instruction = &instructions[0];
        let program_index = usize::from(instruction.program_id_index);
        let program_id = static_accounts.get(program_index).ok_or_else(|| {
            CoordinatorError::InvalidSignedTransaction(
                "refund cancellation program index is out of bounds".into(),
            )
        })?;
        if program_id.to_string() != expected.program_id {
            return Err(CoordinatorError::InvalidSignedTransaction(
                "signed refund cancellation program id does not match the compiled instruction"
                    .into(),
            ));
        }
        let expected_data = BASE64
            .decode(&expected.instruction_data_base64)
            .map_err(|_| {
                CoordinatorError::InvalidSignedTransaction(
                    "compiled refund cancellation data is not valid base64".into(),
                )
            })?;
        if instruction.data != expected_data {
            return Err(CoordinatorError::InvalidSignedTransaction(
                "signed refund cancellation data does not match the compiled instruction".into(),
            ));
        }
        if instruction.accounts.len() != expected.accounts.len()
            || (static_accounts.len() != expected.accounts.len() + 1
                && static_accounts.len() != expected.accounts.len() + 2)
        {
            return Err(CoordinatorError::InvalidSignedTransaction(
                "signed refund cancellation account shape does not match the compiled instruction"
                    .into(),
            ));
        }
        let expected_keys = expected
            .accounts
            .iter()
            .map(|account| account.pubkey.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        if expected_keys.len() != expected.accounts.len() {
            return Err(CoordinatorError::InvalidSignedTransaction(
                "compiled refund cancellation contains duplicate account keys".into(),
            ));
        }
        if static_accounts.len() == expected.accounts.len() + 2
            && expected_keys.contains(static_accounts[0].to_string().as_str())
        {
            return Err(CoordinatorError::InvalidSignedTransaction(
                "refund cancellation fee payer must not duplicate a movement account".into(),
            ));
        }
        if static_accounts
            .iter()
            .filter(|account| account.to_string() == expected.program_id)
            .count()
            != 1
        {
            return Err(CoordinatorError::InvalidSignedTransaction(
                "refund cancellation program id must occur exactly once in static accounts".into(),
            ));
        }
        for (index, account) in static_accounts.iter().enumerate() {
            let key = account.to_string();
            if !expected_keys.contains(key.as_str()) && key != expected.program_id && index != 0 {
                return Err(CoordinatorError::InvalidSignedTransaction(
                    "signed refund cancellation contains an unused unexpected static account"
                        .into(),
                ));
            }
        }
        let reserved_addresses = std::collections::BTreeSet::<solana_transaction::Address>::new();
        let mut unique_accounts = std::collections::BTreeSet::new();
        for (position, (actual_index, expected_account)) in instruction
            .accounts
            .iter()
            .zip(&expected.accounts)
            .enumerate()
        {
            let actual_index = usize::from(*actual_index);
            let actual_key = static_accounts.get(actual_index).ok_or_else(|| {
                CoordinatorError::InvalidSignedTransaction(format!(
                    "signed refund cancellation account index is out of bounds at position {position}"
                ))
            })?;
            let actual_is_signer = message.is_signer(actual_index);
            let actual_is_writable = message
                .is_maybe_writable_with_reserved_addresses(actual_index, Some(&reserved_addresses));
            if actual_key.to_string() != expected_account.pubkey
                || actual_is_signer != expected_account.is_signer
                || actual_is_writable != expected_account.is_writable
            {
                return Err(CoordinatorError::InvalidSignedTransaction(format!(
                    "signed refund cancellation account policy does not match the compiled instruction at position {position}"
                )));
            }
            if !unique_accounts.insert(actual_key) {
                return Err(CoordinatorError::InvalidSignedTransaction(
                    "signed refund cancellation contains duplicate instruction accounts".into(),
                ));
            }
        }
        Ok(())
    }

    /// Proves that a wallet signed exactly one server-compiled revenue-stream
    /// instruction. Revenue-stream movement remains deliberately bounded: no
    /// lookup tables, compute-budget additions, extra instructions, account
    /// substitution, or client-selected signer topology are accepted here.
    pub fn verify_revenue_stream_instruction(
        &self,
        expected: &CompiledRevenueStreamInstruction,
    ) -> Result<(), CoordinatorError> {
        if expected.status != "prepared_unsubmitted" {
            return Err(CoordinatorError::InvalidSignedTransaction(
                "revenue stream instruction is not prepared for submission".into(),
            ));
        }
        let transaction = self.decode_verified_transaction()?;
        if self.bytes.len() > MAX_SIGNED_TRANSACTION_BYTES {
            return Err(CoordinatorError::InvalidSignedTransaction(
                "signed revenue stream instruction exceeds the bounded packet size".into(),
            ));
        }
        let message = &transaction.message;
        if !lookup_table_keys(message)?.is_empty() {
            return Err(CoordinatorError::InvalidSignedTransaction(
                "revenue stream instructions do not permit address lookup tables".into(),
            ));
        }
        let instructions = message.instructions();
        if instructions.len() != 1 {
            return Err(CoordinatorError::InvalidSignedTransaction(
                "revenue stream transaction must contain exactly one instruction".into(),
            ));
        }
        let static_accounts = message.static_account_keys();
        let instruction = &instructions[0];
        let program_index = usize::from(instruction.program_id_index);
        let program_id = static_accounts.get(program_index).ok_or_else(|| {
            CoordinatorError::InvalidSignedTransaction(
                "revenue stream program index is out of bounds".into(),
            )
        })?;
        if program_id.to_string() != expected.program_id {
            return Err(CoordinatorError::InvalidSignedTransaction(
                "signed revenue stream program id does not match the compiled instruction".into(),
            ));
        }
        let expected_data = BASE64
            .decode(&expected.instruction_data_base64)
            .map_err(|_| {
                CoordinatorError::InvalidSignedTransaction(
                    "compiled revenue stream instruction data is not valid base64".into(),
                )
            })?;
        if instruction.data != expected_data {
            return Err(CoordinatorError::InvalidSignedTransaction(
                "signed revenue stream instruction data does not match the compiled instruction"
                    .into(),
            ));
        }
        if instruction.accounts.len() != expected.accounts.len() {
            return Err(CoordinatorError::InvalidSignedTransaction(
                "signed revenue stream account count does not match the compiled instruction"
                    .into(),
            ));
        }
        if static_accounts.len() != expected.accounts.len() + 1
            && static_accounts.len() != expected.accounts.len() + 2
        {
            return Err(CoordinatorError::InvalidSignedTransaction(
                "signed revenue stream contains unexpected static account keys".into(),
            ));
        }
        let expected_keys = expected
            .accounts
            .iter()
            .map(|account| account.pubkey.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        if expected_keys.len() != expected.accounts.len() {
            return Err(CoordinatorError::InvalidSignedTransaction(
                "compiled revenue stream contains duplicate account keys".into(),
            ));
        }
        let fee_payer_key = static_accounts.first().map(ToString::to_string);
        if static_accounts.len() == expected.accounts.len() + 2
            && fee_payer_key
                .as_deref()
                .is_some_and(|payer| expected_keys.contains(payer))
        {
            return Err(CoordinatorError::InvalidSignedTransaction(
                "revenue stream fee payer must not duplicate an expected account".into(),
            ));
        }
        if static_accounts
            .iter()
            .filter(|account| account.to_string() == expected.program_id)
            .count()
            != 1
        {
            return Err(CoordinatorError::InvalidSignedTransaction(
                "revenue stream program id must occur exactly once in static accounts".into(),
            ));
        }
        for (index, account) in static_accounts.iter().enumerate() {
            let key = account.to_string();
            if !expected_keys.contains(key.as_str()) && key != expected.program_id && index != 0 {
                return Err(CoordinatorError::InvalidSignedTransaction(
                    "signed revenue stream contains an unused unexpected static account".into(),
                ));
            }
        }
        let reserved_addresses = std::collections::BTreeSet::<solana_transaction::Address>::new();
        let mut unique_accounts = std::collections::BTreeSet::new();
        for (position, (actual_index, expected_account)) in instruction
            .accounts
            .iter()
            .zip(&expected.accounts)
            .enumerate()
        {
            let actual_index = usize::from(*actual_index);
            let actual_key = static_accounts.get(actual_index).ok_or_else(|| {
                CoordinatorError::InvalidSignedTransaction(format!(
                    "signed revenue stream account index is out of bounds at position {position}"
                ))
            })?;
            let actual_is_signer = message.is_signer(actual_index);
            let actual_is_writable = message
                .is_maybe_writable_with_reserved_addresses(actual_index, Some(&reserved_addresses));
            let fee_payer_writable_elevation = actual_index == 0
                && actual_is_signer
                && expected_account.is_signer
                && actual_is_writable
                && !expected_account.is_writable;
            if actual_key.to_string() != expected_account.pubkey
                || actual_is_signer != expected_account.is_signer
                || (actual_is_writable != expected_account.is_writable
                    && !fee_payer_writable_elevation)
            {
                return Err(CoordinatorError::InvalidSignedTransaction(format!(
                    "signed revenue stream account policy does not match the compiled instruction at position {position}"
                )));
            }
            if !unique_accounts.insert(actual_key) {
                return Err(CoordinatorError::InvalidSignedTransaction(
                    "signed revenue stream contains duplicate instruction accounts".into(),
                ));
            }
        }
        Ok(())
    }

    /// Verifies a settlement against lookup-table contents obtained from a
    /// server-owned source. This method never trusts client-provided table
    /// digests or account lists; the caller must have resolved the exact table
    /// keys present in the signed v0 message.
    pub fn verify_settlement_instruction_with_lookup_tables(
        &self,
        expected: &CompiledSettlementInstruction,
        lookup_tables: &[ResolvedAddressLookupTable],
    ) -> Result<(), CoordinatorError> {
        if expected.status != "prepared_unsubmitted" {
            return Err(CoordinatorError::InvalidSignedTransaction(
                "settlement instruction is not prepared for submission".into(),
            ));
        }
        let transaction = self.decode_verified_transaction()?;

        let message = &transaction.message;
        if self.bytes.len() > MAX_SETTLEMENT_SIGNED_BYTES {
            return Err(CoordinatorError::InvalidSignedTransaction(
                "signed settlement exceeds the conservative wire budget".into(),
            ));
        }
        let (loaded_writable, loaded_readonly) = resolve_loaded_addresses(message, lookup_tables)?;
        let instructions = message.instructions();
        if instructions.len() != 3 {
            return Err(CoordinatorError::InvalidSignedTransaction(
                "settlement transaction must contain compute budget pair followed by exactly one settlement instruction".into(),
            ));
        }
        let static_accounts = message.static_account_keys();
        let mut accounts = static_accounts.to_vec();
        accounts.extend_from_slice(&loaded_writable);
        accounts.extend_from_slice(&loaded_readonly);
        if accounts.len() != expected.accounts.len() + 2 {
            return Err(CoordinatorError::InvalidSignedTransaction(format!(
                "signed settlement contains unexpected account keys: expected {}, received {}",
                expected.accounts.len() + 2,
                accounts.len()
            )));
        }
        let mut unique_accounts = std::collections::BTreeSet::new();
        if accounts
            .iter()
            .any(|account| !unique_accounts.insert(account))
        {
            return Err(CoordinatorError::InvalidSignedTransaction(
                "signed settlement contains duplicate expanded account keys".into(),
            ));
        }
        let compute_budget_program = COMPUTE_BUDGET_PROGRAM_ID.parse().map_err(|_| {
            CoordinatorError::InvalidSignedTransaction(
                "configured compute-budget program id is invalid".into(),
            )
        })?;
        verify_compute_budget_instruction(
            &instructions[0],
            static_accounts,
            &accounts,
            &compute_budget_program,
            &expected.compute_budget_policy,
            &expected.compute_budget_policy.set_compute_unit_limit_data(),
        )?;
        verify_compute_budget_instruction(
            &instructions[1],
            static_accounts,
            &accounts,
            &compute_budget_program,
            &expected.compute_budget_policy,
            &expected.compute_budget_policy.set_compute_unit_price_data(),
        )?;
        let instruction = &instructions[2];
        let program_id = accounts
            .get(usize::from(instruction.program_id_index))
            .ok_or_else(|| {
                CoordinatorError::InvalidSignedTransaction(
                    "settlement instruction program index is out of bounds".into(),
                )
            })?;
        if usize::from(instruction.program_id_index) >= static_accounts.len() {
            return Err(CoordinatorError::InvalidSignedTransaction(
                "settlement program id cannot be loaded from an address table".into(),
            ));
        }
        if program_id.to_string() != expected.program_id {
            return Err(CoordinatorError::InvalidSignedTransaction(
                "signed instruction program id does not match compiled settlement".into(),
            ));
        }
        let expected_data = BASE64
            .decode(&expected.instruction_data_base64)
            .map_err(|_| {
                CoordinatorError::InvalidSignedTransaction(
                    "compiled settlement instruction data is not valid base64".into(),
                )
            })?;
        if instruction.data != expected_data {
            return Err(CoordinatorError::InvalidSignedTransaction(
                "signed instruction data does not match compiled settlement".into(),
            ));
        }
        if instruction.accounts.len() != expected.accounts.len() {
            return Err(CoordinatorError::InvalidSignedTransaction(
                "signed instruction account count does not match compiled settlement".into(),
            ));
        }
        expected.compute_budget_policy.validate().map_err(|_| {
            CoordinatorError::InvalidSignedTransaction(
                "compiled compute-budget policy is invalid".into(),
            )
        })?;
        estimate_legacy_settlement_wire_bytes_with_compute_budget(
            expected.accounts.len(),
            expected_data.len(),
        )
        .map_err(|_| {
            CoordinatorError::InvalidSignedTransaction(
                "compiled settlement exceeds the conservative wire budget".into(),
            )
        })?;
        for (position, (actual_index, expected_account)) in instruction
            .accounts
            .iter()
            .zip(&expected.accounts)
            .enumerate()
        {
            let actual_index = usize::from(*actual_index);
            let actual_key = accounts.get(actual_index).ok_or_else(|| {
                CoordinatorError::InvalidSignedTransaction(format!(
                    "signed instruction account index is out of bounds at position {position}"
                ))
            })?;
            let actual_is_signer =
                actual_index < static_accounts.len() && message.is_signer(actual_index);
            let actual_is_writable = if actual_index < static_accounts.len() {
                let reserved_addresses =
                    std::collections::BTreeSet::<solana_transaction::Address>::new();
                message.is_maybe_writable_with_reserved_addresses(
                    actual_index,
                    Some(&reserved_addresses),
                )
            } else {
                actual_index < static_accounts.len() + loaded_writable.len()
            };
            if actual_key.to_string() != expected_account.pubkey
                || actual_is_signer != expected_account.is_signer
                || actual_is_writable != expected_account.is_writable
            {
                return Err(CoordinatorError::InvalidSignedTransaction(format!(
                    "signed instruction account policy does not match compiled settlement at position {position}"
                )));
            }
        }
        Ok(())
    }

    /// Resolves all v0 lookup tables through a server-owned resolver and then
    /// applies the same exact expanded-domain checks as the synchronous path.
    pub async fn verify_settlement_instruction_with_resolver(
        &self,
        expected: &CompiledSettlementInstruction,
        resolver: &dyn AddressLookupTableResolver,
    ) -> Result<(), CoordinatorError> {
        let transaction = self.decode_verified_transaction()?;
        let lookup_keys = lookup_table_keys(&transaction.message)?;
        let lookup_tables = match lookup_keys.as_slice() {
            [] => Vec::new(),
            [first] => vec![resolver.resolve(first).await?],
            [first, second] => {
                let (first_result, second_result) =
                    tokio::join!(resolver.resolve(first), resolver.resolve(second));
                vec![first_result?, second_result?]
            }
            _ => {
                return Err(CoordinatorError::InvalidSignedTransaction(
                    "signed settlement contains too many address table lookups".into(),
                ));
            }
        };
        self.verify_settlement_instruction_with_lookup_tables(expected, &lookup_tables)
    }

    fn decode_verified_transaction(
        &self,
    ) -> Result<solana_transaction::versioned::VersionedTransaction, CoordinatorError> {
        let transaction = wincode::deserialize_exact::<
            solana_transaction::versioned::VersionedTransaction,
        >(self.bytes())
        .map_err(|error| {
            CoordinatorError::InvalidSignedTransaction(format!(
                "unable to decode versioned transaction: {error}"
            ))
        })?;
        transaction.verify_and_hash_message().map_err(|error| {
            CoordinatorError::InvalidSignedTransaction(format!(
                "transaction signatures or message are invalid: {error}"
            ))
        })?;
        Ok(transaction)
    }

    /// Returns the signed message's recent blockhash after wire decoding and
    /// structural sanitization. Signature verification is intentionally owned
    /// by `verify_settlement_instruction`; this helper is used only after that
    /// gate to apply the provider-backed freshness policy.
    fn recent_blockhash(&self) -> Result<String, CoordinatorError> {
        let transaction = wincode::deserialize_exact::<
            solana_transaction::versioned::VersionedTransaction,
        >(self.bytes())
        .map_err(|error| {
            CoordinatorError::InvalidSignedTransaction(format!(
                "unable to decode versioned transaction: {error}"
            ))
        })?;
        transaction.sanitize().map_err(|error| {
            CoordinatorError::InvalidSignedTransaction(format!(
                "signed transaction message is invalid: {error}"
            ))
        })?;
        Ok(transaction.message.recent_blockhash().to_string())
    }
}

fn verify_compute_budget_instruction(
    instruction: &solana_message::compiled_instruction::CompiledInstruction,
    static_accounts: &[solana_transaction::Address],
    accounts: &[solana_transaction::Address],
    expected_program: &solana_transaction::Address,
    policy: &ComputeBudgetPolicy,
    expected_data: &[u8],
) -> Result<(), CoordinatorError> {
    policy.validate().map_err(|_| {
        CoordinatorError::InvalidSignedTransaction(
            "compiled compute-budget policy is invalid".into(),
        )
    })?;
    let program_index = usize::from(instruction.program_id_index);
    if program_index >= static_accounts.len()
        || accounts.get(program_index) != Some(expected_program)
        || !instruction.accounts.is_empty()
        || instruction.data != expected_data
    {
        return Err(CoordinatorError::InvalidSignedTransaction(
            "signed compute-budget instruction is not the exact server policy".into(),
        ));
    }
    Ok(())
}

fn lookup_table_keys(
    message: &solana_transaction::VersionedMessage,
) -> Result<Vec<String>, CoordinatorError> {
    match message {
        solana_transaction::VersionedMessage::Legacy(_) => Ok(Vec::new()),
        solana_transaction::VersionedMessage::V0(message) => {
            if message.address_table_lookups.len() > MAX_ADDRESS_LOOKUP_TABLES {
                return Err(CoordinatorError::InvalidSignedTransaction(
                    "signed settlement contains too many address table lookups".into(),
                ));
            }
            let mut keys = Vec::with_capacity(message.address_table_lookups.len());
            for lookup in &message.address_table_lookups {
                let key = lookup.account_key.to_string();
                if keys.iter().any(|existing| existing == &key) {
                    return Err(CoordinatorError::InvalidSignedTransaction(
                        "signed settlement repeats an address lookup table".into(),
                    ));
                }
                keys.push(key);
            }
            Ok(keys)
        }
        solana_transaction::VersionedMessage::V1(_) => {
            Err(CoordinatorError::InvalidSignedTransaction(
                "settlement message version v1 is not governed".into(),
            ))
        }
    }
}

fn resolve_loaded_addresses(
    message: &solana_transaction::VersionedMessage,
    lookup_tables: &[ResolvedAddressLookupTable],
) -> Result<
    (
        Vec<solana_transaction::Address>,
        Vec<solana_transaction::Address>,
    ),
    CoordinatorError,
> {
    let lookup_keys = lookup_table_keys(message)?;
    if lookup_keys.len() != lookup_tables.len() {
        return Err(CoordinatorError::InvalidSignedTransaction(
            "server-resolved address tables do not match the signed message".into(),
        ));
    }
    if lookup_keys.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }
    let lookups = if let solana_transaction::VersionedMessage::V0(message) = message {
        &message.address_table_lookups
    } else {
        return Err(CoordinatorError::InvalidSignedTransaction(
            "only v0 messages may contain address table lookups".into(),
        ));
    };
    let mut loaded_writable = Vec::new();
    let mut loaded_readonly = Vec::new();
    let mut seen_tables = std::collections::BTreeSet::new();
    for lookup in lookups {
        let lookup_key = lookup.account_key.to_string();
        let table = lookup_tables
            .iter()
            .find(|table| table.table == lookup_key)
            .ok_or_else(|| {
                CoordinatorError::AddressLookupTable(format!(
                    "missing server-resolved table {lookup_key}"
                ))
            })?;
        if !seen_tables.insert(lookup_key.clone()) {
            return Err(CoordinatorError::AddressLookupTable(
                "duplicate lookup table resolution".into(),
            ));
        }
        if table.addresses.len() > MAX_ADDRESS_LOOKUP_TABLE_ADDRESSES {
            return Err(CoordinatorError::AddressLookupTable(format!(
                "lookup table {lookup_key} exceeds the address bound"
            )));
        }
        if lookup
            .writable_indexes
            .len()
            .saturating_add(lookup.readonly_indexes.len())
            > MAX_LOADED_ADDRESSES_PER_TABLE
        {
            return Err(CoordinatorError::AddressLookupTable(format!(
                "lookup table {lookup_key} exceeds the loaded-address bound"
            )));
        }
        for index in &lookup.writable_indexes {
            let address = table.addresses.get(usize::from(*index)).ok_or_else(|| {
                CoordinatorError::AddressLookupTable(format!(
                    "lookup table {lookup_key} has an out-of-range writable index"
                ))
            })?;
            loaded_writable.push(address.parse().map_err(|_| {
                CoordinatorError::AddressLookupTable(format!(
                    "lookup table {lookup_key} contains an invalid address"
                ))
            })?);
        }
        for index in &lookup.readonly_indexes {
            let address = table.addresses.get(usize::from(*index)).ok_or_else(|| {
                CoordinatorError::AddressLookupTable(format!(
                    "lookup table {lookup_key} has an out-of-range readonly index"
                ))
            })?;
            loaded_readonly.push(address.parse().map_err(|_| {
                CoordinatorError::AddressLookupTable(format!(
                    "lookup table {lookup_key} contains an invalid address"
                ))
            })?);
        }
    }
    Ok((loaded_writable, loaded_readonly))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SettlementStatus {
    PreparedUnsubmitted,
    Signed,
    Submitting,
    Submitted,
    Processed,
    Confirmed,
    Finalized,
    Cancelled,
    Rejected,
    Unknown,
}

impl SettlementStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PreparedUnsubmitted => "prepared_unsubmitted",
            Self::Signed => "signed",
            Self::Submitting => "submitting",
            Self::Submitted => "submitted",
            Self::Processed => "processed",
            Self::Confirmed => "confirmed",
            Self::Finalized => "finalized",
            Self::Cancelled => "cancelled",
            Self::Rejected => "rejected",
            Self::Unknown => "unknown",
        }
    }
}

/// Returns whether a persisted lifecycle transition is monotonic and safe.
/// `unknown` is recoverable only through later chain evidence; it is never a
/// successful terminal state.
pub const fn can_transition(from: SettlementStatus, to: SettlementStatus) -> bool {
    matches!(
        (from, to),
        (
            SettlementStatus::PreparedUnsubmitted,
            SettlementStatus::Signed
        ) | (SettlementStatus::Signed, SettlementStatus::Submitting)
            | (SettlementStatus::Submitting, SettlementStatus::Submitted)
            | (SettlementStatus::Submitting, SettlementStatus::Unknown)
            | (SettlementStatus::Submitting, SettlementStatus::Processed)
            | (SettlementStatus::Submitting, SettlementStatus::Confirmed)
            | (SettlementStatus::Submitting, SettlementStatus::Finalized)
            | (SettlementStatus::Submitting, SettlementStatus::Rejected)
            | (SettlementStatus::Submitted, SettlementStatus::Processed)
            | (SettlementStatus::Submitted, SettlementStatus::Confirmed)
            | (SettlementStatus::Submitted, SettlementStatus::Finalized)
            | (SettlementStatus::Submitted, SettlementStatus::Rejected)
            | (SettlementStatus::Submitted, SettlementStatus::Unknown)
            | (SettlementStatus::Processed, SettlementStatus::Confirmed)
            | (SettlementStatus::Processed, SettlementStatus::Finalized)
            | (SettlementStatus::Processed, SettlementStatus::Rejected)
            | (SettlementStatus::Processed, SettlementStatus::Unknown)
            | (SettlementStatus::Confirmed, SettlementStatus::Finalized)
            | (SettlementStatus::Finalized, SettlementStatus::Cancelled)
            | (SettlementStatus::Confirmed, SettlementStatus::Unknown)
            | (SettlementStatus::Unknown, SettlementStatus::Submitted)
            | (SettlementStatus::Unknown, SettlementStatus::Processed)
            | (SettlementStatus::Unknown, SettlementStatus::Confirmed)
            | (SettlementStatus::Unknown, SettlementStatus::Finalized)
            | (SettlementStatus::Unknown, SettlementStatus::Rejected)
            | (SettlementStatus::Unknown, SettlementStatus::Unknown)
    )
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubmissionReceipt {
    pub signature: String,
    pub status: SettlementStatus,
    pub provider_index: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObservedStatus {
    pub signature: String,
    pub status: SettlementStatus,
    pub slot: Option<u64>,
    pub provider_index: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SettlementProviderHealthSnapshot {
    pub provider_index: usize,
    pub score: i32,
    pub consecutive_failures: u32,
    pub cooldown_remaining: Duration,
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum CoordinatorError {
    #[error("invalid signed transaction: {0}")]
    InvalidSignedTransaction(String),
    #[error("address lookup table resolution failed: {0}")]
    AddressLookupTable(String),
    #[error("invalid coordinator configuration: {0}")]
    InvalidConfiguration(String),
    #[error("provider attempts exhausted: {0}")]
    ProviderAttemptsExhausted(String),
    #[error("provider response is invalid: {0}")]
    InvalidProviderResponse(String),
    #[error("adjustment registry verification failed: {0}")]
    InvalidAdjustmentRegistry(String),
    #[error("signature is invalid")]
    InvalidSignature,
}

#[derive(Clone)]
pub struct SettlementCoordinator {
    providers: Vec<ProviderSlot>,
    attempt_timeout: Duration,
}

#[derive(Clone)]
struct ProviderSlot {
    transport: Arc<dyn JsonRpcTransport>,
    health: AdaptiveProviderHealth,
}

impl SettlementCoordinator {
    pub fn new(
        providers: Vec<Arc<dyn JsonRpcTransport>>,
        attempt_timeout: Duration,
    ) -> Result<Self, CoordinatorError> {
        if providers.is_empty() || providers.len() > MAX_PROVIDER_ATTEMPTS {
            return Err(CoordinatorError::InvalidConfiguration(
                "settlement provider pool must contain 1 to 3 endpoints".into(),
            ));
        }
        if !(Duration::from_millis(50)..=Duration::from_secs(10)).contains(&attempt_timeout) {
            return Err(CoordinatorError::InvalidConfiguration(
                "settlement attempt timeout must be between 50ms and 10s".into(),
            ));
        }
        Ok(Self {
            providers: providers
                .into_iter()
                .map(|transport| ProviderSlot {
                    transport,
                    health: AdaptiveProviderHealth::new(),
                })
                .collect(),
            attempt_timeout,
        })
    }

    fn provider_order(&self) -> Vec<usize> {
        let mut available = self
            .providers
            .iter()
            .enumerate()
            .filter(|(_, provider)| provider.health.is_available())
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        if available.is_empty()
            && let Some((index, _)) = self
                .providers
                .iter()
                .enumerate()
                .min_by_key(|(_, provider)| provider.health.cooldown_remaining())
        {
            available.push(index);
        }
        available.sort_by_key(|index| {
            (
                std::cmp::Reverse(self.providers[*index].health.score()),
                *index,
            )
        });
        available
    }

    pub fn health_snapshot(&self) -> Vec<SettlementProviderHealthSnapshot> {
        self.providers
            .iter()
            .enumerate()
            .map(
                |(provider_index, provider)| SettlementProviderHealthSnapshot {
                    provider_index,
                    score: provider.health.score(),
                    consecutive_failures: provider.health.consecutive_failures(),
                    cooldown_remaining: provider.health.cooldown_remaining(),
                },
            )
            .collect()
    }

    /// Verifies the server-derived adjustment registry at finalized
    /// commitment. Successful providers must return byte-identical account
    /// data; a transport failure may fail over, but conflicting valid state
    /// is never selected.
    pub async fn verify_adjustment_registry(
        &self,
        request: &AdjustmentRegistryVerificationRequest,
    ) -> Result<AdjustmentRegistryVerification, CoordinatorError> {
        let (registry, expected_bump) =
            settlement_client::adjustment_registry_pda(&request.merchant, &request.mint)
                .map_err(|error| CoordinatorError::InvalidAdjustmentRegistry(error.to_string()))?;
        let (expected, expected_governance) = parse_registry_keys(request)?;
        let mut observations = Vec::new();
        let mut last_error = None;

        for provider_index in self.provider_order() {
            let provider = &self.providers[provider_index];
            let result = tokio::time::timeout(
                self.attempt_timeout,
                provider.transport.call(
                    "getAccountInfo",
                    json!([
                        registry,
                        { "commitment": "finalized", "encoding": "base64" }
                    ]),
                ),
            )
            .await;
            match result {
                Ok(Ok(value)) => match parse_adjustment_registry_response(
                    &registry,
                    expected_bump,
                    &expected,
                    expected_governance.as_ref(),
                    &value,
                ) {
                    Ok(observation) => {
                        provider.health.record_success();
                        if let Some((_, first_data)) = observations.first()
                            && first_data != &observation.data
                        {
                            return Err(CoordinatorError::InvalidAdjustmentRegistry(
                                "configured finalized providers returned conflicting registry bytes"
                                    .into(),
                            ));
                        }
                        observations.push((observation.slot, observation.data));
                    }
                    Err(error) => {
                        provider.health.record_failure();
                        last_error = Some(error);
                    }
                },
                Ok(Err(error)) => {
                    provider.health.record_failure();
                    last_error = Some(CoordinatorError::from(error));
                }
                Err(_) => {
                    provider.health.record_failure();
                    last_error = Some(CoordinatorError::ProviderAttemptsExhausted(
                        "adjustment registry verification deadline exceeded".into(),
                    ));
                }
            }
        }

        let (finalized_slot, data) = observations
            .into_iter()
            .min_by_key(|(slot, _)| *slot)
            .ok_or_else(|| {
                last_error.unwrap_or_else(|| {
                    CoordinatorError::InvalidAdjustmentRegistry(
                        "no finalized adjustment registry evidence was returned".into(),
                    )
                })
            })?;
        let mut digest = Sha256::new();
        digest.update(b"solana-tax-protocol/adjustment-registry-account/v1");
        digest.update([0]);
        digest.update(registry.as_bytes());
        digest.update([0]);
        digest.update(&data);
        Ok(AdjustmentRegistryVerification {
            registry,
            merchant: request.merchant.clone(),
            mint: request.mint.clone(),
            proposer: request.proposer.clone(),
            approver: request.approver.clone(),
            governance: request.governance.clone(),
            finalized_slot,
            account_digest: digest.finalize().into(),
            commitment: "finalized",
        })
    }

    pub async fn submit(
        &self,
        transaction: &SignedTransaction,
    ) -> Result<SubmissionReceipt, CoordinatorError> {
        let expected_signature = transaction.signature()?;
        let mut last_error = None;
        for provider_index in self.provider_order() {
            let provider = &self.providers[provider_index];
            let request = json!([
                transaction.canonical_base64(),
                {
                    "encoding": "base64",
                    "skipPreflight": false,
                    "maxRetries": 0,
                    "preflightCommitment": "confirmed"
                }
            ]);
            let result = tokio::time::timeout(
                self.attempt_timeout,
                provider.transport.call("sendTransaction", request),
            )
            .await;
            match result {
                Ok(Ok(value)) => match parse_signature(&value) {
                    Ok(signature) => {
                        if signature != expected_signature {
                            provider.health.record_failure();
                            last_error = Some(CoordinatorError::InvalidProviderResponse(
                                "provider returned a signature that does not match the signed transaction".into(),
                            ));
                        } else {
                            provider.health.record_success();
                            return Ok(SubmissionReceipt {
                                signature,
                                status: SettlementStatus::Submitted,
                                provider_index,
                            });
                        }
                    }
                    Err(error) => {
                        provider.health.record_failure();
                        last_error = Some(error);
                    }
                },
                Ok(Err(error)) => {
                    provider.health.record_failure();
                    last_error = Some(CoordinatorError::from(error));
                }
                Err(_) => {
                    provider.health.record_failure();
                    last_error = Some(CoordinatorError::ProviderAttemptsExhausted(
                        "provider attempt deadline exceeded".into(),
                    ))
                }
            }
        }
        Err(last_error.unwrap_or_else(|| {
            CoordinatorError::ProviderAttemptsExhausted("no provider result".into())
        }))
    }

    /// Checks the signed transaction's recent blockhash against the bounded
    /// provider pool. A false response is definitive stale-message evidence;
    /// transport and malformed-provider failures may fail over, but are never
    /// treated as proof that the blockhash is valid.
    pub async fn validate_blockhash(
        &self,
        transaction: &SignedTransaction,
    ) -> Result<(), CoordinatorError> {
        let blockhash = transaction.recent_blockhash()?;
        let mut last_error = None;
        for provider_index in self.provider_order() {
            let provider = &self.providers[provider_index];
            let request = json!([blockhash, {"commitment": "confirmed"}]);
            let result = tokio::time::timeout(
                self.attempt_timeout,
                provider.transport.call("isBlockhashValid", request),
            )
            .await;
            match result {
                Ok(Ok(value)) => match value.get("value").and_then(Value::as_bool) {
                    Some(true) => {
                        provider.health.record_success();
                        return Ok(());
                    }
                    Some(false) => {
                        provider.health.record_success();
                        return Err(CoordinatorError::InvalidSignedTransaction(
                            "signed transaction blockhash is stale or not valid at confirmed commitment".into(),
                        ));
                    }
                    None => {
                        provider.health.record_failure();
                        last_error = Some(CoordinatorError::InvalidProviderResponse(format!(
                            "provider {provider_index} returned an invalid blockhash response"
                        )))
                    }
                },
                Ok(Err(error)) => {
                    provider.health.record_failure();
                    last_error = Some(CoordinatorError::from(error));
                }
                Err(_) => {
                    provider.health.record_failure();
                    last_error = Some(CoordinatorError::ProviderAttemptsExhausted(
                        "blockhash validation deadline exceeded".into(),
                    ))
                }
            }
        }
        Err(last_error.unwrap_or_else(|| {
            CoordinatorError::ProviderAttemptsExhausted("no provider result".into())
        }))
    }

    pub async fn observe(&self, signature: &str) -> Result<ObservedStatus, CoordinatorError> {
        validate_signature(signature)?;
        let mut last_error = None;
        for provider_index in self.provider_order() {
            let provider = &self.providers[provider_index];
            let request = json!([[signature], {"searchTransactionHistory": true}]);
            let result = tokio::time::timeout(
                self.attempt_timeout,
                provider.transport.call("getSignatureStatuses", request),
            )
            .await;
            match result {
                Ok(Ok(value)) => match parse_observed_status(&value, signature, provider_index) {
                    Ok(status) => {
                        provider.health.record_success();
                        return Ok(status);
                    }
                    Err(error) => {
                        provider.health.record_failure();
                        last_error = Some(error);
                    }
                },
                Ok(Err(error)) => {
                    provider.health.record_failure();
                    last_error = Some(CoordinatorError::from(error));
                }
                Err(_) => {
                    provider.health.record_failure();
                    last_error = Some(CoordinatorError::ProviderAttemptsExhausted(
                        "provider attempt deadline exceeded".into(),
                    ))
                }
            }
        }
        Err(last_error.unwrap_or_else(|| {
            CoordinatorError::ProviderAttemptsExhausted("no provider result".into())
        }))
    }
}

#[async_trait]
impl AddressLookupTableResolver for SettlementCoordinator {
    async fn resolve(&self, table: &str) -> Result<ResolvedAddressLookupTable, CoordinatorError> {
        table.parse::<solana_transaction::Address>().map_err(|_| {
            CoordinatorError::AddressLookupTable("lookup table address is invalid".into())
        })?;
        let mut last_error = None;
        for provider_index in self.provider_order() {
            let provider = &self.providers[provider_index];
            let request = json!([
                table,
                {
                    "commitment": "finalized",
                    "encoding": "base64"
                }
            ]);
            let result = tokio::time::timeout(
                self.attempt_timeout,
                provider.transport.call("getAccountInfo", request),
            )
            .await;
            match result {
                Ok(Ok(value)) => {
                    let resolved = parse_lookup_table_response(table, &value)?;
                    provider.health.record_success();
                    return Ok(resolved);
                }
                Ok(Err(error)) => {
                    provider.health.record_failure();
                    last_error = Some(CoordinatorError::from(error));
                }
                Err(_) => {
                    provider.health.record_failure();
                    last_error = Some(CoordinatorError::ProviderAttemptsExhausted(
                        "address lookup table resolution deadline exceeded".into(),
                    ));
                }
            }
        }
        Err(last_error.unwrap_or_else(|| {
            CoordinatorError::ProviderAttemptsExhausted(
                "no provider available for address lookup table resolution".into(),
            )
        }))
    }
}

#[derive(Clone, Debug)]
struct AdjustmentRegistryObservation {
    slot: u64,
    data: Vec<u8>,
}

fn parse_registry_keys(
    request: &AdjustmentRegistryVerificationRequest,
) -> Result<([Address; 4], Option<Address>), CoordinatorError> {
    let keys = [
        ("merchant", request.merchant.as_str()),
        ("mint", request.mint.as_str()),
        ("proposer", request.proposer.as_str()),
        ("approver", request.approver.as_str()),
    ]
    .map(|(field, value)| {
        value.parse::<Address>().map_err(|_| {
            CoordinatorError::InvalidAdjustmentRegistry(format!(
                "{field} is not a valid Solana public key"
            ))
        })
    });
    let keys = keys.into_iter().collect::<Result<Vec<_>, _>>()?;
    let keys: [Address; 4] = keys.try_into().map_err(|_| {
        CoordinatorError::InvalidAdjustmentRegistry("registry key cardinality changed".into())
    })?;
    if keys[2] == keys[3] {
        return Err(CoordinatorError::InvalidAdjustmentRegistry(
            "registry proposer and approver must be distinct".into(),
        ));
    }
    let governance = request
        .governance
        .as_deref()
        .map(|value| {
            value.parse::<Address>().map_err(|_| {
                CoordinatorError::InvalidAdjustmentRegistry(
                    "governance is not a valid Solana public key".into(),
                )
            })
        })
        .transpose()?
        .filter(|key| *key != Address::default());
    if request.governance.is_some() && governance.is_none() {
        return Err(CoordinatorError::InvalidAdjustmentRegistry(
            "governance must be a non-default public key".into(),
        ));
    }
    Ok((keys, governance))
}

fn parse_adjustment_registry_response(
    registry: &str,
    expected_bump: u8,
    expected: &[Address; 4],
    expected_governance: Option<&Address>,
    response: &Value,
) -> Result<AdjustmentRegistryObservation, CoordinatorError> {
    let encoded_response = serde_json::to_vec(response)
        .map_err(|error| CoordinatorError::InvalidAdjustmentRegistry(error.to_string()))?;
    if encoded_response.len() > MAX_ADJUSTMENT_REGISTRY_RPC_RESULT_BYTES {
        return Err(CoordinatorError::InvalidAdjustmentRegistry(
            "registry account response exceeds the bounded size".into(),
        ));
    }
    let slot = response
        .get("context")
        .and_then(|context| context.get("slot"))
        .and_then(Value::as_u64)
        .ok_or_else(|| {
            CoordinatorError::InvalidAdjustmentRegistry(
                "registry response context.slot is invalid".into(),
            )
        })?;
    let account = response
        .get("value")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            CoordinatorError::InvalidAdjustmentRegistry(format!(
                "derived registry {registry} is absent at finalized commitment"
            ))
        })?;
    if account.get("owner").and_then(Value::as_str)
        != Some(settlement_client::SETTLEMENT_PROGRAM_ID)
    {
        return Err(CoordinatorError::InvalidAdjustmentRegistry(
            "registry account is not owned by the settlement program".into(),
        ));
    }
    if account.get("executable").and_then(Value::as_bool) != Some(false) {
        return Err(CoordinatorError::InvalidAdjustmentRegistry(
            "registry account executable flag is invalid".into(),
        ));
    }
    let encoded_data = account
        .get("data")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            CoordinatorError::InvalidAdjustmentRegistry(
                "registry account data is not an encoded array".into(),
            )
        })?;
    if encoded_data.len() != 2 || encoded_data[1].as_str() != Some("base64") {
        return Err(CoordinatorError::InvalidAdjustmentRegistry(
            "registry account data is not base64".into(),
        ));
    }
    let data = BASE64
        .decode(encoded_data[0].as_str().ok_or_else(|| {
            CoordinatorError::InvalidAdjustmentRegistry(
                "registry account base64 payload is not a string".into(),
            )
        })?)
        .map_err(|_| {
            CoordinatorError::InvalidAdjustmentRegistry(
                "registry account base64 payload is invalid".into(),
            )
        })?;
    if data.len() != ADJUSTMENT_REGISTRY_ACCOUNT_DATA_LEN {
        return Err(CoordinatorError::InvalidAdjustmentRegistry(
            "registry account layout is unsupported".into(),
        ));
    }
    let mut discriminator = Sha256::new();
    discriminator.update(b"account:SettlementAdjustmentRegistry");
    if data[..8] != discriminator.finalize()[..8] {
        return Err(CoordinatorError::InvalidAdjustmentRegistry(
            "registry account discriminator is invalid".into(),
        ));
    }
    let read_address = |offset: usize| -> Address {
        let mut bytes = [0; 32];
        bytes.copy_from_slice(&data[offset..offset + 32]);
        Address::new_from_array(bytes)
    };
    let merchant = read_address(8);
    let mint = read_address(40);
    let governance = read_address(72);
    let proposer = read_address(104);
    let approver = read_address(136);
    if merchant != expected[0]
        || mint != expected[1]
        || proposer != expected[2]
        || approver != expected[3]
        || expected_governance.is_some_and(|expected| governance != *expected)
    {
        return Err(CoordinatorError::InvalidAdjustmentRegistry(
            "registry account authority or scope fields do not match".into(),
        ));
    }
    if data[168] != 1 {
        return Err(CoordinatorError::InvalidAdjustmentRegistry(
            "registry account is inactive".into(),
        ));
    }
    if data[169] != expected_bump {
        return Err(CoordinatorError::InvalidAdjustmentRegistry(
            "registry account bump does not match the derived PDA".into(),
        ));
    }
    Ok(AdjustmentRegistryObservation { slot, data })
}

fn parse_lookup_table_response(
    table: &str,
    value: &Value,
) -> Result<ResolvedAddressLookupTable, CoordinatorError> {
    let account = value
        .get("value")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            CoordinatorError::AddressLookupTable(format!(
                "lookup table {table} is absent at finalized commitment"
            ))
        })?;
    let owner = account
        .get("owner")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            CoordinatorError::AddressLookupTable(format!(
                "lookup table {table} response does not contain an owner"
            ))
        })?;
    if owner != address_lookup_program::ID.to_string() {
        return Err(CoordinatorError::AddressLookupTable(format!(
            "lookup table {table} is not owned by the address lookup table program"
        )));
    }
    let data = account
        .get("data")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            CoordinatorError::AddressLookupTable(format!(
                "lookup table {table} response does not contain encoded data"
            ))
        })?;
    if data.len() != 2 || data[1].as_str() != Some("base64") {
        return Err(CoordinatorError::AddressLookupTable(format!(
            "lookup table {table} data encoding is not base64"
        )));
    }
    let encoded = data[0].as_str().ok_or_else(|| {
        CoordinatorError::AddressLookupTable(format!(
            "lookup table {table} base64 payload is not a string"
        ))
    })?;
    let bytes = BASE64.decode(encoded).map_err(|error| {
        CoordinatorError::AddressLookupTable(format!(
            "lookup table {table} base64 payload is invalid: {error}"
        ))
    })?;
    let lookup_table = AddressLookupTable::deserialize(&bytes).map_err(|error| {
        CoordinatorError::AddressLookupTable(format!(
            "lookup table {table} account data is invalid: {error:?}"
        ))
    })?;
    if lookup_table.meta.deactivation_slot != u64::MAX {
        return Err(CoordinatorError::AddressLookupTable(format!(
            "lookup table {table} is deactivated"
        )));
    }
    if lookup_table.addresses.len() > MAX_ADDRESS_LOOKUP_TABLE_ADDRESSES {
        return Err(CoordinatorError::AddressLookupTable(format!(
            "lookup table {table} exceeds the address bound"
        )));
    }
    Ok(ResolvedAddressLookupTable {
        table: table.to_string(),
        addresses: lookup_table
            .addresses
            .iter()
            .map(ToString::to_string)
            .collect(),
    })
}

fn parse_signature(value: &Value) -> Result<String, CoordinatorError> {
    let signature = value.as_str().ok_or_else(|| {
        CoordinatorError::InvalidProviderResponse("signature is not a string".into())
    })?;
    validate_signature(signature)?;
    Ok(signature.to_string())
}

fn validate_signature(signature: &str) -> Result<(), CoordinatorError> {
    if signature.is_empty()
        || signature.len() > MAX_SIGNATURE_BYTES
        || !signature.bytes().all(|byte| {
            matches!(byte, b'1'..=b'9' | b'A'..=b'H' | b'J'..=b'N' | b'P'..=b'Z' | b'a'..=b'k' | b'm'..=b'z')
        })
    {
        return Err(CoordinatorError::InvalidSignature);
    }
    Ok(())
}

fn parse_observed_status(
    value: &Value,
    signature: &str,
    provider_index: usize,
) -> Result<ObservedStatus, CoordinatorError> {
    let status_value = value
        .get("value")
        .and_then(Value::as_array)
        .and_then(|values| values.first())
        .ok_or_else(|| CoordinatorError::InvalidProviderResponse("missing status value".into()))?;
    if status_value.is_null() {
        return Ok(ObservedStatus {
            signature: signature.to_string(),
            status: SettlementStatus::Unknown,
            slot: None,
            provider_index,
        });
    }
    let slot = status_value.get("slot").and_then(Value::as_u64);
    let status = if !status_value.get("err").is_none_or(Value::is_null) {
        SettlementStatus::Rejected
    } else {
        match status_value
            .get("confirmationStatus")
            .and_then(Value::as_str)
        {
            Some("finalized") => SettlementStatus::Finalized,
            Some("confirmed") => SettlementStatus::Confirmed,
            Some("processed") => SettlementStatus::Processed,
            Some(_) | None => SettlementStatus::Unknown,
        }
    };
    Ok(ObservedStatus {
        signature: signature.to_string(),
        status,
        slot,
        provider_index,
    })
}

impl From<ProviderError> for CoordinatorError {
    fn from(error: ProviderError) -> Self {
        Self::ProviderAttemptsExhausted(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use proptest::prelude::*;
    use serde_json::json;
    use settlement_client::{
        CompiledRevenueStreamInstruction, CompiledSettlementAdjustmentInstruction,
        CompiledSettlementInstruction, CompiledSettlementRefundCancellationInstruction,
        RevenueStreamClaimCancellationCompileRequest, SETTLEMENT_INSTRUCTION_ACCOUNT_COUNT,
        SettlementAdjustmentCompileRequest, SettlementCompileRequest,
        SettlementRefundCancellationCompileRequest, SettlementRefundClaimCompileRequest,
        compile_revenue_stream_claim_cancellation_instruction,
        compile_settlement_adjustment_instruction, compile_settlement_instruction,
        compile_settlement_refund_cancellation_instruction,
        compile_settlement_refund_claim_instruction,
    };
    use solana_keypair::Keypair;
    use solana_signer::Signer;
    use std::sync::Mutex;

    #[derive(Default)]
    struct FakeTransport {
        responses: Mutex<Vec<Result<Value, ProviderError>>>,
        requests: Mutex<Vec<(String, Value)>>,
    }

    #[async_trait]
    impl JsonRpcTransport for FakeTransport {
        async fn call(&self, method: &str, params: Value) -> Result<Value, ProviderError> {
            self.requests
                .lock()
                .expect("requests lock")
                .push((method.to_string(), params));
            self.responses.lock().expect("responses lock").remove(0)
        }
    }

    struct FakeLookupResolver {
        table: ResolvedAddressLookupTable,
        calls: Mutex<Vec<String>>,
    }

    #[async_trait]
    impl AddressLookupTableResolver for FakeLookupResolver {
        async fn resolve(
            &self,
            table: &str,
        ) -> Result<ResolvedAddressLookupTable, CoordinatorError> {
            self.calls
                .lock()
                .expect("lookup calls lock")
                .push(table.into());
            if table == self.table.table {
                Ok(self.table.clone())
            } else {
                Err(CoordinatorError::AddressLookupTable(
                    "unexpected lookup table".into(),
                ))
            }
        }
    }

    fn transaction() -> SignedTransaction {
        let payer = Keypair::new();
        let expected = compile_settlement_instruction(&compile_request(&payer)).expect("compile");
        signed_settlement(&expected, &payer)
    }

    fn compile_request(payer: &Keypair) -> SettlementCompileRequest {
        let unique = || solana_transaction::Address::new_unique().to_string();
        SettlementCompileRequest {
            program_id: settlement_client::SETTLEMENT_PROGRAM_ID.to_string(),
            payer: payer.pubkey().to_string(),
            merchant: unique(),
            source: unique(),
            mint: unique(),
            tax_vault: unique(),
            net_destination: unique(),
            token_program: settlement_client::SPL_TOKEN_PROGRAM_ID.to_string(),
            nonce: 7,
            policy_sequence: 3,
            gross_atomic: "1000".into(),
            tax_atomic: "100".into(),
            net_atomic: "900".into(),
            policy_digest: [1; 32],
        }
    }

    fn adjustment_registry_response(
        merchant: Address,
        mint: Address,
        proposer: Address,
        approver: Address,
        governance_byte: u8,
        slot: u64,
    ) -> Value {
        let (registry, bump) =
            settlement_client::adjustment_registry_pda(&merchant.to_string(), &mint.to_string())
                .expect("registry PDA");
        let mut data = vec![0; ADJUSTMENT_REGISTRY_ACCOUNT_DATA_LEN];
        let mut discriminator = Sha256::new();
        discriminator.update(b"account:SettlementAdjustmentRegistry");
        data[..8].copy_from_slice(&discriminator.finalize()[..8]);
        data[8..40].copy_from_slice(merchant.as_ref());
        data[40..72].copy_from_slice(mint.as_ref());
        data[72..104].fill(governance_byte);
        data[104..136].copy_from_slice(proposer.as_ref());
        data[136..168].copy_from_slice(approver.as_ref());
        data[168] = 1;
        data[169] = bump;
        json!({
            "context": { "slot": slot },
            "value": {
                "owner": settlement_client::SETTLEMENT_PROGRAM_ID,
                "executable": false,
                "data": [BASE64.encode(data), "base64"],
                "address": registry
            }
        })
    }

    fn signed_settlement(
        expected: &CompiledSettlementInstruction,
        payer: &Keypair,
    ) -> SignedTransaction {
        let program_id = expected.program_id.parse().expect("program id");
        let accounts = expected
            .accounts
            .iter()
            .map(|account| {
                let pubkey = account.pubkey.parse().expect("account pubkey");
                if account.is_writable {
                    solana_transaction::AccountMeta::new(pubkey, account.is_signer)
                } else {
                    solana_transaction::AccountMeta::new_readonly(pubkey, account.is_signer)
                }
            })
            .collect();
        let instruction = solana_transaction::Instruction {
            program_id,
            accounts,
            data: BASE64
                .decode(&expected.instruction_data_base64)
                .expect("instruction data"),
        };
        let compute_budget_program = COMPUTE_BUDGET_PROGRAM_ID.parse().expect("compute program");
        let policy = &expected.compute_budget_policy;
        let compute_budget_instructions = [
            solana_transaction::Instruction {
                program_id: compute_budget_program,
                accounts: Vec::new(),
                data: policy.set_compute_unit_limit_data(),
            },
            solana_transaction::Instruction {
                program_id: compute_budget_program,
                accounts: Vec::new(),
                data: policy.set_compute_unit_price_data(),
            },
        ];
        let transaction = solana_transaction::Transaction::new_signed_with_payer(
            &[
                compute_budget_instructions[0].clone(),
                compute_budget_instructions[1].clone(),
                instruction,
            ],
            Some(&payer.pubkey()),
            &[payer],
            solana_transaction::Hash::new_from_array([9; 32]),
        );
        SignedTransaction::parse(
            &BASE64.encode(
                wincode::serialize(&solana_transaction::versioned::VersionedTransaction::from(
                    transaction,
                ))
                .expect("serialize transaction"),
            ),
        )
        .expect("signed transaction")
    }

    fn compile_adjustment_request(approver: &Keypair) -> SettlementAdjustmentCompileRequest {
        let unique = || solana_transaction::Address::new_unique().to_string();
        SettlementAdjustmentCompileRequest {
            program_id: settlement_client::SETTLEMENT_PROGRAM_ID.to_string(),
            proposer: unique(),
            approver: approver.pubkey().to_string(),
            merchant: unique(),
            mint: unique(),
            token_program: settlement_client::SPL_TOKEN_PROGRAM_ID.to_string(),
            original_receipt: unique(),
            adjustment_nonce: 11,
            kind: 1,
            gross_sign: 1,
            gross_magnitude: "100".into(),
            tax_sign: 1,
            tax_magnitude: "25".into(),
            net_sign: 1,
            net_magnitude: "75".into(),
            adjustment_digest: [1; 32],
            evidence_digest: [2; 32],
            source_calculation_digest: [3; 32],
            replacement_calculation_digest: [0; 32],
        }
    }

    fn signed_adjustment(
        expected: &CompiledSettlementAdjustmentInstruction,
        payer: &Keypair,
    ) -> SignedTransaction {
        let instruction = solana_transaction::Instruction {
            program_id: expected.program_id.parse().expect("program id"),
            accounts: expected
                .accounts
                .iter()
                .map(|account| {
                    let pubkey = account.pubkey.parse().expect("account pubkey");
                    if account.is_writable {
                        solana_transaction::AccountMeta::new(pubkey, account.is_signer)
                    } else {
                        solana_transaction::AccountMeta::new_readonly(pubkey, account.is_signer)
                    }
                })
                .collect(),
            data: BASE64
                .decode(&expected.instruction_data_base64)
                .expect("instruction data"),
        };
        let transaction = solana_transaction::Transaction::new_signed_with_payer(
            &[instruction],
            Some(&payer.pubkey()),
            &[payer],
            solana_transaction::Hash::new_from_array([9; 32]),
        );
        SignedTransaction::parse(
            &BASE64.encode(
                wincode::serialize(&solana_transaction::versioned::VersionedTransaction::from(
                    transaction,
                ))
                .expect("serialize transaction"),
            ),
        )
        .expect("signed adjustment")
    }

    fn signed_with_instruction_sequence(
        payer: &Keypair,
        instructions: &[solana_transaction::Instruction],
    ) -> SignedTransaction {
        let transaction = solana_transaction::Transaction::new_signed_with_payer(
            instructions,
            Some(&payer.pubkey()),
            &[payer],
            solana_transaction::Hash::new_from_array([9; 32]),
        );
        SignedTransaction::parse(
            &BASE64.encode(
                wincode::serialize(&solana_transaction::versioned::VersionedTransaction::from(
                    transaction,
                ))
                .expect("serialize transaction"),
            ),
        )
        .expect("signed transaction")
    }

    fn signed_refund(
        expected: &settlement_client::CompiledSettlementRefundInstruction,
        proposer: &Keypair,
        refund_funder: &Keypair,
    ) -> SignedTransaction {
        assert_eq!(expected.accounts[0].pubkey, proposer.pubkey().to_string());
        assert_eq!(
            expected.accounts[1].pubkey,
            refund_funder.pubkey().to_string()
        );
        let program_id = expected.program_id.parse().expect("program id");
        let accounts = expected
            .accounts
            .iter()
            .map(|account| {
                let pubkey = account.pubkey.parse().expect("account pubkey");
                if account.is_writable {
                    solana_transaction::AccountMeta::new(pubkey, account.is_signer)
                } else {
                    solana_transaction::AccountMeta::new_readonly(pubkey, account.is_signer)
                }
            })
            .collect();
        let instruction = solana_transaction::Instruction {
            program_id,
            accounts,
            data: BASE64
                .decode(&expected.instruction_data_base64)
                .expect("instruction data"),
        };
        let blockhash = solana_transaction::Hash::new_from_array([9; 32]);
        let mut transaction = solana_transaction::Transaction::new_with_payer(
            &[instruction],
            Some(&proposer.pubkey()),
        );
        transaction
            .try_partial_sign(&[proposer], blockhash)
            .expect("sign proposer");
        transaction
            .try_partial_sign(&[refund_funder], blockhash)
            .expect("sign refund funder");
        SignedTransaction::parse(
            &BASE64.encode(
                wincode::serialize(&solana_transaction::versioned::VersionedTransaction::from(
                    transaction,
                ))
                .expect("serialize transaction"),
            ),
        )
        .expect("signed transaction")
    }

    fn signed_refund_cancellation(
        expected: &CompiledSettlementRefundCancellationInstruction,
        governance_authority: &Keypair,
    ) -> SignedTransaction {
        let instruction = solana_transaction::Instruction {
            program_id: expected.program_id.parse().expect("program id"),
            accounts: expected
                .accounts
                .iter()
                .map(|account| {
                    let pubkey = account.pubkey.parse().expect("account pubkey");
                    if account.is_writable {
                        solana_transaction::AccountMeta::new(pubkey, account.is_signer)
                    } else {
                        solana_transaction::AccountMeta::new_readonly(pubkey, account.is_signer)
                    }
                })
                .collect(),
            data: BASE64
                .decode(&expected.instruction_data_base64)
                .expect("instruction data"),
        };
        signed_with_instruction_sequence(governance_authority, &[instruction])
    }

    fn refund_claim_request(
        proposer: &Keypair,
        refund_funder: &Keypair,
    ) -> SettlementRefundClaimCompileRequest {
        let unique = || solana_transaction::Address::new_unique().to_string();
        SettlementRefundClaimCompileRequest {
            program_id: settlement_client::SETTLEMENT_PROGRAM_ID.to_string(),
            proposer: proposer.pubkey().to_string(),
            refund_funder: refund_funder.pubkey().to_string(),
            merchant: unique(),
            mint: unique(),
            token_program: settlement_client::SPL_TOKEN_PROGRAM_ID.to_string(),
            original_receipt: unique(),
            adjustment: unique(),
            tax_vault: unique(),
            net_source: unique(),
            refund_destination: unique(),
            claim_nonce: 7,
            execute_after_unix: 1_790_000_000,
            expires_at_unix: 1_790_003_600,
            reference_digest: [8; 32],
        }
    }

    fn refund_cancellation_request(
        governance_authority: &Keypair,
    ) -> SettlementRefundCancellationCompileRequest {
        let unique = || solana_transaction::Address::new_unique().to_string();
        SettlementRefundCancellationCompileRequest {
            program_id: settlement_client::SETTLEMENT_PROGRAM_ID.to_string(),
            governance_authority: governance_authority.pubkey().to_string(),
            merchant: unique(),
            mint: unique(),
            adjustment: unique(),
            claim_nonce: 7,
        }
    }

    fn settlement_instruction(
        expected: &CompiledSettlementInstruction,
    ) -> solana_transaction::Instruction {
        solana_transaction::Instruction {
            program_id: expected.program_id.parse().expect("program id"),
            accounts: expected
                .accounts
                .iter()
                .map(|account| {
                    let pubkey = account.pubkey.parse().expect("account pubkey");
                    if account.is_writable {
                        solana_transaction::AccountMeta::new(pubkey, account.is_signer)
                    } else {
                        solana_transaction::AccountMeta::new_readonly(pubkey, account.is_signer)
                    }
                })
                .collect(),
            data: BASE64
                .decode(&expected.instruction_data_base64)
                .expect("instruction data"),
        }
    }

    fn compute_budget_instructions(
        expected: &CompiledSettlementInstruction,
    ) -> [solana_transaction::Instruction; 2] {
        let program_id = COMPUTE_BUDGET_PROGRAM_ID.parse().expect("compute program");
        [
            solana_transaction::Instruction {
                program_id,
                accounts: Vec::new(),
                data: expected.compute_budget_policy.set_compute_unit_limit_data(),
            },
            solana_transaction::Instruction {
                program_id,
                accounts: Vec::new(),
                data: expected.compute_budget_policy.set_compute_unit_price_data(),
            },
        ]
    }

    fn signed_v0_settlement_with_lookup(
        expected: &CompiledSettlementInstruction,
        payer: &Keypair,
    ) -> (SignedTransaction, ResolvedAddressLookupTable) {
        let table_key = solana_transaction::Address::new_unique();
        let table_addresses = expected
            .accounts
            .iter()
            .map(|account| account.pubkey.parse().expect("account pubkey"))
            .collect::<Vec<solana_transaction::Address>>();
        let program_id = expected.program_id.parse().expect("program id");
        let instruction = solana_transaction::Instruction {
            program_id,
            accounts: expected
                .accounts
                .iter()
                .map(|account| {
                    let pubkey = account.pubkey.parse().expect("account pubkey");
                    if account.is_writable {
                        solana_transaction::AccountMeta::new(pubkey, account.is_signer)
                    } else {
                        solana_transaction::AccountMeta::new_readonly(pubkey, account.is_signer)
                    }
                })
                .collect(),
            data: BASE64
                .decode(&expected.instruction_data_base64)
                .expect("instruction data"),
        };
        let lookup_account = solana_message::AddressLookupTableAccount {
            key: table_key,
            addresses: table_addresses.clone(),
        };
        let compute_budget_program = COMPUTE_BUDGET_PROGRAM_ID.parse().expect("compute program");
        let policy = &expected.compute_budget_policy;
        let compute_budget_instructions = [
            solana_transaction::Instruction {
                program_id: compute_budget_program,
                accounts: Vec::new(),
                data: policy.set_compute_unit_limit_data(),
            },
            solana_transaction::Instruction {
                program_id: compute_budget_program,
                accounts: Vec::new(),
                data: policy.set_compute_unit_price_data(),
            },
        ];
        let message = solana_message::v0::Message::try_compile(
            &payer.pubkey(),
            &[
                compute_budget_instructions[0].clone(),
                compute_budget_instructions[1].clone(),
                instruction,
            ],
            &[lookup_account],
            solana_transaction::Hash::new_from_array([9; 32]),
        )
        .expect("compile v0 settlement");
        assert!(!message.address_table_lookups.is_empty());
        let transaction = solana_transaction::versioned::VersionedTransaction::try_new(
            solana_transaction::VersionedMessage::V0(message),
            &[payer],
        )
        .expect("sign v0 settlement");
        let signed = SignedTransaction::parse(
            &BASE64.encode(wincode::serialize(&transaction).expect("serialize v0 transaction")),
        )
        .expect("bounded v0 transaction");
        (
            signed,
            ResolvedAddressLookupTable {
                table: table_key.to_string(),
                addresses: table_addresses
                    .into_iter()
                    .map(|address| address.to_string())
                    .collect(),
            },
        )
    }

    fn signed_settlement_with_account_substitution(
        expected: &CompiledSettlementInstruction,
        payer: &Keypair,
        position: usize,
    ) -> SignedTransaction {
        let program_id = expected.program_id.parse().expect("program id");
        let replacement_keypair = Keypair::new();
        let replacement = replacement_keypair.pubkey();
        let accounts = expected
            .accounts
            .iter()
            .enumerate()
            .map(|(index, account)| {
                let pubkey = if index == position {
                    replacement
                } else {
                    account.pubkey.parse().expect("account pubkey")
                };
                if account.is_writable {
                    solana_transaction::AccountMeta::new(pubkey, account.is_signer)
                } else {
                    solana_transaction::AccountMeta::new_readonly(pubkey, account.is_signer)
                }
            })
            .collect();
        let instruction = solana_transaction::Instruction {
            program_id,
            accounts,
            data: BASE64
                .decode(&expected.instruction_data_base64)
                .expect("instruction data"),
        };
        let transaction = if expected.accounts[position].is_signer {
            solana_transaction::Transaction::new_signed_with_payer(
                &[instruction],
                Some(&payer.pubkey()),
                &[payer, &replacement_keypair],
                solana_transaction::Hash::new_from_array([9; 32]),
            )
        } else {
            solana_transaction::Transaction::new_signed_with_payer(
                &[instruction],
                Some(&payer.pubkey()),
                &[payer],
                solana_transaction::Hash::new_from_array([9; 32]),
            )
        };
        SignedTransaction::parse(
            &BASE64.encode(
                wincode::serialize(&solana_transaction::versioned::VersionedTransaction::from(
                    transaction,
                ))
                .expect("serialize transaction"),
            ),
        )
        .expect("signed transaction")
    }

    fn signed_settlement_with_writable_substitution(
        expected: &CompiledSettlementInstruction,
        payer: &Keypair,
        position: usize,
    ) -> SignedTransaction {
        let program_id = expected.program_id.parse().expect("program id");
        let accounts = expected
            .accounts
            .iter()
            .enumerate()
            .map(|(index, account)| {
                let pubkey = account.pubkey.parse().expect("account pubkey");
                let is_writable = if index == position {
                    !account.is_writable
                } else {
                    account.is_writable
                };
                if is_writable {
                    solana_transaction::AccountMeta::new(pubkey, account.is_signer)
                } else {
                    solana_transaction::AccountMeta::new_readonly(pubkey, account.is_signer)
                }
            })
            .collect();
        let instruction = solana_transaction::Instruction {
            program_id,
            accounts,
            data: BASE64
                .decode(&expected.instruction_data_base64)
                .expect("instruction data"),
        };
        let transaction = solana_transaction::Transaction::new_signed_with_payer(
            &[instruction],
            Some(&payer.pubkey()),
            &[payer],
            solana_transaction::Hash::new_from_array([9; 32]),
        );
        SignedTransaction::parse(
            &BASE64.encode(
                wincode::serialize(&solana_transaction::versioned::VersionedTransaction::from(
                    transaction,
                ))
                .expect("serialize transaction"),
            ),
        )
        .expect("signed transaction")
    }

    #[test]
    fn accepts_only_canonical_bounded_base64() {
        let valid = BASE64.encode([7_u8; 64]);
        assert_eq!(
            SignedTransaction::parse(&valid).unwrap().bytes(),
            &[7_u8; 64]
        );
        assert!(SignedTransaction::parse(" ").is_err());
        assert!(SignedTransaction::parse(&(valid.clone() + "=")).is_err());
        let oversized = BASE64.encode([9_u8; MAX_SIGNED_TRANSACTION_BYTES + 1]);
        assert!(SignedTransaction::parse(&oversized).is_err());
    }

    #[test]
    fn verifies_the_exact_signed_settlement_instruction() {
        let payer = Keypair::new();
        let expected = compile_settlement_instruction(&compile_request(&payer)).expect("compile");
        let signed = signed_settlement(&expected, &payer);
        signed
            .verify_settlement_instruction(&expected)
            .expect("exact instruction must verify");

        let mut wrong_data = expected.clone();
        wrong_data.instruction_data_base64 = BASE64.encode([0; 8]);
        assert!(signed.verify_settlement_instruction(&wrong_data).is_err());

        let mut wrong_account = expected.clone();
        wrong_account.accounts.swap(0, 1);
        assert!(
            signed
                .verify_settlement_instruction(&wrong_account)
                .is_err()
        );

        let mut wrong_program = expected;
        wrong_program.program_id = solana_transaction::Address::new_unique().to_string();
        assert!(
            signed
                .verify_settlement_instruction(&wrong_program)
                .is_err()
        );
    }

    #[test]
    fn verifies_the_exact_signed_adjustment_instruction() {
        let approver = Keypair::new();
        let expected =
            compile_settlement_adjustment_instruction(&compile_adjustment_request(&approver))
                .expect("compile adjustment");
        let signed = signed_adjustment(&expected, &approver);
        signed
            .verify_adjustment_instruction(&expected)
            .expect("exact adjustment must verify");

        let mut wrong_data = expected.clone();
        wrong_data.instruction_data_base64 = BASE64.encode([0; 8]);
        assert!(signed.verify_adjustment_instruction(&wrong_data).is_err());

        let mut wrong_account = expected.clone();
        wrong_account.accounts.swap(0, 1);
        assert!(
            signed
                .verify_adjustment_instruction(&wrong_account)
                .is_err()
        );

        let extra = signed_with_instruction_sequence(
            &approver,
            &[solana_transaction::Instruction {
                program_id: solana_transaction::Address::new_unique(),
                accounts: Vec::new(),
                data: vec![1],
            }],
        );
        assert!(extra.verify_adjustment_instruction(&expected).is_err());
    }

    #[test]
    fn verifies_the_exact_signed_refund_instruction() {
        let proposer = Keypair::new();
        let refund_funder = Keypair::new();
        let expected = compile_settlement_refund_claim_instruction(&refund_claim_request(
            &proposer,
            &refund_funder,
        ))
        .expect("compile refund claim");
        let signed = signed_refund(&expected, &proposer, &refund_funder);
        signed
            .verify_refund_instruction(&expected)
            .expect("exact refund instruction must verify");

        let mut wrong_data = expected.clone();
        wrong_data.instruction_data_base64 = BASE64.encode([0; 8]);
        assert!(signed.verify_refund_instruction(&wrong_data).is_err());

        let mut wrong_account = expected.clone();
        wrong_account.accounts.swap(0, 1);
        assert!(signed.verify_refund_instruction(&wrong_account).is_err());

        let base_instruction = solana_transaction::Instruction {
            program_id: expected.program_id.parse().expect("program id"),
            accounts: expected
                .accounts
                .iter()
                .map(|account| {
                    let pubkey = account.pubkey.parse().expect("account pubkey");
                    if account.is_writable {
                        solana_transaction::AccountMeta::new(pubkey, account.is_signer)
                    } else {
                        solana_transaction::AccountMeta::new_readonly(pubkey, account.is_signer)
                    }
                })
                .collect(),
            data: BASE64
                .decode(&expected.instruction_data_base64)
                .expect("instruction data"),
        };
        let mut extra_tx = solana_transaction::Transaction::new_with_payer(
            &[
                base_instruction,
                solana_transaction::Instruction {
                    program_id: solana_transaction::Address::new_unique(),
                    accounts: Vec::new(),
                    data: vec![1],
                },
            ],
            Some(&proposer.pubkey()),
        );
        let extra_blockhash = solana_transaction::Hash::new_from_array([9; 32]);
        extra_tx
            .try_partial_sign(&[&proposer], extra_blockhash)
            .expect("sign proposer");
        extra_tx
            .try_partial_sign(&[&refund_funder], extra_blockhash)
            .expect("sign refund funder");
        let extra = SignedTransaction::parse(
            &BASE64.encode(
                wincode::serialize(&solana_transaction::versioned::VersionedTransaction::from(
                    extra_tx,
                ))
                .expect("serialize transaction"),
            ),
        )
        .expect("signed transaction");
        assert!(extra.verify_refund_instruction(&expected).is_err());
    }

    #[test]
    fn verifies_the_exact_signed_refund_cancellation_instruction() {
        let governance_authority = Keypair::new();
        let request = refund_cancellation_request(&governance_authority);
        let expected = compile_settlement_refund_cancellation_instruction(&request)
            .expect("compile refund cancellation");
        let signed = signed_refund_cancellation(&expected, &governance_authority);
        signed
            .verify_refund_cancellation_instruction(&expected)
            .expect("exact refund cancellation must verify");

        let mut wrong_data = expected.clone();
        wrong_data.instruction_data_base64 = BASE64.encode([0; 16]);
        assert!(
            signed
                .verify_refund_cancellation_instruction(&wrong_data)
                .is_err()
        );

        let mut wrong_account = expected.clone();
        wrong_account.accounts.swap(0, 1);
        assert!(
            signed
                .verify_refund_cancellation_instruction(&wrong_account)
                .is_err()
        );

        let extra = signed_with_instruction_sequence(
            &governance_authority,
            &[
                solana_transaction::Instruction {
                    program_id: expected.program_id.parse().expect("program id"),
                    accounts: expected
                        .accounts
                        .iter()
                        .map(|account| {
                            let pubkey = account.pubkey.parse().expect("account pubkey");
                            solana_transaction::AccountMeta::new_readonly(pubkey, account.is_signer)
                        })
                        .collect(),
                    data: BASE64
                        .decode(&expected.instruction_data_base64)
                        .expect("instruction data"),
                },
                solana_transaction::Instruction {
                    program_id: solana_transaction::Address::new_unique(),
                    accounts: Vec::new(),
                    data: vec![1],
                },
            ],
        );
        assert!(
            extra
                .verify_refund_cancellation_instruction(&expected)
                .is_err()
        );
    }

    fn revenue_stream_cancellation_request(
        governance_authority: &Keypair,
    ) -> RevenueStreamClaimCancellationCompileRequest {
        RevenueStreamClaimCancellationCompileRequest {
            program_id: settlement_client::SETTLEMENT_PROGRAM_ID.to_string(),
            governance_authority: governance_authority.pubkey().to_string(),
            plan: solana_transaction::Address::new_unique().to_string(),
            nonce: 17,
        }
    }

    fn signed_revenue_stream_instruction(
        expected: &CompiledRevenueStreamInstruction,
        payer: &Keypair,
    ) -> SignedTransaction {
        let instruction = solana_transaction::Instruction {
            program_id: expected.program_id.parse().expect("program id"),
            accounts: expected
                .accounts
                .iter()
                .map(|account| {
                    let pubkey = account.pubkey.parse().expect("account pubkey");
                    if account.is_writable {
                        solana_transaction::AccountMeta::new(pubkey, account.is_signer)
                    } else {
                        solana_transaction::AccountMeta::new_readonly(pubkey, account.is_signer)
                    }
                })
                .collect(),
            data: BASE64
                .decode(&expected.instruction_data_base64)
                .expect("instruction data"),
        };
        signed_with_instruction_sequence(payer, &[instruction])
    }

    #[test]
    fn verifies_the_exact_signed_revenue_stream_instruction() {
        let governance_authority = Keypair::new();
        let request = revenue_stream_cancellation_request(&governance_authority);
        let expected = compile_revenue_stream_claim_cancellation_instruction(&request)
            .expect("compile revenue stream cancellation");
        let signed = signed_revenue_stream_instruction(&expected, &governance_authority);
        signed
            .verify_revenue_stream_instruction(&expected)
            .expect("exact revenue stream instruction must verify");

        let mut wrong_data = expected.clone();
        wrong_data.instruction_data_base64 = BASE64.encode([0; 16]);
        assert!(
            signed
                .verify_revenue_stream_instruction(&wrong_data)
                .is_err()
        );

        let mut wrong_account = expected.clone();
        wrong_account.accounts.swap(0, 1);
        assert!(
            signed
                .verify_revenue_stream_instruction(&wrong_account)
                .is_err()
        );

        let extra = signed_with_instruction_sequence(
            &governance_authority,
            &[
                solana_transaction::Instruction {
                    program_id: expected.program_id.parse().expect("program id"),
                    accounts: expected
                        .accounts
                        .iter()
                        .map(|account| {
                            let pubkey = account.pubkey.parse().expect("account pubkey");
                            if account.is_writable {
                                solana_transaction::AccountMeta::new(pubkey, account.is_signer)
                            } else {
                                solana_transaction::AccountMeta::new_readonly(
                                    pubkey,
                                    account.is_signer,
                                )
                            }
                        })
                        .collect(),
                    data: BASE64
                        .decode(&expected.instruction_data_base64)
                        .expect("instruction data"),
                },
                solana_transaction::Instruction {
                    program_id: solana_transaction::Address::new_unique(),
                    accounts: Vec::new(),
                    data: vec![1],
                },
            ],
        );
        assert!(extra.verify_revenue_stream_instruction(&expected).is_err());

        let mut not_ready = expected;
        not_ready.status = "submitted".into();
        assert!(
            signed
                .verify_revenue_stream_instruction(&not_ready)
                .is_err()
        );
    }

    #[test]
    fn rejects_missing_compute_budget_pair() {
        let payer = Keypair::new();
        let expected = compile_settlement_instruction(&compile_request(&payer)).expect("compile");
        let signed = signed_with_instruction_sequence(&payer, &[settlement_instruction(&expected)]);

        assert!(signed.verify_settlement_instruction(&expected).is_err());
    }

    #[test]
    fn rejects_reordered_or_mutated_compute_budget_instructions() {
        let payer = Keypair::new();
        let expected = compile_settlement_instruction(&compile_request(&payer)).expect("compile");
        let [limit, price] = compute_budget_instructions(&expected);
        let settlement = settlement_instruction(&expected);

        let reordered = signed_with_instruction_sequence(
            &payer,
            &[price.clone(), limit.clone(), settlement.clone()],
        );
        assert!(reordered.verify_settlement_instruction(&expected).is_err());

        let mut mutated_limit = limit;
        mutated_limit.data[1] = mutated_limit.data[1].saturating_add(1);
        let mutated = signed_with_instruction_sequence(&payer, &[mutated_limit, price, settlement]);
        assert!(mutated.verify_settlement_instruction(&expected).is_err());
    }

    #[test]
    fn v0_requires_server_owned_lookup_resolution() {
        let payer = Keypair::new();
        let expected = compile_settlement_instruction(&compile_request(&payer)).expect("compile");
        let (signed, _) = signed_v0_settlement_with_lookup(&expected, &payer);

        let error = signed
            .verify_settlement_instruction(&expected)
            .expect_err("v0 must not use unresolved client metadata");
        assert!(error.to_string().contains("server-resolved address tables"));
    }

    #[test]
    fn verifies_v0_against_server_resolved_lookup_contents() {
        let payer = Keypair::new();
        let expected = compile_settlement_instruction(&compile_request(&payer)).expect("compile");
        let (signed, table) = signed_v0_settlement_with_lookup(&expected, &payer);

        signed
            .verify_settlement_instruction_with_lookup_tables(&expected, &[table])
            .expect("resolved v0 settlement must verify");
    }

    #[tokio::test]
    async fn resolver_path_fetches_only_signed_lookup_tables() {
        let payer = Keypair::new();
        let expected = compile_settlement_instruction(&compile_request(&payer)).expect("compile");
        let (signed, table) = signed_v0_settlement_with_lookup(&expected, &payer);
        let resolver = FakeLookupResolver {
            table,
            calls: Mutex::new(Vec::new()),
        };

        signed
            .verify_settlement_instruction_with_resolver(&expected, &resolver)
            .await
            .expect("resolver-backed v0 settlement must verify");
        assert_eq!(resolver.calls.lock().unwrap().len(), 1);
    }

    #[test]
    fn parses_only_active_lookup_table_account_data() {
        use std::borrow::Cow;
        let table = solana_transaction::Address::new_unique();
        let addresses = vec![
            solana_transaction::Address::new_unique(),
            solana_transaction::Address::new_unique(),
        ];
        let state = AddressLookupTable {
            meta: solana_address_lookup_table_interface::state::LookupTableMeta::default(),
            addresses: Cow::Owned(addresses.clone()),
        };
        let response = json!({
            "value": {
                "owner": address_lookup_program::ID.to_string(),
                "data": [BASE64.encode(state.clone().serialize_for_tests().expect("serialize table")), "base64"]
            }
        });

        let resolved = parse_lookup_table_response(&table.to_string(), &response)
            .expect("valid lookup table response");
        assert_eq!(resolved.table, table.to_string());
        assert_eq!(resolved.addresses.len(), addresses.len());

        let mut deactivated = state;
        deactivated.meta.deactivation_slot = 7;
        let response = json!({
            "value": {
                "owner": address_lookup_program::ID.to_string(),
                "data": [BASE64.encode(deactivated.serialize_for_tests().expect("serialize table")), "base64"]
            }
        });
        assert!(parse_lookup_table_response(&table.to_string(), &response).is_err());
    }

    #[tokio::test]
    async fn rpc_lookup_resolver_requires_finalized_owned_base64_account() {
        use std::borrow::Cow;
        let table = solana_transaction::Address::new_unique();
        let state = AddressLookupTable {
            meta: solana_address_lookup_table_interface::state::LookupTableMeta::default(),
            addresses: Cow::Owned(vec![solana_transaction::Address::new_unique()]),
        };
        let response = json!({
            "value": {
                "owner": address_lookup_program::ID.to_string(),
                "data": [BASE64.encode(state.serialize_for_tests().expect("serialize table")), "base64"]
            }
        });
        let transport = Arc::new(FakeTransport {
            responses: Mutex::new(vec![Ok(response)]),
            ..Default::default()
        });
        let coordinator =
            SettlementCoordinator::new(vec![transport.clone()], DEFAULT_ATTEMPT_TIMEOUT)
                .expect("coordinator");
        let resolved = AddressLookupTableResolver::resolve(&coordinator, &table.to_string())
            .await
            .expect("lookup table must resolve");
        assert_eq!(resolved.table, table.to_string());
        let requests = transport.requests.lock().unwrap();
        assert_eq!(requests[0].0, "getAccountInfo");
        assert_eq!(requests[0].1[1]["commitment"], "finalized");
        assert_eq!(requests[0].1[1]["encoding"], "base64");
    }

    #[tokio::test]
    async fn adjustment_registry_verification_requires_finalized_provider_agreement() {
        let merchant = Address::new_unique();
        let mint = Address::new_unique();
        let proposer = Address::new_unique();
        let approver = Address::new_unique();
        let request = AdjustmentRegistryVerificationRequest {
            merchant: merchant.to_string(),
            mint: mint.to_string(),
            proposer: proposer.to_string(),
            approver: approver.to_string(),
            governance: None,
        };
        let first = Arc::new(FakeTransport {
            responses: Mutex::new(vec![Ok(adjustment_registry_response(
                merchant, mint, proposer, approver, 7, 120,
            ))]),
            ..Default::default()
        });
        let second = Arc::new(FakeTransport {
            responses: Mutex::new(vec![Ok(adjustment_registry_response(
                merchant, mint, proposer, approver, 7, 121,
            ))]),
            ..Default::default()
        });
        let coordinator = SettlementCoordinator::new(
            vec![first.clone(), second.clone()],
            DEFAULT_ATTEMPT_TIMEOUT,
        )
        .expect("coordinator");

        let verification = coordinator
            .verify_adjustment_registry(&request)
            .await
            .expect("matching finalized registries must verify");
        assert_eq!(verification.finalized_slot, 120);
        assert_eq!(verification.commitment, "finalized");
        assert_ne!(verification.account_digest, [0; 32]);
        let requests = first.requests.lock().unwrap();
        assert_eq!(requests[0].0, "getAccountInfo");
        assert_eq!(requests[0].1[1]["commitment"], "finalized");
        assert_eq!(requests[0].1[1]["encoding"], "base64");
    }

    #[tokio::test]
    async fn adjustment_registry_verification_rejects_valid_provider_disagreement() {
        let merchant = Address::new_unique();
        let mint = Address::new_unique();
        let proposer = Address::new_unique();
        let approver = Address::new_unique();
        let request = AdjustmentRegistryVerificationRequest {
            merchant: merchant.to_string(),
            mint: mint.to_string(),
            proposer: proposer.to_string(),
            approver: approver.to_string(),
            governance: None,
        };
        let first = Arc::new(FakeTransport {
            responses: Mutex::new(vec![Ok(adjustment_registry_response(
                merchant, mint, proposer, approver, 7, 120,
            ))]),
            ..Default::default()
        });
        let second = Arc::new(FakeTransport {
            responses: Mutex::new(vec![Ok(adjustment_registry_response(
                merchant, mint, proposer, approver, 8, 120,
            ))]),
            ..Default::default()
        });
        let coordinator = SettlementCoordinator::new(vec![first, second], DEFAULT_ATTEMPT_TIMEOUT)
            .expect("coordinator");

        let error = coordinator
            .verify_adjustment_registry(&request)
            .await
            .expect_err("conflicting valid registry bytes must fail closed");
        assert!(error.to_string().contains("conflicting registry bytes"));
    }

    #[test]
    fn adjustment_registry_parser_rejects_wrong_owner_inactive_and_layout() {
        let merchant = Address::new_unique();
        let mint = Address::new_unique();
        let proposer = Address::new_unique();
        let approver = Address::new_unique();
        let (registry, bump) =
            settlement_client::adjustment_registry_pda(&merchant.to_string(), &mint.to_string())
                .expect("registry PDA");
        let expected = [merchant, mint, proposer, approver];
        let valid = adjustment_registry_response(merchant, mint, proposer, approver, 7, 120);
        let wrong_owner = json!({
            "context": { "slot": 120 },
            "value": {
                "owner": settlement_client::SYSTEM_PROGRAM_ID,
                "executable": false,
                "data": valid["value"]["data"].clone()
            }
        });
        assert!(
            parse_adjustment_registry_response(&registry, bump, &expected, None, &wrong_owner)
                .is_err()
        );

        let mut inactive = valid.clone();
        let inactive_data = BASE64
            .decode(inactive["value"]["data"][0].as_str().unwrap())
            .unwrap();
        let mut inactive_bytes = inactive_data;
        inactive_bytes[168] = 0;
        inactive["value"]["data"][0] = Value::String(BASE64.encode(inactive_bytes));
        assert!(
            parse_adjustment_registry_response(&registry, bump, &expected, None, &inactive)
                .is_err()
        );

        let malformed = json!({
            "context": { "slot": 120 },
            "value": {
                "owner": settlement_client::SETTLEMENT_PROGRAM_ID,
                "executable": false,
                "data": [BASE64.encode([0_u8; 8]), "base64"]
            }
        });
        assert!(
            parse_adjustment_registry_response(&registry, bump, &expected, None, &malformed)
                .is_err()
        );
    }

    #[tokio::test]
    async fn rpc_lookup_resolver_fails_over_only_on_transport_failure() {
        use std::borrow::Cow;
        let table = solana_transaction::Address::new_unique();
        let state = AddressLookupTable {
            meta: solana_address_lookup_table_interface::state::LookupTableMeta::default(),
            addresses: Cow::Owned(vec![solana_transaction::Address::new_unique()]),
        };
        let response = json!({
            "value": {
                "owner": address_lookup_program::ID.to_string(),
                "data": [BASE64.encode(state.serialize_for_tests().expect("serialize table")), "base64"]
            }
        });
        let first = Arc::new(FakeTransport {
            responses: Mutex::new(vec![Err(ProviderError::Transport("offline".into()))]),
            ..Default::default()
        });
        let second = Arc::new(FakeTransport {
            responses: Mutex::new(vec![Ok(response)]),
            ..Default::default()
        });
        let coordinator = SettlementCoordinator::new(
            vec![first.clone(), second.clone()],
            DEFAULT_ATTEMPT_TIMEOUT,
        )
        .expect("coordinator");

        AddressLookupTableResolver::resolve(&coordinator, &table.to_string())
            .await
            .expect("lookup table must fail over after transport failure");
        assert_eq!(first.requests.lock().unwrap().len(), 1);
        assert_eq!(second.requests.lock().unwrap().len(), 1);
    }

    #[test]
    fn rejects_v0_lookup_with_substituted_expanded_address() {
        let payer = Keypair::new();
        let expected = compile_settlement_instruction(&compile_request(&payer)).expect("compile");
        let (signed, mut table) = signed_v0_settlement_with_lookup(&expected, &payer);
        table.addresses[3] = solana_transaction::Address::new_unique().to_string();

        assert!(
            signed
                .verify_settlement_instruction_with_lookup_tables(&expected, &[table])
                .is_err()
        );
    }

    proptest! {
        #[test]
        fn rejects_every_generated_instruction_account_substitution(
            position in 0usize..SETTLEMENT_INSTRUCTION_ACCOUNT_COUNT,
        ) {
            let payer = Keypair::new();
            let expected = compile_settlement_instruction(&compile_request(&payer))
                .expect("valid instruction");
            let signed = signed_settlement_with_account_substitution(&expected, &payer, position);

            prop_assert!(signed.verify_settlement_instruction(&expected).is_err());
        }

        #[test]
        fn rejects_every_generated_writable_privilege_substitution(
            position in 1usize..SETTLEMENT_INSTRUCTION_ACCOUNT_COUNT,
        ) {
            let payer = Keypair::new();
            let expected = compile_settlement_instruction(&compile_request(&payer))
                .expect("valid instruction");
            let signed = signed_settlement_with_writable_substitution(&expected, &payer, position);

            prop_assert!(signed.verify_settlement_instruction(&expected).is_err());
        }
    }

    #[test]
    fn rejects_trailing_bytes_and_invalid_signatures() {
        let payer = Keypair::new();
        let expected = compile_settlement_instruction(&compile_request(&payer)).expect("compile");
        let mut bytes = signed_settlement(&expected, &payer).bytes().to_vec();
        bytes.push(0);
        let trailing = SignedTransaction::parse(&BASE64.encode(bytes)).expect("bounded bytes");
        assert!(trailing.verify_settlement_instruction(&expected).is_err());

        let mut bytes = signed_settlement(&expected, &payer).bytes().to_vec();
        bytes[1] ^= 1;
        let invalid_signature = SignedTransaction::parse(&BASE64.encode(bytes)).expect("bytes");
        assert!(
            invalid_signature
                .verify_settlement_instruction(&expected)
                .is_err()
        );
    }

    #[tokio::test]
    async fn submits_immutable_bytes_with_explicit_preflight_policy() {
        let tx = transaction();
        let transport = Arc::new(FakeTransport {
            responses: Mutex::new(vec![Ok(json!(tx.signature().expect("signature")))]),
            ..Default::default()
        });
        let coordinator =
            SettlementCoordinator::new(vec![transport.clone()], DEFAULT_ATTEMPT_TIMEOUT).unwrap();
        let receipt = coordinator.submit(&tx).await.unwrap();
        assert_eq!(receipt.status, SettlementStatus::Submitted);
        let requests = transport.requests.lock().unwrap();
        assert_eq!(requests[0].0, "sendTransaction");
        assert_eq!(requests[0].1[1]["encoding"], "base64");
        assert_eq!(requests[0].1[1]["skipPreflight"], false);
        assert_eq!(requests[0].1[1]["maxRetries"], 0);
    }

    #[tokio::test]
    async fn rejects_provider_signature_that_does_not_match_signed_bytes() {
        let payer = Keypair::new();
        let expected = compile_settlement_instruction(&compile_request(&payer)).expect("compile");
        let signed = signed_settlement(&expected, &payer);
        let transport = Arc::new(FakeTransport {
            responses: Mutex::new(vec![Ok(json!("5".repeat(88)))]),
            ..Default::default()
        });
        let coordinator =
            SettlementCoordinator::new(vec![transport], DEFAULT_ATTEMPT_TIMEOUT).unwrap();

        let error = coordinator.submit(&signed).await.unwrap_err();

        assert!(matches!(
            error,
            CoordinatorError::InvalidProviderResponse(message)
                if message.contains("does not match the signed transaction")
        ));
    }

    #[test]
    fn derives_a_stable_signature_from_verified_signed_bytes() {
        let payer = Keypair::new();
        let expected = compile_settlement_instruction(&compile_request(&payer)).expect("compile");
        let signed = signed_settlement(&expected, &payer);

        let signature = signed.signature().expect("signature");

        assert!(!signature.is_empty());
        assert_eq!(signature, signed.signature().expect("same signature"));
    }

    #[tokio::test]
    async fn validates_recent_blockhash_at_confirmed_commitment() {
        let payer = Keypair::new();
        let expected = compile_settlement_instruction(&compile_request(&payer)).expect("compile");
        let signed = signed_settlement(&expected, &payer);
        let transport = Arc::new(FakeTransport {
            responses: Mutex::new(vec![Ok(json!({"value": true}))]),
            ..Default::default()
        });
        let coordinator =
            SettlementCoordinator::new(vec![transport.clone()], DEFAULT_ATTEMPT_TIMEOUT).unwrap();
        coordinator.validate_blockhash(&signed).await.unwrap();
        let requests = transport.requests.lock().unwrap();
        assert_eq!(requests[0].0, "isBlockhashValid");
        assert_eq!(requests[0].1[1]["commitment"], "confirmed");
    }

    #[tokio::test]
    async fn rejects_stale_blockhash_without_failover() {
        let payer = Keypair::new();
        let expected = compile_settlement_instruction(&compile_request(&payer)).expect("compile");
        let signed = signed_settlement(&expected, &payer);
        let transport = Arc::new(FakeTransport {
            responses: Mutex::new(vec![Ok(json!({"value": false}))]),
            ..Default::default()
        });
        let coordinator =
            SettlementCoordinator::new(vec![transport], DEFAULT_ATTEMPT_TIMEOUT).unwrap();
        assert!(matches!(
            coordinator.validate_blockhash(&signed).await,
            Err(CoordinatorError::InvalidSignedTransaction(_))
        ));
    }

    #[tokio::test]
    async fn definitive_stale_blockhash_does_not_call_a_second_provider() {
        let payer = Keypair::new();
        let expected = compile_settlement_instruction(&compile_request(&payer)).expect("compile");
        let signed = signed_settlement(&expected, &payer);
        let first = Arc::new(FakeTransport {
            responses: Mutex::new(vec![Ok(json!({"value": false}))]),
            ..Default::default()
        });
        let second = Arc::new(FakeTransport {
            responses: Mutex::new(vec![Ok(json!({"value": true}))]),
            ..Default::default()
        });
        let coordinator =
            SettlementCoordinator::new(vec![first, second.clone()], DEFAULT_ATTEMPT_TIMEOUT)
                .unwrap();

        assert!(matches!(
            coordinator.validate_blockhash(&signed).await,
            Err(CoordinatorError::InvalidSignedTransaction(_))
        ));
        assert!(second.requests.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn fails_over_without_mutating_signed_bytes() {
        let tx = transaction();
        let signature = tx.signature().expect("signature");
        let first = Arc::new(FakeTransport {
            responses: Mutex::new(vec![Err(ProviderError::Transport("offline".into()))]),
            ..Default::default()
        });
        let second = Arc::new(FakeTransport {
            responses: Mutex::new(vec![Ok(json!(signature))]),
            ..Default::default()
        });
        let coordinator =
            SettlementCoordinator::new(vec![first, second.clone()], DEFAULT_ATTEMPT_TIMEOUT)
                .unwrap();
        coordinator.submit(&tx).await.unwrap();
        let requests = second.requests.lock().unwrap();
        assert_eq!(requests[0].1[0], tx.canonical_base64());
    }

    #[tokio::test]
    async fn adaptive_health_prefers_a_successful_provider_after_failure() {
        let tx = transaction();
        let signature = tx.signature().expect("signature");
        let first = Arc::new(FakeTransport {
            responses: Mutex::new(vec![Err(ProviderError::Transport("offline".into()))]),
            ..Default::default()
        });
        let second = Arc::new(FakeTransport {
            responses: Mutex::new(vec![Ok(json!(signature.clone())), Ok(json!(signature))]),
            ..Default::default()
        });
        let coordinator = SettlementCoordinator::new(
            vec![first.clone(), second.clone()],
            DEFAULT_ATTEMPT_TIMEOUT,
        )
        .unwrap();

        coordinator.submit(&tx).await.unwrap();
        let receipt = coordinator.submit(&tx).await.unwrap();

        assert_eq!(receipt.provider_index, 1);
        assert_eq!(first.requests.lock().unwrap().len(), 1);
        assert_eq!(second.requests.lock().unwrap().len(), 2);
        let health = coordinator.health_snapshot();
        assert_eq!(health[0].provider_index, 0);
        assert_eq!(health[0].score, -1);
        assert_eq!(health[0].consecutive_failures, 1);
        assert_eq!(health[1].provider_index, 1);
        assert_eq!(health[1].score, 2);
        assert_eq!(health[1].consecutive_failures, 0);
    }

    #[tokio::test]
    async fn maps_commitments_and_unknown_status_without_overclaiming() {
        let transport = Arc::new(FakeTransport {
            responses: Mutex::new(vec![
                Ok(json!({"value": [null]})),
                Ok(
                    json!({"value": [{"slot": 42, "err": null, "confirmationStatus": "finalized"}]}),
                ),
            ]),
            ..Default::default()
        });
        let coordinator =
            SettlementCoordinator::new(vec![transport], DEFAULT_ATTEMPT_TIMEOUT).unwrap();
        let unknown = coordinator.observe(&"5".repeat(88)).await.unwrap();
        assert_eq!(unknown.status, SettlementStatus::Unknown);
        let finalized = coordinator.observe(&"5".repeat(88)).await.unwrap();
        assert_eq!(finalized.status, SettlementStatus::Finalized);
        assert_eq!(finalized.slot, Some(42));
    }

    #[tokio::test]
    async fn maps_chain_error_to_rejected() {
        let transport = Arc::new(FakeTransport {
            responses: Mutex::new(vec![Ok(json!({
                "value": [{"slot": 42, "err": {"InstructionError": [0, "Custom"]}, "confirmationStatus": "confirmed"}]
            }))]),
            ..Default::default()
        });
        let coordinator =
            SettlementCoordinator::new(vec![transport], DEFAULT_ATTEMPT_TIMEOUT).unwrap();
        let result = coordinator.observe(&"5".repeat(88)).await.unwrap();
        assert_eq!(result.status, SettlementStatus::Rejected);
    }

    #[test]
    fn lifecycle_does_not_allow_confirmation_regression() {
        assert!(can_transition(
            SettlementStatus::Submitted,
            SettlementStatus::Confirmed
        ));
        assert!(can_transition(
            SettlementStatus::Confirmed,
            SettlementStatus::Finalized
        ));
        assert!(!can_transition(
            SettlementStatus::Finalized,
            SettlementStatus::Submitted
        ));
        assert!(!can_transition(
            SettlementStatus::PreparedUnsubmitted,
            SettlementStatus::Finalized
        ));
    }
}
