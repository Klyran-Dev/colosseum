export type AdjustmentKind = "Refund" | "Reversal" | "Correction" | "Chargeback" | "RemittanceAdjustment";
export type AdjustmentStatus = "proposed" | "approved" | "rejected" | "voided";
export type AdjustmentTransitionStatus = "approved" | "rejected" | "voided";

export type AdjustmentDelta = {
  gross_atomic: string;
  tax_atomic: string;
  net_atomic: string;
};

export type SettlementAdjustment = {
  id: string;
  merchant_id: string;
  idempotency_key: string;
  request_digest: number[];
  adjustment_digest: number[];
  original_settlement_id: string;
  asset_id: number[];
  kind: AdjustmentKind;
  delta: AdjustmentDelta;
  reason_code: string;
  evidence_digest: number[];
  source_calculation_id: string;
  replacement_calculation_id: string | null;
  status: AdjustmentStatus;
  proposer_subject: string;
  approver_subject: string | null;
};

export type SettlementAdjustmentTransitionResponse = {
  status: "transitioned";
  adjustment_id: string;
  from_status: AdjustmentStatus;
  to_status: AdjustmentTransitionStatus;
  approver_subject: string | null;
};

export type CompiledSettlementAdjustmentAccount = {
  pubkey: string;
  is_signer: boolean;
  is_writable: boolean;
};

export type CompiledSettlementAdjustmentInstruction = {
  status: "prepared_unsubmitted";
  program_id: string;
  instruction_data_base64: string;
  accounts: CompiledSettlementAdjustmentAccount[];
  adjustment: string;
  adjustment_registry: string;
  original_receipt: string;
  adjustment_nonce: string;
};

export type CompiledSettlementAdjustmentArtifact = {
  status: "prepared_unsubmitted";
  intent_id: string;
  chain_intent_id: string;
  request_digest: number[];
  instruction_digest: number[];
  registry: string;
  registry_slot: string;
  registry_commitment: "finalized";
  registry_account_digest: number[];
  instruction: CompiledSettlementAdjustmentInstruction;
};

export type AdjustmentChainIntentStatus =
  | "prepared_unsubmitted"
  | "signed"
  | "submitting"
  | "submitted"
  | "processed"
  | "confirmed"
  | "finalized"
  | "rejected"
  | "unknown";

export type AdjustmentChainIntentResponse = {
  chain_intent_id: string;
  status: AdjustmentChainIntentStatus;
  signature: string | null;
  provider_index: number | null;
  finalized_evidence: FinalizedSettlementAdjustmentEvidence | null;
};

export type FinalizedSettlementAdjustmentEvidence = {
  event_type: "SettlementAdjustmentRecorded";
  event_version: "v1";
  event_discriminator: number[];
  commitment: "finalized";
  signature: string;
  slot: string;
  event_index: number;
  adjustment: number[];
  original_receipt: number[];
  merchant: number[];
  mint: number[];
  nonce: string;
  kind: number;
  gross_sign: number;
  gross_magnitude: string;
  tax_sign: number;
  tax_magnitude: string;
  net_sign: number;
  net_magnitude: string;
  adjustment_digest: number[];
  adjustment_evidence_digest: number[];
  event_evidence_digest: number[];
  approver: number[];
};

const UUID_V7 = /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
const DECIMAL = /^(?:0|-[1-9][0-9]*|[1-9][0-9]*)$/;
const UINT64_DECIMAL = /^(?:0|[1-9][0-9]*)$/;
const UINT64_MAX_DECIMAL = "18446744073709551615";
const MAX_ATOMIC = BigInt("18446744073709551615");
const NEGATIVE_KINDS: readonly AdjustmentKind[] = ["Refund", "Reversal", "Chargeback"];
const KINDS: readonly AdjustmentKind[] = ["Refund", "Reversal", "Correction", "Chargeback", "RemittanceAdjustment"];
const STATUSES: readonly AdjustmentStatus[] = ["proposed", "approved", "rejected", "voided"];

function objectValue(value: unknown, label: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) throw new Error(`${label} must be an object.`);
  return value as Record<string, unknown>;
}

function requiredString(value: unknown, label: string): string {
  if (typeof value !== "string" || value.length === 0) throw new Error(`${label} must be a non-empty string.`);
  return value;
}

export function parseCanonicalU64(value: unknown, label: string): string {
  if (typeof value !== "string"
      || !UINT64_DECIMAL.test(value)
      || value.length > UINT64_MAX_DECIMAL.length
      || (value.length === UINT64_MAX_DECIMAL.length && value > UINT64_MAX_DECIMAL)) {
    throw new Error(`${label} must be a canonical unsigned 64-bit decimal string.`);
  }
  return value;
}

function uuidV7(value: unknown, label: string): string {
  const candidate = requiredString(value, label);
  if (!UUID_V7.test(candidate)) throw new Error(`${label} must be a UUIDv7.`);
  return candidate;
}

export function validateUuidV7(value: string, label: string): string {
  return uuidV7(value, label);
}

export function validateByteArray(value: unknown, label: string, requireNonZero = true): number[] {
  if (!Array.isArray(value) || value.length !== 32 || value.some((byte) => !Number.isInteger(byte) || byte < 0 || byte > 255)) {
    throw new Error(`${label} must contain exactly 32 bytes.`);
  }
  const bytes = value as number[];
  if (requireNonZero && bytes.every((byte) => byte === 0)) throw new Error(`${label} must be non-zero.`);
  return [...bytes];
}

