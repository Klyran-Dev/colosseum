#![deny(unsafe_code)]

use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use indexer::{AdaptiveProviderHealth, JsonRpcTransport, ProviderError};
use serde_json::{Value, json};
use settlement_client::{
    COMPUTE_BUDGET_PROGRAM_ID, CompiledSettlementInstruction, ComputeBudgetPolicy,
    MAX_SETTLEMENT_SIGNED_BYTES, estimate_legacy_settlement_wire_bytes_with_compute_budget,
};
use solana_address_lookup_table_interface::{
    program as address_lookup_program, state::AddressLookupTable,
};
use std::{sync::Arc, time::Duration};
use thiserror::Error;

pub const MAX_SIGNED_TRANSACTION_BYTES: usize = 1232;
pub const MAX_PROVIDER_ATTEMPTS: usize = 3;
pub const DEFAULT_ATTEMPT_TIMEOUT: Duration = Duration::from_secs(2);
pub const MAX_SIGNATURE_BYTES: usize = 128;
pub const MAX_ADDRESS_LOOKUP_TABLES: usize = 2;
pub const MAX_LOADED_ADDRESSES_PER_TABLE: usize = 32;
pub const MAX_ADDRESS_LOOKUP_TABLE_ADDRESSES: usize = 256;

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
        CompiledSettlementInstruction, SETTLEMENT_INSTRUCTION_ACCOUNT_COUNT,
        SettlementCompileRequest, compile_settlement_instruction,
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
