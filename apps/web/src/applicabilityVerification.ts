const UUIDV7 = /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;

export const APPLICABILITY_VERIFICATION_STATUSES = [
  "not_applicable",
  "pending",
  "retrying",
  "verified",
  "dead_lettered",
] as const;

export type ApplicabilityVerificationStatus = (typeof APPLICABILITY_VERIFICATION_STATUSES)[number];

export type ApplicabilityVerification = {
  settlement_intent_id: string;
  status: ApplicabilityVerificationStatus;
  signature: string | null;
  event_index: number | null;
  event_slot: string | null;
  provider_slot: string | null;
  receipt: string | null;
  binding: string | null;
  merchant: string | null;
  mint: string | null;
  context_id_hex: string | null;
  attestation_digest_hex: string | null;
  event_evidence_digest_hex: string | null;
  verification_commitment: string | null;
  attempts: number | null;
  last_error_code: string | null;
  verified_at: string | null;
  claimed_at: string | null;
  dead_lettered_at: string | null;
};

function objectValue(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error("The applicability evidence response is malformed.");
  return value as Record<string, unknown>;
}

function nullableString(value: unknown, field: string): string | null {
  if (value === null || value === undefined) return null;
  if (typeof value !== "string" || value.length > 512) throw new Error(`The applicability evidence field ${field} is invalid.`);
  return value;
}

function nullableBoundedNumber(value: unknown, field: string): number | null {
  if (value === null || value === undefined) return null;
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0) throw new Error(`The applicability evidence field ${field} is invalid.`);
  return value;
}

export function parseApplicabilityVerification(value: unknown): ApplicabilityVerification {
  const response = objectValue(value);
  if (typeof response.settlement_intent_id !== "string" || !UUIDV7.test(response.settlement_intent_id)) throw new Error("The applicability evidence returned an invalid settlement intent.");
  if (typeof response.status !== "string" || !(APPLICABILITY_VERIFICATION_STATUSES as readonly string[]).includes(response.status)) throw new Error("The applicability evidence returned an unknown status.");
  return {
    settlement_intent_id: response.settlement_intent_id.toLowerCase(),
    status: response.status as ApplicabilityVerificationStatus,
    signature: nullableString(response.signature, "signature"),
    event_index: nullableBoundedNumber(response.event_index, "event_index"),
    event_slot: nullableString(response.event_slot, "event_slot"),
    provider_slot: nullableString(response.provider_slot, "provider_slot"),
    receipt: nullableString(response.receipt, "receipt"),
    binding: nullableString(response.binding, "binding"),
    merchant: nullableString(response.merchant, "merchant"),
    mint: nullableString(response.mint, "mint"),
    context_id_hex: nullableString(response.context_id_hex, "context_id_hex"),
    attestation_digest_hex: nullableString(response.attestation_digest_hex, "attestation_digest_hex"),
    event_evidence_digest_hex: nullableString(response.event_evidence_digest_hex, "event_evidence_digest_hex"),
    verification_commitment: nullableString(response.verification_commitment, "verification_commitment"),
    attempts: nullableBoundedNumber(response.attempts, "attempts"),
    last_error_code: nullableString(response.last_error_code, "last_error_code"),
    verified_at: nullableString(response.verified_at, "verified_at"),
    claimed_at: nullableString(response.claimed_at, "claimed_at"),
    dead_lettered_at: nullableString(response.dead_lettered_at, "dead_lettered_at"),
  };
}

export function buildApplicabilityVerificationUrl(apiBase: string, intentId: string, streamKey: string): string {
  if (!UUIDV7.test(intentId)) throw new Error("The settlement intent must be UUIDv7.");
  if (!streamKey.trim() || streamKey.length > 200) throw new Error("The indexer stream is not configured.");
  const base = apiBase.replace(/\/$/, "");
  return `${base}/v1/settlement/intents/${encodeURIComponent(intentId)}/applicability-binding-verification?stream_key=${encodeURIComponent(streamKey)}`;
}