function decimal(value: unknown, label: string): bigint {
  if (typeof value !== "string" || !DECIMAL.test(value)) throw new Error(`${label} must be a canonical decimal string.`);
  const parsed = BigInt(value);
  if (parsed > MAX_ATOMIC || parsed < -MAX_ATOMIC) throw new Error(`${label} exceeds the atomic-unit bound.`);
  return parsed;
}

export function validateAdjustmentDelta(kind: string, delta: unknown): void {
  if (!KINDS.includes(kind as AdjustmentKind)) throw new Error("adjustment kind is unsupported.");
  const input = objectValue(delta, "adjustment delta");
  const gross = decimal(input.gross_atomic, "gross_atomic");
  const tax = decimal(input.tax_atomic, "tax_atomic");
  const net = decimal(input.net_atomic, "net_atomic");
  if (gross === 0n && tax === 0n && net === 0n) throw new Error("adjustment delta must be non-zero.");
  if (gross !== tax + net) throw new Error("adjustment delta must conserve gross = tax + net.");
  if (NEGATIVE_KINDS.includes(kind as AdjustmentKind) && (gross > 0n || tax > 0n || net > 0n)) {
    throw new Error(`${kind} deltas must be non-positive.`);
  }
}

function adjustmentKind(value: unknown): AdjustmentKind {
  if (typeof value !== "string" || !KINDS.includes(value as AdjustmentKind)) throw new Error("adjustment kind is unsupported.");
  return value as AdjustmentKind;
}

function adjustmentStatus(value: unknown): AdjustmentStatus {
  if (typeof value !== "string" || !STATUSES.includes(value as AdjustmentStatus)) throw new Error("adjustment status is unsupported.");
  return value as AdjustmentStatus;
}

function digest(value: unknown, label: string, requireNonZero = false): number[] {
  return validateByteArray(value, label, requireNonZero);
}

export function parseSettlementAdjustment(value: unknown): SettlementAdjustment {
  const input = objectValue(value, "settlement adjustment response");
  const kind = adjustmentKind(input.kind);
  const deltaInput = objectValue(input.delta, "adjustment delta");
  validateAdjustmentDelta(kind, deltaInput);
  const status = adjustmentStatus(input.status);
  const replacement = input.replacement_calculation_id === null || input.replacement_calculation_id === undefined
    ? null
    : uuidV7(input.replacement_calculation_id, "replacement_calculation_id");
  const reason = requiredString(input.reason_code, "reason_code");
  if (reason.length > 256) throw new Error("reason_code exceeds 256 characters.");
  const proposer = requiredString(input.proposer_subject, "proposer_subject");
  const approver = input.approver_subject === null || input.approver_subject === undefined
    ? null
    : requiredString(input.approver_subject, "approver_subject");
  if (approver === proposer) throw new Error("approver_subject must differ from proposer_subject.");
  return {
    id: uuidV7(input.id, "adjustment ID"),
    merchant_id: uuidV7(input.merchant_id, "merchant_id"),
    idempotency_key: requiredString(input.idempotency_key, "idempotency_key"),
    request_digest: digest(input.request_digest, "request_digest"),
    adjustment_digest: digest(input.adjustment_digest, "adjustment_digest"),
    original_settlement_id: uuidV7(input.original_settlement_id, "original_settlement_id"),
    asset_id: digest(input.asset_id, "asset_id", true),
    kind,
    delta: {
      gross_atomic: requiredString(deltaInput.gross_atomic, "gross_atomic"),
      tax_atomic: requiredString(deltaInput.tax_atomic, "tax_atomic"),
      net_atomic: requiredString(deltaInput.net_atomic, "net_atomic"),
    },
    reason_code: reason,
    evidence_digest: digest(input.evidence_digest, "evidence_digest", true),
    source_calculation_id: uuidV7(input.source_calculation_id, "source_calculation_id"),
    replacement_calculation_id: replacement,
    status,
    proposer_subject: proposer,
    approver_subject: approver,
  };
}

export function parseSettlementAdjustmentTransition(value: unknown): SettlementAdjustmentTransitionResponse {
  const input = objectValue(value, "settlement adjustment transition response");
  if (input.status !== "transitioned") throw new Error("transition response status is invalid.");
  const fromStatus = adjustmentStatus(input.from_status);
  const toStatus = input.to_status;
  if (toStatus !== "approved" && toStatus !== "rejected" && toStatus !== "voided") throw new Error("transition target is unsupported.");
  return {
    status: "transitioned",
    adjustment_id: uuidV7(input.adjustment_id, "adjustment_id"),
    from_status: fromStatus,
    to_status: toStatus,
    approver_subject: input.approver_subject === null || input.approver_subject === undefined ? null : requiredString(input.approver_subject, "approver_subject"),
  };
}

export function nextAdjustmentStatuses(status: string): AdjustmentTransitionStatus[] {
  switch (status) {
    case "proposed": return ["approved", "rejected"];
    case "approved": return ["voided"];
    default: return [];
  }
}

export function buildSettlementAdjustmentUrl(apiBase: string, adjustmentId: string): string {
  if (!UUID_V7.test(adjustmentId)) throw new Error("adjustment ID must be UUIDv7.");
  return `${apiBase.replace(/\/$/, "")}/v1/settlement/adjustments/${encodeURIComponent(adjustmentId)}`;
}
