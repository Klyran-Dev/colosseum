#![cfg_attr(
    not(test),
    deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)
)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sha2::{Digest as ShaDigest, Sha256};
use thiserror::Error;
use uuid::Uuid;

pub const STREAMING_SCHEMA: &str = "revenue-streaming/v1";

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct Digest(pub [u8; 32]);

impl Digest {
    fn is_zero(self) -> bool {
        self.0.iter().all(|byte| *byte == 0)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Signature(pub Vec<u8>);

impl Signature {
    fn is_zero(&self) -> bool {
        self.0.len() != 64 || self.0.iter().all(|byte| *byte == 0)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct AssetId(pub [u8; 32]);

impl AssetId {
    fn is_zero(self) -> bool {
        self.0.iter().all(|byte| *byte == 0)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct RecipientId(pub [u8; 32]);

impl RecipientId {
    fn is_zero(self) -> bool {
        self.0.iter().all(|byte| *byte == 0)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct AuthorityId(pub [u8; 32]);

impl AuthorityId {
    fn is_zero(self) -> bool {
        self.0.iter().all(|byte| *byte == 0)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct StreamPlanId(pub Uuid);

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum StreamStatus {
    Draft,
    Proposed,
    Approved,
    Active,
    Paused,
    Cancelled,
    Completed,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StreamPlanSpec {
    pub id: StreamPlanId,
    pub merchant_id: Uuid,
    pub settlement_route_id: Uuid,
    pub operator_subject: Option<String>,
    pub asset: AssetId,
    pub recipient: RecipientId,
    pub plan_digest: Digest,
    pub policy_digest: Digest,
    pub start_slot: u64,
    pub end_slot: Option<u64>,
    pub max_total_atomic: u64,
    pub max_claim_atomic: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StreamPlan {
    spec: StreamPlanSpec,
    status: StreamStatus,
    proposer: Option<AuthorityId>,
    approver: Option<AuthorityId>,
}

impl StreamPlan {
    pub fn new(spec: StreamPlanSpec) -> Result<Self, StreamingError> {
        if !is_uuid_v7(spec.id.0)
            || spec.merchant_id.is_nil()
            || !is_uuid_v7(spec.settlement_route_id)
        {
            return Err(StreamingError::InvalidIdentifier);
        }
        if spec.operator_subject.as_deref().is_some_and(|subject| {
            subject.trim().is_empty()
                || subject.len() > 256
                || subject.bytes().any(|byte| byte.is_ascii_control())
        }) {
            return Err(StreamingError::InvalidIdentifier);
        }
        if spec.asset.is_zero()
            || spec.recipient.is_zero()
            || spec.plan_digest.is_zero()
            || spec.policy_digest.is_zero()
        {
            return Err(StreamingError::InvalidIdentifier);
        }
        if spec.start_slot == 0
            || spec
                .end_slot
                .is_some_and(|end_slot| end_slot < spec.start_slot)
        {
            return Err(StreamingError::InvalidWindow);
        }
        if spec.max_total_atomic == 0
            || spec.max_claim_atomic == 0
            || spec.max_claim_atomic > spec.max_total_atomic
        {
            return Err(StreamingError::InvalidAmount);
        }
        Ok(Self {
            spec,
            status: StreamStatus::Draft,
            proposer: None,
            approver: None,
        })
    }

    pub fn id(&self) -> StreamPlanId {
        self.spec.id
    }

    pub fn digest(&self) -> Digest {
        self.spec.plan_digest
    }

    pub fn policy_digest(&self) -> Digest {
        self.spec.policy_digest
    }

    pub fn asset(&self) -> AssetId {
        self.spec.asset
    }

    pub fn recipient(&self) -> RecipientId {
        self.spec.recipient
    }

    pub fn status(&self) -> StreamStatus {
        self.status
    }

    fn slot_is_valid(&self, slot: u64) -> bool {
        slot >= self.spec.start_slot && self.spec.end_slot.is_none_or(|end_slot| slot <= end_slot)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FinalizedAccrual {
    pub source_signature: Signature,
    pub finalized_slot: u64,
    pub receipt_digest: Digest,
    pub plan_digest: Digest,
    pub policy_digest: Digest,
    pub asset: AssetId,
    pub recipient: RecipientId,
    pub gross_atomic: u64,
    pub tax_atomic: u64,
    pub net_atomic: u64,
    pub recipient_atomic: u64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FinalityCommitment {
    Processed,
    Confirmed,
    Finalized,
}

/// Server-owned evidence produced by a finalized settlement indexer. This is
/// intentionally a value contract rather than an RPC or database adapter so
/// the revenue domain remains deterministic and replayable.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FinalizedSettlementAllocationProof {
    pub source_signature: Signature,
    pub source_slot: u64,
    pub observed_finalized_slot: u64,
    pub commitment: FinalityCommitment,
    pub receipt_digest: Digest,
    pub plan_digest: Digest,
    pub policy_digest: Digest,
    pub asset: AssetId,
    pub recipient: RecipientId,
    pub gross_atomic: u64,
    pub tax_atomic: u64,
    pub net_atomic: u64,
    pub recipient_atomic: u64,
}

/// Converts only an explicitly finalized, slot-observed allocation into the
/// domain's immutable accrual value. No caller-provided value can substitute
/// for the finality marker or observation bound.
pub fn finalized_accrual_from_proof(
    proof: FinalizedSettlementAllocationProof,
) -> Result<FinalizedAccrual, StreamingError> {
    if proof.commitment != FinalityCommitment::Finalized {
        return Err(StreamingError::NotFinalized);
    }
    if proof.source_slot == 0 || proof.observed_finalized_slot < proof.source_slot {
        return Err(StreamingError::StaleFinalityObservation);
    }
    if proof.source_signature.is_zero()
        || proof.receipt_digest.is_zero()
        || proof.plan_digest.is_zero()
        || proof.policy_digest.is_zero()
        || proof.asset.is_zero()
        || proof.recipient.is_zero()
        || proof.gross_atomic == 0
        || proof.recipient_atomic == 0
    {
        return Err(StreamingError::InvalidIdentifier);
    }
    let conserved_gross = proof
        .tax_atomic
        .checked_add(proof.net_atomic)
        .ok_or(StreamingError::ArithmeticOverflow)?;
    if conserved_gross != proof.gross_atomic || proof.recipient_atomic > proof.net_atomic {
        return Err(StreamingError::NonConservingAmounts);
    }
    Ok(FinalizedAccrual {
        source_signature: proof.source_signature,
        finalized_slot: proof.source_slot,
        receipt_digest: proof.receipt_digest,
        plan_digest: proof.plan_digest,
        policy_digest: proof.policy_digest,
        asset: proof.asset,
        recipient: proof.recipient,
        gross_atomic: proof.gross_atomic,
        tax_atomic: proof.tax_atomic,
        net_atomic: proof.net_atomic,
        recipient_atomic: proof.recipient_atomic,
    })
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FinalizedCorrection {
    pub correction_signature: Signature,
    pub source_signature: Signature,
    pub finalized_slot: u64,
    pub plan_digest: Digest,
    pub decrease: bool,
    pub amount_atomic: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PreparedClaim {
    pub claim_id: Uuid,
    pub stream_plan_id: StreamPlanId,
    pub amount_atomic: u64,
    pub nonce: u64,
    pub accrual_total_atomic: u64,
    pub plan_digest: Digest,
    pub asset: AssetId,
    pub recipient: RecipientId,
    pub claim_digest: Digest,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FinalizedRelease {
    pub claim_id: Uuid,
    pub release_signature: Signature,
    pub finalized_slot: u64,
    pub claim_digest: Digest,
    pub plan_digest: Digest,
    pub asset: AssetId,
    pub recipient: RecipientId,
    pub amount_atomic: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AccrualOutcome {
    Recorded,
    Duplicate,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReleaseOutcome {
    Recorded,
    Duplicate,
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum StreamingError {
    #[error("stream is in an invalid state")]
    InvalidState,
    #[error("distinct proposer and approver are required")]
    FourEyesRequired,
    #[error("stream identifier or required binding is invalid")]
    InvalidIdentifier,
    #[error("stream window is invalid")]
    InvalidWindow,
    #[error("amount or cap is invalid")]
    InvalidAmount,
    #[error("amounts do not conserve")]
    NonConservingAmounts,
    #[error("source settlement is not finalized")]
    NotFinalized,
    #[error("finalized observation does not cover the source slot")]
    StaleFinalityObservation,
    #[error("server binding does not match the stream plan")]
    BindingMismatch,
    #[error("finalized evidence is outside the stream window")]
    OutsideWindow,
    #[error("evidence conflicts with an existing record")]
    ConflictingEvidence,
    #[error("requested amount exceeds the available balance")]
    InsufficientAvailable,
    #[error("claim was not found")]
    UnknownClaim,
    #[error("claim evidence does not match the prepared claim")]
    ClaimBindingMismatch,
    #[error("claim identifier is invalid or already exists")]
    DuplicateClaim,
    #[error("arithmetic limit was exceeded")]
    ArithmeticOverflow,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
enum ClaimState {
    Pending,
    Finalized(Signature),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct ClaimRecord {
    claim: PreparedClaim,
    state: ClaimState,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StreamLedger {
    plan: StreamPlan,
    accruals: BTreeMap<Signature, FinalizedAccrual>,
    corrections: BTreeMap<Signature, FinalizedCorrection>,
    claims: BTreeMap<Uuid, ClaimRecord>,
    releases: BTreeMap<Signature, FinalizedRelease>,
    accrued_total_atomic: u64,
    released_total_atomic: u64,
    next_nonce: u64,
}

impl StreamLedger {
    pub fn new(plan: StreamPlan) -> Self {
        Self {
            plan,
            accruals: BTreeMap::new(),
            corrections: BTreeMap::new(),
            claims: BTreeMap::new(),
            releases: BTreeMap::new(),
            accrued_total_atomic: 0,
            released_total_atomic: 0,
            next_nonce: 1,
        }
    }

    pub fn status(&self) -> StreamStatus {
        self.plan.status
    }

    pub fn propose(&mut self, proposer: AuthorityId) -> Result<(), StreamingError> {
        if self.plan.status != StreamStatus::Draft || proposer.is_zero() {
            return Err(StreamingError::InvalidState);
        }
        self.plan.proposer = Some(proposer);
        self.plan.status = StreamStatus::Proposed;
        Ok(())
    }

    pub fn approve(&mut self, approver: AuthorityId) -> Result<(), StreamingError> {
        if self.plan.status != StreamStatus::Proposed || approver.is_zero() {
            return Err(StreamingError::InvalidState);
        }
        if self.plan.proposer == Some(approver) {
            return Err(StreamingError::FourEyesRequired);
        }
        self.plan.approver = Some(approver);
        self.plan.status = StreamStatus::Approved;
        Ok(())
    }

    pub fn activate(&mut self, actor: AuthorityId) -> Result<(), StreamingError> {
        if self.plan.status != StreamStatus::Approved || self.plan.approver != Some(actor) {
            return Err(StreamingError::InvalidState);
        }
        self.plan.status = StreamStatus::Active;
        Ok(())
    }

    pub fn pause(&mut self) -> Result<(), StreamingError> {
        if self.plan.status != StreamStatus::Active {
            return Err(StreamingError::InvalidState);
        }
        self.plan.status = StreamStatus::Paused;
        Ok(())
    }

    pub fn resume(&mut self) -> Result<(), StreamingError> {
        if self.plan.status != StreamStatus::Paused {
            return Err(StreamingError::InvalidState);
        }
        self.plan.status = StreamStatus::Active;
        Ok(())
    }

    pub fn cancel(&mut self, actor: AuthorityId) -> Result<(), StreamingError> {
        if !matches!(
            self.plan.status,
            StreamStatus::Approved | StreamStatus::Active | StreamStatus::Paused
        ) || self.plan.approver != Some(actor)
        {
            return Err(StreamingError::InvalidState);
        }
        self.plan.status = StreamStatus::Cancelled;
        Ok(())
    }

    pub fn complete(&mut self, observed_slot: u64) -> Result<(), StreamingError> {
        if !matches!(
            self.plan.status,
            StreamStatus::Active | StreamStatus::Paused
        ) || self
            .plan
            .spec
            .end_slot
            .is_none_or(|end_slot| observed_slot < end_slot)
        {
            return Err(StreamingError::InvalidState);
        }
        if self.available_atomic()? != 0 || self.pending_claims_atomic()? != 0 {
            return Err(StreamingError::InsufficientAvailable);
        }
        self.plan.status = StreamStatus::Completed;
        Ok(())
    }

    pub fn record_accrual(
        &mut self,
        accrual: FinalizedAccrual,
    ) -> Result<AccrualOutcome, StreamingError> {
        if self.plan.status != StreamStatus::Active {
            return Err(StreamingError::InvalidState);
        }
        if accrual.source_signature.is_zero()
            || accrual.receipt_digest.is_zero()
            || accrual.finalized_slot == 0
        {
            return Err(StreamingError::InvalidIdentifier);
        }
        if !self.plan.slot_is_valid(accrual.finalized_slot) {
            return Err(StreamingError::OutsideWindow);
        }
        if accrual.plan_digest != self.plan.digest()
            || accrual.policy_digest != self.plan.policy_digest()
            || accrual.asset != self.plan.asset()
            || accrual.recipient != self.plan.recipient()
        {
            return Err(StreamingError::BindingMismatch);
        }
        let expected_net = accrual
            .tax_atomic
            .checked_add(accrual.net_atomic)
            .ok_or(StreamingError::ArithmeticOverflow)?;
        if expected_net != accrual.gross_atomic
            || accrual.recipient_atomic > accrual.net_atomic
            || accrual.recipient_atomic == 0
        {
            return Err(StreamingError::NonConservingAmounts);
        }
        if let Some(existing) = self.accruals.get(&accrual.source_signature) {
            return if existing == &accrual {
                Ok(AccrualOutcome::Duplicate)
            } else {
                Err(StreamingError::ConflictingEvidence)
            };
        }
        let next_total = self
            .accrued_total_atomic
            .checked_add(accrual.recipient_atomic)
            .ok_or(StreamingError::ArithmeticOverflow)?;
        if next_total > self.plan.spec.max_total_atomic {
            return Err(StreamingError::InsufficientAvailable);
        }
        self.accrued_total_atomic = next_total;
        self.accruals
            .insert(accrual.source_signature.clone(), accrual);
        Ok(AccrualOutcome::Recorded)
    }

    pub fn available_atomic(&self) -> Result<u64, StreamingError> {
        let pending = self.pending_claims_atomic()?;
        self.accrued_total_atomic
            .checked_sub(self.released_total_atomic)
            .and_then(|available| available.checked_sub(pending))
            .ok_or(StreamingError::ArithmeticOverflow)
    }

    pub fn prepare_claim(
        &mut self,
        plan_id: StreamPlanId,
        claim_id: Uuid,
        amount_atomic: u64,
    ) -> Result<PreparedClaim, StreamingError> {
        if self.plan.status != StreamStatus::Active || plan_id != self.plan.id() {
            return Err(StreamingError::InvalidState);
        }
        if !is_uuid_v7(claim_id)
            || claim_id.is_nil()
            || amount_atomic == 0
            || amount_atomic > self.plan.spec.max_claim_atomic
        {
            return Err(StreamingError::InvalidIdentifier);
        }
        if self.claims.contains_key(&claim_id) {
            return Err(StreamingError::DuplicateClaim);
        }
        if amount_atomic > self.available_atomic()? {
            return Err(StreamingError::InsufficientAvailable);
        }
        let nonce = self.next_nonce;
        self.next_nonce = self
            .next_nonce
            .checked_add(1)
            .ok_or(StreamingError::ArithmeticOverflow)?;
        let claim = PreparedClaim {
            claim_id,
            stream_plan_id: self.plan.id(),
            amount_atomic,
            nonce,
            accrual_total_atomic: self.accrued_total_atomic,
            plan_digest: self.plan.digest(),
            asset: self.plan.asset(),
            recipient: self.plan.recipient(),
            claim_digest: claim_digest(
                claim_id,
                self.plan.id(),
                amount_atomic,
                nonce,
                self.plan.digest(),
                self.plan.asset(),
                self.plan.recipient(),
            ),
        };
        self.claims.insert(
            claim_id,
            ClaimRecord {
                claim: claim.clone(),
                state: ClaimState::Pending,
            },
        );
        Ok(claim)
    }

    pub fn finalize_claim(
        &mut self,
        release: FinalizedRelease,
    ) -> Result<ReleaseOutcome, StreamingError> {
        if release.release_signature.is_zero() || release.finalized_slot < self.plan.spec.start_slot
        {
            return Err(StreamingError::InvalidIdentifier);
        }
        if let Some(existing) = self.releases.get(&release.release_signature) {
            return if existing == &release {
                Ok(ReleaseOutcome::Duplicate)
            } else {
                Err(StreamingError::ConflictingEvidence)
            };
        }
        let record = self
            .claims
            .get(&release.claim_id)
            .ok_or(StreamingError::UnknownClaim)?;
        let claim = record.claim.clone();
        if let ClaimState::Finalized(signature) = &record.state {
            return if signature == &release.release_signature {
                Ok(ReleaseOutcome::Duplicate)
            } else {
                Err(StreamingError::ConflictingEvidence)
            };
        }
        if release.claim_digest != claim.claim_digest
            || release.plan_digest != claim.plan_digest
            || release.asset != claim.asset
            || release.recipient != claim.recipient
            || release.amount_atomic != claim.amount_atomic
        {
            return Err(StreamingError::ClaimBindingMismatch);
        }
        self.released_total_atomic = self
            .released_total_atomic
            .checked_add(release.amount_atomic)
            .ok_or(StreamingError::ArithmeticOverflow)?;
        self.claims
            .get_mut(&release.claim_id)
            .ok_or(StreamingError::UnknownClaim)?
            .state = ClaimState::Finalized(release.release_signature.clone());
        self.releases
            .insert(release.release_signature.clone(), release);
        Ok(ReleaseOutcome::Recorded)
    }

    pub fn apply_correction(
        &mut self,
        correction: FinalizedCorrection,
    ) -> Result<(), StreamingError> {
        if !matches!(
            self.plan.status,
            StreamStatus::Active | StreamStatus::Paused
        ) {
            return Err(StreamingError::InvalidState);
        }
        if correction.correction_signature.is_zero()
            || correction.source_signature.is_zero()
            || correction.finalized_slot == 0
            || correction.amount_atomic == 0
            || correction.plan_digest != self.plan.digest()
            || !self.plan.slot_is_valid(correction.finalized_slot)
        {
            return Err(StreamingError::BindingMismatch);
        }
        if !self.accruals.contains_key(&correction.source_signature) {
            return Err(StreamingError::UnknownClaim);
        }
        if let Some(existing) = self.corrections.get(&correction.correction_signature) {
            return if existing == &correction {
                Ok(())
            } else {
                Err(StreamingError::ConflictingEvidence)
            };
        }
        if correction.decrease {
            if correction.amount_atomic > self.available_atomic()? {
                return Err(StreamingError::InsufficientAvailable);
            }
            self.accrued_total_atomic = self
                .accrued_total_atomic
                .checked_sub(correction.amount_atomic)
                .ok_or(StreamingError::ArithmeticOverflow)?;
        } else {
            let next_total = self
                .accrued_total_atomic
                .checked_add(correction.amount_atomic)
                .ok_or(StreamingError::ArithmeticOverflow)?;
            if next_total > self.plan.spec.max_total_atomic {
                return Err(StreamingError::InsufficientAvailable);
            }
            self.accrued_total_atomic = next_total;
        }
        self.corrections
            .insert(correction.correction_signature.clone(), correction);
        Ok(())
    }

    pub fn released_atomic(&self) -> u64 {
        self.released_total_atomic
    }

    fn pending_claims_atomic(&self) -> Result<u64, StreamingError> {
        self.claims
            .values()
            .try_fold(0_u64, |total, record| match record.state {
                ClaimState::Pending => total
                    .checked_add(record.claim.amount_atomic)
                    .ok_or(StreamingError::ArithmeticOverflow),
                ClaimState::Finalized(_) => Ok(total),
            })
    }
}

fn is_uuid_v7(value: Uuid) -> bool {
    let bytes = value.as_bytes();
    (bytes[6] >> 4) == 7 && (bytes[8] & 0xc0) == 0x80
}

pub fn claim_digest(
    claim_id: Uuid,
    plan_id: StreamPlanId,
    amount_atomic: u64,
    nonce: u64,
    plan_digest: Digest,
    asset: AssetId,
    recipient: RecipientId,
) -> Digest {
    let mut hasher = Sha256::new();
    hasher.update(b"solana-tax-protocol/revenue-streaming-claim/v1");
    hasher.update(claim_id.as_bytes());
    hasher.update(plan_id.0.as_bytes());
    hasher.update(amount_atomic.to_be_bytes());
    hasher.update(nonce.to_be_bytes());
    hasher.update(plan_digest.0);
    hasher.update(asset.0);
    hasher.update(recipient.0);
    Digest(hasher.finalize().into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uuid(seed: u8) -> Uuid {
        let mut bytes = [0_u8; 16];
        bytes[0] = seed;
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        Uuid::from_bytes(bytes)
    }

    fn digest(seed: u8) -> Digest {
        Digest([seed; 32])
    }

    fn key(seed: u8) -> [u8; 32] {
        [seed; 32]
    }

    fn signature(seed: u8) -> Signature {
        Signature(vec![seed; 64])
    }

    fn spec() -> StreamPlanSpec {
        StreamPlanSpec {
            id: StreamPlanId(uuid(1)),
            merchant_id: uuid(2),
            settlement_route_id: uuid(3),
            operator_subject: Some("stream-operator".to_string()),
            asset: AssetId(key(3)),
            recipient: RecipientId(key(4)),
            plan_digest: digest(5),
            policy_digest: digest(6),
            start_slot: 100,
            end_slot: Some(1_000),
            max_total_atomic: 1_000,
            max_claim_atomic: 600,
        }
    }

    fn active_ledger() -> StreamLedger {
        let mut ledger = StreamLedger::new(StreamPlan::new(spec()).expect("valid fixture"));
        let proposer = AuthorityId(key(7));
        let approver = AuthorityId(key(8));
        ledger.propose(proposer).expect("proposal");
        ledger.approve(approver).expect("approval");
        ledger.activate(approver).expect("activation");
        ledger
    }

    fn accrual(source: u8, amount: u64) -> FinalizedAccrual {
        FinalizedAccrual {
            source_signature: signature(source),
            finalized_slot: 200,
            receipt_digest: digest(source + 20),
            plan_digest: digest(5),
            policy_digest: digest(6),
            asset: AssetId(key(3)),
            recipient: RecipientId(key(4)),
            gross_atomic: amount + 100,
            tax_atomic: 100,
            net_atomic: amount,
            recipient_atomic: amount,
        }
    }

    #[test]
    fn lifecycle_requires_distinct_proposer_and_approver() {
        let mut ledger = StreamLedger::new(StreamPlan::new(spec()).expect("valid fixture"));
        let authority = AuthorityId(key(7));
        assert_eq!(
            ledger.activate(authority),
            Err(StreamingError::InvalidState)
        );
        ledger.propose(authority).expect("proposal");
        assert_eq!(
            ledger.approve(authority),
            Err(StreamingError::FourEyesRequired)
        );
        ledger.approve(AuthorityId(key(8))).expect("approval");
        ledger.activate(AuthorityId(key(8))).expect("activation");
        assert_eq!(ledger.status(), StreamStatus::Active);
    }

    #[test]
    fn plan_rejects_invalid_windows_caps_and_non_v7_ids() {
        let mut invalid = spec();
        invalid.start_slot = 1_000;
        invalid.end_slot = Some(100);
        assert_eq!(StreamPlan::new(invalid), Err(StreamingError::InvalidWindow));

        let mut invalid = spec();
        invalid.max_claim_atomic = 0;
        assert_eq!(StreamPlan::new(invalid), Err(StreamingError::InvalidAmount));

        let mut invalid = spec();
        invalid.id = StreamPlanId(Uuid::nil());
        assert_eq!(
            StreamPlan::new(invalid),
            Err(StreamingError::InvalidIdentifier)
        );
    }

    #[test]
    fn accrual_requires_conservation_and_exact_server_bindings() {
        let mut ledger = active_ledger();
        let mut invalid = accrual(10, 400);
        invalid.gross_atomic = 499;
        assert_eq!(
            ledger.record_accrual(invalid),
            Err(StreamingError::NonConservingAmounts)
        );

        let mut invalid = accrual(11, 400);
        invalid.plan_digest = digest(99);
        assert_eq!(
            ledger.record_accrual(invalid),
            Err(StreamingError::BindingMismatch)
        );

        let mut invalid = accrual(12, 400);
        invalid.finalized_slot = 99;
        assert_eq!(
            ledger.record_accrual(invalid),
            Err(StreamingError::OutsideWindow)
        );
    }

    #[test]
    fn identical_accrual_replays_idempotently_but_conflict_fails_closed() {
        let mut ledger = active_ledger();
        let first = accrual(10, 400);
        assert_eq!(
            ledger.record_accrual(first.clone()),
            Ok(AccrualOutcome::Recorded)
        );
        assert_eq!(ledger.record_accrual(first), Ok(AccrualOutcome::Duplicate));

        let mut conflict = accrual(10, 401);
        conflict.gross_atomic = 501;
        assert_eq!(
            ledger.record_accrual(conflict),
            Err(StreamingError::ConflictingEvidence)
        );
    }

    #[test]
    fn claims_are_bounded_and_two_preparations_cannot_overdraw() {
        let mut ledger = active_ledger();
        ledger.record_accrual(accrual(10, 1_000)).expect("accrual");
        let first = ledger
            .prepare_claim(
                StreamPlanId(uuid(1)),
                Uuid::from_bytes({
                    let mut b = [0_u8; 16];
                    b[0] = 9;
                    b[6] = 0x70;
                    b[8] = 0x80;
                    b
                }),
                600,
            )
            .expect("claim");
        assert_eq!(first.amount_atomic, 600);
        assert_eq!(
            ledger.prepare_claim(StreamPlanId(uuid(1)), uuid(10), 401),
            Err(StreamingError::InsufficientAvailable)
        );
        assert_eq!(ledger.available_atomic(), Ok(400));
    }

    #[test]
    fn bounded_amounts_preserve_conservation_for_many_inputs() {
        for amount in 1..=100 {
            let mut ledger = active_ledger();
            ledger
                .record_accrual(accrual(amount as u8, amount))
                .expect("accrual");
            assert_eq!(ledger.available_atomic(), Ok(amount));
            let claim_id = uuid(amount as u8).into_bytes();
            let mut claim_uuid = claim_id;
            claim_uuid[6] = 0x70;
            claim_uuid[8] = 0x80;
            let claim = ledger
                .prepare_claim(StreamPlanId(uuid(1)), Uuid::from_bytes(claim_uuid), amount)
                .expect("claim");
            assert_eq!(claim.amount_atomic, amount);
            assert_eq!(ledger.available_atomic(), Ok(0));
        }
    }

    #[test]
    fn finalization_requires_exact_claim_binding_and_is_single_use() {
        let mut ledger = active_ledger();
        ledger.record_accrual(accrual(10, 400)).expect("accrual");
        let claim_id = uuid(9);
        let claim = ledger
            .prepare_claim(StreamPlanId(uuid(1)), claim_id, 400)
            .expect("claim");
        let release = FinalizedRelease {
            claim_id,
            release_signature: signature(30),
            finalized_slot: 300,
            claim_digest: claim.claim_digest,
            plan_digest: digest(5),
            asset: AssetId(key(3)),
            recipient: RecipientId(key(4)),
            amount_atomic: 400,
        };
        assert_eq!(
            ledger.finalize_claim(release.clone()),
            Ok(ReleaseOutcome::Recorded)
        );
        assert_eq!(
            ledger.finalize_claim(release),
            Ok(ReleaseOutcome::Duplicate)
        );
        assert_eq!(ledger.released_atomic(), 400);
        assert_eq!(ledger.available_atomic(), Ok(0));

        let mut wrong = FinalizedRelease {
            claim_id: uuid(11),
            ..release_for_test()
        };
        wrong.claim_digest = digest(99);
        assert_eq!(
            ledger.finalize_claim(wrong),
            Err(StreamingError::UnknownClaim)
        );
    }

    fn release_for_test() -> FinalizedRelease {
        FinalizedRelease {
            claim_id: uuid(12),
            release_signature: signature(31),
            finalized_slot: 300,
            claim_digest: digest(5),
            plan_digest: digest(5),
            asset: AssetId(key(3)),
            recipient: RecipientId(key(4)),
            amount_atomic: 1,
        }
    }

    #[test]
    fn correction_cannot_reduce_released_or_pending_balance() {
        let mut ledger = active_ledger();
        ledger.record_accrual(accrual(10, 500)).expect("accrual");
        let claim = ledger
            .prepare_claim(StreamPlanId(uuid(1)), uuid(9), 400)
            .expect("claim");
        let correction = FinalizedCorrection {
            correction_signature: signature(40),
            source_signature: signature(10),
            finalized_slot: 301,
            plan_digest: digest(5),
            decrease: true,
            amount_atomic: 101,
        };
        assert_eq!(
            ledger.apply_correction(correction),
            Err(StreamingError::InsufficientAvailable)
        );
        assert_eq!(ledger.available_atomic(), Ok(100));
        let correction = FinalizedCorrection {
            correction_signature: signature(41),
            source_signature: signature(10),
            finalized_slot: 301,
            plan_digest: digest(5),
            decrease: true,
            amount_atomic: 100,
        };
        ledger
            .apply_correction(correction)
            .expect("bounded correction");
        assert_eq!(ledger.available_atomic(), Ok(0));
        assert_eq!(claim.amount_atomic, 400);
    }

    #[test]
    fn cancellation_stops_new_claims_and_completion_requires_a_closed_window() {
        let mut ledger = active_ledger();
        ledger.record_accrual(accrual(10, 400)).expect("accrual");
        ledger.cancel(AuthorityId(key(8))).expect("cancellation");
        assert_eq!(
            ledger.prepare_claim(StreamPlanId(uuid(1)), uuid(9), 1),
            Err(StreamingError::InvalidState)
        );

        let mut ledger = active_ledger();
        ledger.record_accrual(accrual(10, 400)).expect("accrual");
        let claim = ledger
            .prepare_claim(StreamPlanId(uuid(1)), uuid(9), 400)
            .expect("claim");
        let release = FinalizedRelease {
            claim_id: claim.claim_id,
            release_signature: signature(50),
            finalized_slot: 1_001,
            claim_digest: claim.claim_digest,
            plan_digest: digest(5),
            asset: AssetId(key(3)),
            recipient: RecipientId(key(4)),
            amount_atomic: 400,
        };
        ledger.finalize_claim(release).expect("release");
        ledger.complete(1_000).expect("completion");
        assert_eq!(ledger.status(), StreamStatus::Completed);
    }

    fn finalized_allocation_proof() -> FinalizedSettlementAllocationProof {
        FinalizedSettlementAllocationProof {
            source_signature: signature(60),
            source_slot: 240,
            observed_finalized_slot: 240,
            commitment: FinalityCommitment::Finalized,
            receipt_digest: digest(61),
            plan_digest: digest(5),
            policy_digest: digest(6),
            asset: AssetId(key(3)),
            recipient: RecipientId(key(4)),
            gross_atomic: 1_100,
            tax_atomic: 100,
            net_atomic: 1_000,
            recipient_atomic: 800,
        }
    }

    #[test]
    fn finality_proof_adapter_emits_only_finalized_conserving_accruals() {
        let accrual = finalized_accrual_from_proof(finalized_allocation_proof())
            .expect("finalized allocation proof");
        assert_eq!(accrual.source_signature, signature(60));
        assert_eq!(accrual.finalized_slot, 240);
        assert_eq!(accrual.recipient_atomic, 800);

        let mut weak = finalized_allocation_proof();
        weak.commitment = FinalityCommitment::Confirmed;
        assert_eq!(
            finalized_accrual_from_proof(weak),
            Err(StreamingError::NotFinalized)
        );

        let mut stale = finalized_allocation_proof();
        stale.observed_finalized_slot = 239;
        assert_eq!(
            finalized_accrual_from_proof(stale),
            Err(StreamingError::StaleFinalityObservation)
        );

        let mut non_conserving = finalized_allocation_proof();
        non_conserving.net_atomic = 999;
        assert_eq!(
            finalized_accrual_from_proof(non_conserving),
            Err(StreamingError::NonConservingAmounts)
        );
    }
}
