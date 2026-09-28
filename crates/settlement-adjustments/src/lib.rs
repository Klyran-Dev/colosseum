#![deny(unsafe_code)]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use thiserror::Error;
use uuid::Uuid;

pub const ADJUSTMENT_SCHEMA_V1: &[u8] = b"solana-tax-protocol/settlement-adjustment/v1";
pub const REFUND_BALANCE_PROOF_SCHEMA_V1: &[u8] = b"solana-tax-protocol/refund-balance-proof/v1";
pub const AUTHORITY_CHALLENGE_SCHEMA_V1: &[u8] =
    b"solana-tax-protocol/adjustment-authority-binding/v1";
pub const MAX_REASON_BYTES: usize = 256;
pub const MAX_SUBJECT_BYTES: usize = 256;
pub const MAX_ATOMIC_AMOUNT: i128 = u64::MAX as i128;

/// Returns the canonical wallet-authority challenge bytes. The exact message
/// is shared by the API and wallet clients; it is deliberately explicit and
/// versioned so a signature cannot be replayed for another tenant, role, or
/// expiry window.
pub fn authority_challenge_message(
    merchant_id: Uuid,
    challenge_id: Uuid,
    subject: &str,
    role: &str,
    expires_at_unix: i64,
) -> Result<Vec<u8>, AdjustmentError> {
    if merchant_id.get_version_num() != 7 {
        return Err(AdjustmentError::InvalidUuid("merchant ID"));
    }
    if challenge_id.get_version_num() != 7 {
        return Err(AdjustmentError::InvalidUuid("challenge ID"));
    }
    validate_subject(subject, "authority subject")?;
    if !matches!(role, "proposer" | "approver" | "refund_funder") {
        return Err(AdjustmentError::InvalidAuthorityRole);
    }
    if expires_at_unix <= 0 {
        return Err(AdjustmentError::InvalidAuthorityChallenge);
    }
    Ok(format!(
        "{}\nmerchant_id={}\nchallenge_id={}\nsubject={}\nrole={}\nexpires_at_unix={}\n",
        std::str::from_utf8(AUTHORITY_CHALLENGE_SCHEMA_V1)
            .map_err(|_| AdjustmentError::InvalidAuthorityChallenge)?,
        merchant_id,
        challenge_id,
        subject,
        role,
        expires_at_unix
    )
    .into_bytes())
}

pub fn authority_challenge_digest(message: &[u8]) -> [u8; 32] {
    Sha256::digest(message).into()
}

pub mod serde_i128_string {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(value: &i128, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&value.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<i128, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value.parse::<i128>().map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum AdjustmentKind {
    Refund,
    Reversal,
    Correction,
    Chargeback,
    RemittanceAdjustment,
}

impl AdjustmentKind {
    fn tag(self) -> u8 {
        match self {
            Self::Refund => 1,
            Self::Reversal => 2,
            Self::Correction => 3,
            Self::Chargeback => 4,
            Self::RemittanceAdjustment => 5,
        }
    }

    fn requires_negative_delta(self) -> bool {
        matches!(self, Self::Refund | Self::Reversal | Self::Chargeback)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AdjustmentStatus {
    Proposed,
    Approved,
    Posted,
    Rejected,
    Voided,
}

/// Validates a durable lifecycle transition without consulting a clock,
/// database, or identity provider. The storage adapter supplies the persisted
/// status and subjects; it remains responsible for the transactional compare
/// and set.
pub fn validate_transition(
    current: AdjustmentStatus,
    target: AdjustmentStatus,
    proposer_subject: &str,
    approver_subject: Option<&str>,
    actor_subject: &str,
) -> Result<(), AdjustmentError> {
    validate_subject(proposer_subject, "proposer subject")?;
    validate_subject(actor_subject, "transition subject")?;
    if actor_subject == proposer_subject
        && matches!(
            (current, target),
            (AdjustmentStatus::Proposed, AdjustmentStatus::Approved)
                | (AdjustmentStatus::Proposed, AdjustmentStatus::Rejected)
        )
    {
        return Err(AdjustmentError::ApprovalSubjectMustDiffer);
    }
    match (current, target) {
        (AdjustmentStatus::Proposed, AdjustmentStatus::Approved)
        | (AdjustmentStatus::Proposed, AdjustmentStatus::Rejected) => Ok(()),
        (AdjustmentStatus::Approved, AdjustmentStatus::Voided)
            if approver_subject == Some(actor_subject) =>
        {
            Ok(())
        }
        _ => Err(AdjustmentError::InvalidTransition),
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AdjustmentDelta {
    #[serde(with = "serde_i128_string")]
    pub gross_atomic: i128,
    #[serde(with = "serde_i128_string")]
    pub tax_atomic: i128,
    #[serde(with = "serde_i128_string")]
    pub net_atomic: i128,
}

impl AdjustmentDelta {
    pub fn validate(self) -> Result<(), AdjustmentError> {
        let tax_plus_net = self
            .tax_atomic
            .checked_add(self.net_atomic)
            .ok_or(AdjustmentError::ArithmeticOverflow)?;
        if self.gross_atomic != tax_plus_net {
            return Err(AdjustmentError::DeltaNotConserving);
        }
        if self.gross_atomic == 0 && self.tax_atomic == 0 && self.net_atomic == 0 {
            return Err(AdjustmentError::DeltaMustNotBeZero);
        }
        for value in [self.gross_atomic, self.tax_atomic, self.net_atomic] {
            if value.unsigned_abs() > MAX_ATOMIC_AMOUNT as u128 {
                return Err(AdjustmentError::AtomicAmountOutOfRange);
            }
        }
        Ok(())
    }

    fn is_non_positive_and_non_zero(self) -> bool {
        self.gross_atomic <= 0
            && self.tax_atomic <= 0
            && self.net_atomic <= 0
            && [self.gross_atomic, self.tax_atomic, self.net_atomic]
                .iter()
                .any(|value| *value < 0)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RefundBalanceSnapshot {
    pub finalized_slot: u64,
    pub tax_vault_atomic: u64,
    pub refund_source_atomic: u64,
    pub payer_destination_atomic: u64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RefundBalanceProof {
    pub finalized_slot: u64,
    pub gross_atomic: u64,
    pub tax_atomic: u64,
    pub net_atomic: u64,
    pub tax_vault_before_atomic: u64,
    pub tax_vault_after_atomic: u64,
    pub refund_source_before_atomic: u64,
    pub refund_source_after_atomic: u64,
    pub payer_destination_before_atomic: u64,
    pub payer_destination_after_atomic: u64,
}

impl RefundBalanceProof {
    /// Returns the canonical digest persisted with the immutable proof
    /// evidence. The digest covers the transaction-slot proof values but not
    /// a server-generated database identifier.
    pub fn digest(&self) -> Result<[u8; 32], AdjustmentError> {
        if self.finalized_slot == 0 || self.gross_atomic == 0 {
            return Err(AdjustmentError::DeltaMustNotBeZero);
        }
        if self.tax_atomic.checked_add(self.net_atomic) != Some(self.gross_atomic) {
            return Err(AdjustmentError::DeltaNotConserving);
        }
        if self.tax_vault_before_atomic.checked_sub(self.tax_atomic)
            != Some(self.tax_vault_after_atomic)
        {
            return Err(AdjustmentError::TaxVaultBalanceDeltaMismatch);
        }
        if self
            .refund_source_before_atomic
            .checked_sub(self.net_atomic)
            != Some(self.refund_source_after_atomic)
        {
            return Err(AdjustmentError::RefundSourceBalanceDeltaMismatch);
        }
        if self
            .payer_destination_before_atomic
            .checked_add(self.gross_atomic)
            != Some(self.payer_destination_after_atomic)
        {
            return Err(AdjustmentError::PayerDestinationBalanceDeltaMismatch);
        }
        let mut encoder = CanonicalEncoder::new();
        encoder.bytes(REFUND_BALANCE_PROOF_SCHEMA_V1);
        encoder.u64(self.finalized_slot);
        encoder.u64(self.gross_atomic);
        encoder.u64(self.tax_atomic);
        encoder.u64(self.net_atomic);
        encoder.u64(self.tax_vault_before_atomic);
        encoder.u64(self.tax_vault_after_atomic);
        encoder.u64(self.refund_source_before_atomic);
        encoder.u64(self.refund_source_after_atomic);
        encoder.u64(self.payer_destination_before_atomic);
        encoder.u64(self.payer_destination_after_atomic);
        Ok(Sha256::digest(encoder.finish()).into())
    }
}

/// Proves the exact three-account balance deltas for the first governed
/// negative Refund movement. Both snapshots must come from one finalized
/// account read at the same slot; this function never accepts confirmed or
/// caller-derived balances as proof.
pub fn prove_refund_balance_deltas(
    delta: AdjustmentDelta,
    before: RefundBalanceSnapshot,
    after: RefundBalanceSnapshot,
) -> Result<RefundBalanceProof, AdjustmentError> {
    delta.validate()?;
    if delta.gross_atomic >= 0 || delta.tax_atomic > 0 || delta.net_atomic > 0 {
        return Err(AdjustmentError::RefundBalanceDeltaMustBeNegative);
    }
    if before.finalized_slot == 0 || before.finalized_slot != after.finalized_slot {
        return Err(AdjustmentError::RefundBalanceSlotMismatch);
    }
    let gross_atomic =
        u64::try_from(-delta.gross_atomic).map_err(|_| AdjustmentError::ArithmeticOverflow)?;
    let tax_atomic =
        u64::try_from(-delta.tax_atomic).map_err(|_| AdjustmentError::ArithmeticOverflow)?;
    let net_atomic =
        u64::try_from(-delta.net_atomic).map_err(|_| AdjustmentError::ArithmeticOverflow)?;
    if tax_atomic.checked_add(net_atomic) != Some(gross_atomic) {
        return Err(AdjustmentError::DeltaNotConserving);
    }
    let expected_tax_after = before
        .tax_vault_atomic
        .checked_sub(tax_atomic)
        .ok_or(AdjustmentError::TaxVaultBalanceUnderflow)?;
    if after.tax_vault_atomic != expected_tax_after {
        return Err(AdjustmentError::TaxVaultBalanceDeltaMismatch);
    }
    let expected_source_after = before
        .refund_source_atomic
        .checked_sub(net_atomic)
        .ok_or(AdjustmentError::RefundSourceBalanceUnderflow)?;
    if after.refund_source_atomic != expected_source_after {
        return Err(AdjustmentError::RefundSourceBalanceDeltaMismatch);
    }
    let expected_destination_after = before
        .payer_destination_atomic
        .checked_add(gross_atomic)
        .ok_or(AdjustmentError::PayerDestinationBalanceOverflow)?;
    if after.payer_destination_atomic != expected_destination_after {
        return Err(AdjustmentError::PayerDestinationBalanceDeltaMismatch);
    }
    Ok(RefundBalanceProof {
        finalized_slot: before.finalized_slot,
        gross_atomic,
        tax_atomic,
        net_atomic,
        tax_vault_before_atomic: before.tax_vault_atomic,
        tax_vault_after_atomic: after.tax_vault_atomic,
        refund_source_before_atomic: before.refund_source_atomic,
        refund_source_after_atomic: after.refund_source_atomic,
        payer_destination_before_atomic: before.payer_destination_atomic,
        payer_destination_after_atomic: after.payer_destination_atomic,
    })
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OriginalSettlementAmounts {
    pub gross_atomic: u64,
    pub tax_atomic: u64,
    pub net_atomic: u64,
}

impl OriginalSettlementAmounts {
    fn validate(self) -> Result<(), AdjustmentError> {
        if self.gross_atomic == 0
            || self.tax_atomic.checked_add(self.net_atomic) != Some(self.gross_atomic)
        {
            return Err(AdjustmentError::InvalidOriginalAmounts);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AdjustmentProposal {
    pub id: Uuid,
    pub merchant_id: Uuid,
    pub original_settlement_id: Uuid,
    pub asset_id: [u8; 32],
    pub kind: AdjustmentKind,
    pub delta: AdjustmentDelta,
    pub reason_code: String,
    pub evidence_digest: [u8; 32],
    pub source_calculation_id: Uuid,
    pub replacement_calculation_id: Option<Uuid>,
    pub proposed_by: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Adjustment {
    id: Uuid,
    merchant_id: Uuid,
    original_settlement_id: Uuid,
    asset_id: [u8; 32],
    kind: AdjustmentKind,
    delta: AdjustmentDelta,
    reason_code: String,
    evidence_digest: [u8; 32],
    source_calculation_id: Uuid,
    replacement_calculation_id: Option<Uuid>,
    proposed_by: String,
    approved_by: Option<String>,
    status: AdjustmentStatus,
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum AdjustmentError {
    #[error("{0} must be UUIDv7")]
    InvalidUuid(&'static str),
    #[error("{0} is empty, invalid, or too long")]
    InvalidSubject(&'static str),
    #[error("adjustment and original settlement IDs must differ")]
    AdjustmentReferencesItself,
    #[error("asset identifier must not be all zeroes")]
    EmptyAsset,
    #[error("reason code is empty, invalid, or too long")]
    InvalidReasonCode,
    #[error("evidence digest must not be all zeroes")]
    EmptyEvidenceDigest,
    #[error("tax, net, and gross deltas do not conserve")]
    DeltaNotConserving,
    #[error("adjustment delta must not be zero")]
    DeltaMustNotBeZero,
    #[error("adjustment delta exceeds an unsigned 64-bit atomic amount")]
    AtomicAmountOutOfRange,
    #[error("refund, reversal, and chargeback deltas must be negative")]
    KindRequiresNegativeDelta,
    #[error("approval subject must differ from proposer")]
    ApprovalSubjectMustDiffer,
    #[error("invalid adjustment lifecycle transition")]
    InvalidTransition,
    #[error("adjustment requires approval before posting")]
    NotApproved,
    #[error("original settlement amounts are not positive and conserving")]
    InvalidOriginalAmounts,
    #[error("adjustment arithmetic overflowed")]
    ArithmeticOverflow,
    #[error("adjustment exceeds the original settlement amount")]
    AdjustmentExceedsOriginal,
    #[error("same adjustment ID was reused with different evidence")]
    IdempotencyConflict,
    #[error("adjustment authority role is invalid")]
    InvalidAuthorityRole,
    #[error("adjustment authority challenge is invalid")]
    InvalidAuthorityChallenge,
    #[error("refund balance proof requires a negative conserving delta")]
    RefundBalanceDeltaMustBeNegative,
    #[error("refund balance snapshots must share a non-zero finalized slot")]
    RefundBalanceSlotMismatch,
    #[error("tax vault balance is insufficient for the refund tax leg")]
    TaxVaultBalanceUnderflow,
    #[error("tax vault balance delta does not match the refund tax leg")]
    TaxVaultBalanceDeltaMismatch,
    #[error("refund source balance is insufficient for the refund net leg")]
    RefundSourceBalanceUnderflow,
    #[error("refund source balance delta does not match the refund net leg")]
    RefundSourceBalanceDeltaMismatch,
    #[error("payer destination balance overflowed the atomic amount range")]
    PayerDestinationBalanceOverflow,
    #[error("payer destination balance delta does not match the refund gross leg")]
    PayerDestinationBalanceDeltaMismatch,
}

impl Adjustment {
    pub fn propose(proposal: AdjustmentProposal) -> Result<Self, AdjustmentError> {
        validate_uuid(proposal.id, "adjustment ID")?;
        validate_uuid(proposal.merchant_id, "merchant ID")?;
        validate_uuid(proposal.original_settlement_id, "original settlement ID")?;
        validate_uuid(proposal.source_calculation_id, "source calculation ID")?;
        validate_subject(&proposal.proposed_by, "proposer subject")?;
        if proposal.id == proposal.original_settlement_id {
            return Err(AdjustmentError::AdjustmentReferencesItself);
        }
        if let Some(replacement) = proposal.replacement_calculation_id {
            validate_uuid(replacement, "replacement calculation ID")?;
        }
        if proposal.asset_id == [0; 32] {
            return Err(AdjustmentError::EmptyAsset);
        }
        if proposal.evidence_digest == [0; 32] {
            return Err(AdjustmentError::EmptyEvidenceDigest);
        }
        let reason_code = proposal.reason_code.trim().to_string();
        if reason_code.is_empty()
            || reason_code.len() > MAX_REASON_BYTES
            || reason_code.bytes().any(|byte| byte.is_ascii_control())
        {
            return Err(AdjustmentError::InvalidReasonCode);
        }
        proposal.delta.validate()?;
        if proposal.kind.requires_negative_delta() && !proposal.delta.is_non_positive_and_non_zero()
        {
            return Err(AdjustmentError::KindRequiresNegativeDelta);
        }
        Ok(Self {
            id: proposal.id,
            merchant_id: proposal.merchant_id,
            original_settlement_id: proposal.original_settlement_id,
            asset_id: proposal.asset_id,
            kind: proposal.kind,
            delta: proposal.delta,
            reason_code,
            evidence_digest: proposal.evidence_digest,
            source_calculation_id: proposal.source_calculation_id,
            replacement_calculation_id: proposal.replacement_calculation_id,
            proposed_by: proposal.proposed_by,
            approved_by: None,
            status: AdjustmentStatus::Proposed,
        })
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn status(&self) -> AdjustmentStatus {
        self.status
    }

    pub fn delta(&self) -> AdjustmentDelta {
        self.delta
    }

    pub fn is_negative_adjustment(&self) -> bool {
        self.delta.is_non_positive_and_non_zero()
    }

    pub fn approve(&mut self, subject: &str) -> Result<(), AdjustmentError> {
        validate_subject(subject, "approval subject")?;
        if self.status != AdjustmentStatus::Proposed {
            return Err(AdjustmentError::InvalidTransition);
        }
        if subject == self.proposed_by {
            return Err(AdjustmentError::ApprovalSubjectMustDiffer);
        }
        self.approved_by = Some(subject.to_string());
        self.status = AdjustmentStatus::Approved;
        Ok(())
    }

    pub fn reject(&mut self) -> Result<(), AdjustmentError> {
        if self.status != AdjustmentStatus::Proposed {
            return Err(AdjustmentError::InvalidTransition);
        }
        self.status = AdjustmentStatus::Rejected;
        Ok(())
    }

    pub fn void(&mut self) -> Result<(), AdjustmentError> {
        if self.status != AdjustmentStatus::Approved {
            return Err(AdjustmentError::InvalidTransition);
        }
        self.status = AdjustmentStatus::Voided;
        Ok(())
    }

    pub fn digest(&self) -> Result<[u8; 32], AdjustmentError> {
        self.delta.validate()?;
        let mut encoder = CanonicalEncoder::new();
        encoder.bytes(ADJUSTMENT_SCHEMA_V1);
        // The record UUID is generated by the server and is not part of the
        // immutable request content. Keeping it outside this digest allows a
        // retry with the same idempotency key to replay the original record
        // even though the API constructs a fresh UUIDv7 before storage checks
        // the request digest.
        encoder.uuid(self.merchant_id);
        encoder.uuid(self.original_settlement_id);
        encoder.bytes(&self.asset_id);
        encoder.u8(self.kind.tag());
        encoder.i128(self.delta.gross_atomic);
        encoder.i128(self.delta.tax_atomic);
        encoder.i128(self.delta.net_atomic);
        encoder.text(&self.reason_code);
        encoder.bytes(&self.evidence_digest);
        encoder.uuid(self.source_calculation_id);
        match self.replacement_calculation_id {
            Some(value) => {
                encoder.u8(1);
                encoder.uuid(value);
            }
            None => encoder.u8(0),
        }
        encoder.text(&self.proposed_by);
        Ok(Sha256::digest(encoder.finish()).into())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PostOutcome {
    Posted,
    Replay,
}

#[derive(Clone, Debug)]
pub struct AdjustmentLedger {
    original: OriginalSettlementAmounts,
    applied: AdjustmentDelta,
    posted: BTreeMap<Uuid, [u8; 32]>,
}

impl AdjustmentLedger {
    pub fn new(original: OriginalSettlementAmounts) -> Self {
        Self {
            original,
            applied: AdjustmentDelta {
                gross_atomic: 0,
                tax_atomic: 0,
                net_atomic: 0,
            },
            posted: BTreeMap::new(),
        }
    }

    pub fn post(&mut self, adjustment: &mut Adjustment) -> Result<PostOutcome, AdjustmentError> {
        let digest = adjustment.digest()?;
        if let Some(existing) = self.posted.get(&adjustment.id) {
            if existing == &digest {
                return Ok(PostOutcome::Replay);
            }
            return Err(AdjustmentError::IdempotencyConflict);
        }
        if adjustment.status != AdjustmentStatus::Approved {
            return Err(AdjustmentError::NotApproved);
        }
        self.original.validate()?;
        let next = AdjustmentDelta {
            gross_atomic: self
                .applied
                .gross_atomic
                .checked_add(adjustment.delta.gross_atomic)
                .ok_or(AdjustmentError::ArithmeticOverflow)?,
            tax_atomic: self
                .applied
                .tax_atomic
                .checked_add(adjustment.delta.tax_atomic)
                .ok_or(AdjustmentError::ArithmeticOverflow)?,
            net_atomic: self
                .applied
                .net_atomic
                .checked_add(adjustment.delta.net_atomic)
                .ok_or(AdjustmentError::ArithmeticOverflow)?,
        };
        validate_effective_amounts(self.original, next)?;
        self.applied = next;
        self.posted.insert(adjustment.id, digest);
        adjustment.status = AdjustmentStatus::Posted;
        Ok(PostOutcome::Posted)
    }

    pub fn effective_amounts(&self) -> Result<OriginalSettlementAmounts, AdjustmentError> {
        self.original.validate()?;
        effective_amounts(self.original, self.applied)
    }
}

fn validate_effective_amounts(
    original: OriginalSettlementAmounts,
    applied: AdjustmentDelta,
) -> Result<(), AdjustmentError> {
    effective_amounts(original, applied).map(|_| ())
}

fn effective_amounts(
    original: OriginalSettlementAmounts,
    applied: AdjustmentDelta,
) -> Result<OriginalSettlementAmounts, AdjustmentError> {
    let values = [
        (original.gross_atomic, applied.gross_atomic),
        (original.tax_atomic, applied.tax_atomic),
        (original.net_atomic, applied.net_atomic),
    ];
    let mut effective = [0_u64; 3];
    for (index, (base, delta)) in values.into_iter().enumerate() {
        let value = i128::from(base)
            .checked_add(delta)
            .ok_or(AdjustmentError::ArithmeticOverflow)?;
        if !(0..=MAX_ATOMIC_AMOUNT).contains(&value) {
            return Err(AdjustmentError::AdjustmentExceedsOriginal);
        }
        effective[index] = u64::try_from(value).map_err(|_| AdjustmentError::ArithmeticOverflow)?;
    }
    Ok(OriginalSettlementAmounts {
        gross_atomic: effective[0],
        tax_atomic: effective[1],
        net_atomic: effective[2],
    })
}

fn validate_uuid(value: Uuid, field: &'static str) -> Result<(), AdjustmentError> {
    if value.get_version_num() != 7 {
        return Err(AdjustmentError::InvalidUuid(field));
    }
    Ok(())
}

fn validate_subject(value: &str, field: &'static str) -> Result<(), AdjustmentError> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > MAX_SUBJECT_BYTES
        || value.bytes().any(|byte| byte.is_ascii_control())
    {
        return Err(AdjustmentError::InvalidSubject(field));
    }
    Ok(())
}

struct CanonicalEncoder {
    bytes: Vec<u8>,
}

impl CanonicalEncoder {
    fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    fn finish(self) -> Vec<u8> {
        self.bytes
    }

    fn bytes(&mut self, value: &[u8]) {
        self.bytes
            .extend_from_slice(&(value.len() as u32).to_le_bytes());
        self.bytes.extend_from_slice(value);
    }

    fn text(&mut self, value: &str) {
        self.bytes(value.as_bytes());
    }

    fn uuid(&mut self, value: Uuid) {
        self.bytes.extend_from_slice(value.as_bytes());
    }

    fn u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    fn u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn i128(&mut self, value: i128) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn id(seed: u8) -> Uuid {
        let mut bytes = [seed; 16];
        bytes[6] = (bytes[6] & 0x0f) | 0x70;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        Uuid::from_bytes(bytes)
    }

    fn proposal(kind: AdjustmentKind, delta: AdjustmentDelta) -> AdjustmentProposal {
        AdjustmentProposal {
            id: id(1),
            merchant_id: id(2),
            original_settlement_id: id(3),
            asset_id: [4; 32],
            kind,
            delta,
            reason_code: "customer_refund".to_string(),
            evidence_digest: [5; 32],
            source_calculation_id: id(6),
            replacement_calculation_id: None,
            proposed_by: "proposer-subject".to_string(),
        }
    }

    fn refund_delta(amount: i128) -> AdjustmentDelta {
        AdjustmentDelta {
            gross_atomic: -amount,
            tax_atomic: 0,
            net_atomic: -amount,
        }
    }

    #[test]
    fn authority_challenge_message_binds_all_replay_sensitive_fields() {
        let merchant = id(20);
        let challenge = id(21);
        let message = authority_challenge_message(
            merchant,
            challenge,
            "approver-subject",
            "approver",
            1_800_000_000,
        )
        .expect("valid authority challenge");
        assert!(
            String::from_utf8(message.clone())
                .expect("utf8 challenge")
                .contains("role=approver")
        );
        assert_ne!(
            authority_challenge_digest(&message),
            authority_challenge_digest(
                &authority_challenge_message(
                    merchant,
                    challenge,
                    "approver-subject",
                    "approver",
                    1_800_000_001,
                )
                .expect("changed challenge")
            )
        );
    }

    #[test]
    fn authority_challenge_rejects_invalid_role_and_subject() {
        let error = authority_challenge_message(id(20), id(21), "subject", "reviewer", 10)
            .expect_err("unsupported authority role");
        assert_eq!(error, AdjustmentError::InvalidAuthorityRole);
        let refund_funder =
            authority_challenge_message(id(20), id(21), "subject", "refund_funder", 10)
                .expect("refund-funder authority is supported");
        assert!(
            String::from_utf8(refund_funder)
                .expect("challenge is UTF-8")
                .contains("role=refund_funder")
        );
        let error = authority_challenge_message(id(20), id(21), "", "proposer", 10)
            .expect_err("empty subject");
        assert_eq!(error, AdjustmentError::InvalidSubject("authority subject"));
    }

    #[test]
    fn refund_is_conserving_and_digest_is_stable() {
        let adjustment = Adjustment::propose(proposal(AdjustmentKind::Refund, refund_delta(250)))
            .expect("valid refund");
        assert_eq!(adjustment.status(), AdjustmentStatus::Proposed);
        assert_eq!(
            adjustment.digest().expect("digest"),
            adjustment.digest().expect("digest")
        );
        assert!(adjustment.is_negative_adjustment());
    }

    #[test]
    fn rejects_nonconserving_and_wrong_sign_refunds() {
        let mut nonconserving = proposal(
            AdjustmentKind::Refund,
            AdjustmentDelta {
                gross_atomic: -100,
                tax_atomic: -20,
                net_atomic: -70,
            },
        );
        assert_eq!(
            Adjustment::propose(nonconserving.clone()).unwrap_err(),
            AdjustmentError::DeltaNotConserving
        );

        nonconserving.delta = AdjustmentDelta {
            gross_atomic: 100,
            tax_atomic: 0,
            net_atomic: 100,
        };
        assert_eq!(
            Adjustment::propose(nonconserving).unwrap_err(),
            AdjustmentError::KindRequiresNegativeDelta
        );
    }

    #[test]
    fn refund_balance_proof_requires_one_finalized_slot_and_exact_three_leg_deltas() {
        let proof = prove_refund_balance_deltas(
            AdjustmentDelta {
                gross_atomic: -100,
                tax_atomic: -25,
                net_atomic: -75,
            },
            RefundBalanceSnapshot {
                finalized_slot: 500,
                tax_vault_atomic: 25,
                refund_source_atomic: 75,
                payer_destination_atomic: 0,
            },
            RefundBalanceSnapshot {
                finalized_slot: 500,
                tax_vault_atomic: 0,
                refund_source_atomic: 0,
                payer_destination_atomic: 100,
            },
        )
        .expect("exact finalized refund proof");
        assert_eq!(proof.finalized_slot, 500);
        assert_eq!(proof.gross_atomic, 100);
        assert_eq!(proof.tax_atomic, 25);
        assert_eq!(proof.net_atomic, 75);
        assert_ne!(proof.digest().expect("proof digest"), [0; 32]);
        let mut changed = proof;
        changed.payer_destination_after_atomic = 101;
        assert_eq!(
            changed.digest().unwrap_err(),
            AdjustmentError::PayerDestinationBalanceDeltaMismatch
        );
    }

    #[test]
    fn refund_balance_proof_rejects_stale_or_mismatched_snapshots() {
        let before = RefundBalanceSnapshot {
            finalized_slot: 500,
            tax_vault_atomic: 25,
            refund_source_atomic: 75,
            payer_destination_atomic: 0,
        };
        let mut after = RefundBalanceSnapshot {
            finalized_slot: 501,
            tax_vault_atomic: 0,
            refund_source_atomic: 0,
            payer_destination_atomic: 100,
        };
        assert_eq!(
            prove_refund_balance_deltas(
                AdjustmentDelta {
                    gross_atomic: -100,
                    tax_atomic: -25,
                    net_atomic: -75,
                },
                before,
                after,
            )
            .unwrap_err(),
            AdjustmentError::RefundBalanceSlotMismatch
        );
        after.finalized_slot = 500;
        after.payer_destination_atomic = 99;
        assert_eq!(
            prove_refund_balance_deltas(
                AdjustmentDelta {
                    gross_atomic: -100,
                    tax_atomic: -25,
                    net_atomic: -75,
                },
                before,
                after,
            )
            .unwrap_err(),
            AdjustmentError::PayerDestinationBalanceDeltaMismatch
        );
    }

    #[test]
    fn refund_balance_proof_rejects_positive_deltas_and_underflow() {
        let snapshot = RefundBalanceSnapshot {
            finalized_slot: 500,
            tax_vault_atomic: 1,
            refund_source_atomic: 1,
            payer_destination_atomic: 0,
        };
        assert_eq!(
            prove_refund_balance_deltas(
                AdjustmentDelta {
                    gross_atomic: 100,
                    tax_atomic: 25,
                    net_atomic: 75,
                },
                snapshot,
                snapshot,
            )
            .unwrap_err(),
            AdjustmentError::RefundBalanceDeltaMustBeNegative
        );
        assert_eq!(
            prove_refund_balance_deltas(
                AdjustmentDelta {
                    gross_atomic: -100,
                    tax_atomic: -25,
                    net_atomic: -75,
                },
                snapshot,
                RefundBalanceSnapshot {
                    finalized_slot: 500,
                    tax_vault_atomic: 0,
                    refund_source_atomic: 0,
                    payer_destination_atomic: 100,
                },
            )
            .unwrap_err(),
            AdjustmentError::TaxVaultBalanceUnderflow
        );
    }

    #[test]
    fn approval_requires_a_distinct_subject_and_posting_is_monotonic() {
        let mut adjustment =
            Adjustment::propose(proposal(AdjustmentKind::Reversal, refund_delta(250)))
                .expect("valid reversal");
        assert_eq!(
            adjustment.approve("proposer-subject").unwrap_err(),
            AdjustmentError::ApprovalSubjectMustDiffer
        );
        adjustment.approve("approver-subject").expect("approval");
        assert_eq!(adjustment.status(), AdjustmentStatus::Approved);
        assert_eq!(
            adjustment.approve("another-subject").unwrap_err(),
            AdjustmentError::InvalidTransition
        );
    }

    #[test]
    fn ledger_bounds_negative_deltas_and_is_idempotent() {
        let mut first = Adjustment::propose(proposal(AdjustmentKind::Refund, refund_delta(600)))
            .expect("first");
        first.approve("approver-first").expect("approval");
        let mut second_input = proposal(AdjustmentKind::Refund, refund_delta(400));
        second_input.id = id(9);
        let mut second = Adjustment::propose(second_input).expect("second");
        second.approve("approver-second").expect("approval");
        let mut ledger = AdjustmentLedger::new(OriginalSettlementAmounts {
            gross_atomic: 1_000,
            tax_atomic: 0,
            net_atomic: 1_000,
        });
        assert_eq!(ledger.post(&mut first).expect("post"), PostOutcome::Posted);
        assert_eq!(
            ledger.post(&mut first).expect("replay"),
            PostOutcome::Replay
        );
        assert_eq!(ledger.post(&mut second).expect("post"), PostOutcome::Posted);
        assert_eq!(
            ledger.effective_amounts().expect("effective"),
            OriginalSettlementAmounts {
                gross_atomic: 0,
                tax_atomic: 0,
                net_atomic: 0,
            }
        );

        let mut over_refund_input = proposal(AdjustmentKind::Refund, refund_delta(1));
        over_refund_input.id = id(11);
        let mut over_refund = Adjustment::propose(over_refund_input).expect("over refund");
        over_refund.approve("approver-over").expect("approval");
        assert_eq!(
            ledger.post(&mut over_refund).unwrap_err(),
            AdjustmentError::AdjustmentExceedsOriginal
        );
    }

    #[test]
    fn same_id_with_different_digest_is_a_conflict() {
        let mut first =
            Adjustment::propose(proposal(AdjustmentKind::Refund, refund_delta(1))).expect("first");
        first.approve("approver-first").expect("approval");
        let mut ledger = AdjustmentLedger::new(OriginalSettlementAmounts {
            gross_atomic: 2,
            tax_atomic: 0,
            net_atomic: 2,
        });
        ledger.post(&mut first).expect("post");

        let mut changed_input = proposal(AdjustmentKind::Refund, refund_delta(2));
        changed_input.id = first.id();
        let mut changed = Adjustment::propose(changed_input).expect("changed");
        changed.approve("approver-first").expect("approval");
        assert_eq!(
            ledger.post(&mut changed).unwrap_err(),
            AdjustmentError::IdempotencyConflict
        );
    }

    #[test]
    fn content_digest_excludes_server_generated_record_id_for_replay() {
        let first = Adjustment::propose(proposal(AdjustmentKind::Refund, refund_delta(100)))
            .expect("first");
        let mut retry_input = proposal(AdjustmentKind::Refund, refund_delta(100));
        retry_input.id = id(9);
        let retry = Adjustment::propose(retry_input).expect("retry");

        assert_eq!(first.digest().unwrap(), retry.digest().unwrap());
    }

    proptest! {
        #[test]
        fn generated_positive_corrections_preserve_conservation(
            tax in 0i128..=u64::MAX as i128 / 2,
            net in 0i128..=u64::MAX as i128 / 2,
        ) {
            let mut input = proposal(
                AdjustmentKind::Correction,
                AdjustmentDelta {
                    gross_atomic: tax + net,
                    tax_atomic: tax,
                    net_atomic: net,
                },
            );
            input.id = id(13);
            prop_assert!(Adjustment::propose(input).is_ok());
        }
    }
}
