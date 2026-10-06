export const REPLAY_STATUSES = ["queued", "leased", "completed", "dead_lettered"] as const;
export const REPLAY_ACTIONS = ["acknowledge", "requeue"] as const;
export const MAX_REPLAY_LIST_LIMIT = 100;
export const MAX_REPLAY_ATTEMPTS = 8;
export const MAX_OPERATOR_NOTE_LENGTH = 1024;
export const MAX_TARGETED_RECOVERY_PAGES = 64;
export const MAX_ROUTE_VERIFICATION_RUNS = 50;

export type ReplayGapStatus = (typeof REPLAY_STATUSES)[number];
export type ReplayGapAction = (typeof REPLAY_ACTIONS)[number];

export const ROUTE_VERIFICATION_RUN_KINDS = ["scheduled", "backfill"] as const;
export const ROUTE_VERIFICATION_RUN_STATUSES = ["running", "completed", "completed_with_failures", "failed"] as const;
export type RouteVerificationRunKind = (typeof ROUTE_VERIFICATION_RUN_KINDS)[number];
export type RouteVerificationRunStatus = (typeof ROUTE_VERIFICATION_RUN_STATUSES)[number];

export type TargetedRecoveryResult = {
  status: "recovered";
  audit_id: string;
  signature: string;
  pages_scanned: number;
  slot: string;
  raw_transaction_count: number;
  raw_evidence_count: number;
  receipt_count: number;
};

export type ReplayGap = {
  id: string;
  stream_key: string;
  start_slot: string;
  end_slot: string;
  observed_finalized_slot: string;
  reason: string;
  attempts: number;
  status: ReplayGapStatus;
  last_error_code: string | null;
  created_at: string;
  dead_lettered_at: string | null;
  operator_acknowledged_at: string | null;
  operator_acknowledged_by: string | null;
  operator_acknowledgement: string | null;
};

export type RouteVerificationRun = {
  id: string;
  run_kind: RouteVerificationRunKind;
  status: RouteVerificationRunStatus;
  batch_size: number;
  examined_count: number;
  activated_count: number;
  already_active_count: number;
  failed_count: number;
  failure_code: string | null;
  operator_approval_id: string | null;
  started_at: string;
  completed_at: string | null;
};

const UUID_V7 = /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
const DECIMAL = /^(0|[1-9][0-9]*)$/;
const MAX_U64 = (1n << 64n) - 1n;
const STATUS_SET = new Set<string>(REPLAY_STATUSES);
const ROUTE_RUN_KIND_SET = new Set<string>(ROUTE_VERIFICATION_RUN_KINDS);
const ROUTE_RUN_STATUS_SET = new Set<string>(ROUTE_VERIFICATION_RUN_STATUSES);

function malformed(): Error {
  return new Error("The replay-gap response is malformed and cannot be displayed safely.");
}

function utf8Length(value: string): number {
  return new TextEncoder().encode(value).length;
}

function boundedString(value: unknown, field: string, maxLength: number, allowEmpty = false): string {
  if (typeof value !== "string" || (!allowEmpty && value.trim() === "") || utf8Length(value) > maxLength) {
    throw new Error(`${field} is malformed.`);
  }
  return value;
}

function timestamp(value: unknown, field: string): string {
  const parsed = boundedString(value, field, 64);
  if (Number.isNaN(Date.parse(parsed))) throw malformed();
  return parsed;
}

function optionalTimestamp(value: unknown, field: string): string | null {
  return value === null ? null : timestamp(value, field);
}

function decimalString(value: unknown, field: string, maximum: bigint = MAX_U64): string {
  const normalized = typeof value === "number"
    ? Number.isSafeInteger(value) && value >= 0 ? String(value) : ""
    : typeof value === "string" ? value : "";
  if (!DECIMAL.test(normalized)) throw malformed();
  try {
    if (BigInt(normalized) > maximum) throw malformed();
  } catch {
    throw malformed();
  }
  return normalized;
}

function parseStatus(value: unknown): ReplayGapStatus {
  if (typeof value !== "string" || !STATUS_SET.has(value)) throw malformed();
  return value as ReplayGapStatus;
}

function parseRouteRunCount(value: unknown, field: string): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0 || value > 64) {
    throw new Error(`${field} is malformed.`);
  }
  return value;
}

function parseOptionalUuidV7(value: unknown, field: string): string | null {
  if (value === null) return null;
  if (typeof value !== "string" || !UUID_V7.test(value)) throw new Error(`${field} is malformed.`);
  return value;
}

function parseRouteRunTimestamp(value: unknown, field: string, nullable: boolean): string | null {
  if (nullable && value === null) return null;
  return timestamp(value, field);
}

export function parseRouteVerificationRun(value: unknown): RouteVerificationRun {
  if (typeof value !== "object" || value === null || Array.isArray(value)) throw new Error("The route-verification response is malformed and cannot be displayed safely.");
  const run = value as Record<string, unknown>;
  if (typeof run.id !== "string" || !UUID_V7.test(run.id)) throw new Error("The route-verification response is malformed and cannot be displayed safely.");
  if (typeof run.run_kind !== "string" || !ROUTE_RUN_KIND_SET.has(run.run_kind)) throw new Error("The route-verification response is malformed and cannot be displayed safely.");
  if (typeof run.status !== "string" || !ROUTE_RUN_STATUS_SET.has(run.status)) throw new Error("The route-verification response is malformed and cannot be displayed safely.");
  const runKind = run.run_kind as RouteVerificationRunKind;
  const status = run.status as RouteVerificationRunStatus;
  const batchSize = parseRouteRunCount(run.batch_size, "batch_size");
  if (batchSize < 1) throw new Error("The route-verification response is malformed and cannot be displayed safely.");
  const examinedCount = parseRouteRunCount(run.examined_count, "examined_count");
  const activatedCount = parseRouteRunCount(run.activated_count, "activated_count");
  const alreadyActiveCount = parseRouteRunCount(run.already_active_count, "already_active_count");
  const failedCount = parseRouteRunCount(run.failed_count, "failed_count");
  const failureCode = run.failure_code === null ? null : boundedString(run.failure_code, "failure_code", 128);
  const operatorApprovalId = parseOptionalUuidV7(run.operator_approval_id, "operator_approval_id");
  const startedAt = timestamp(run.started_at, "started_at");
  const completedAt = parseRouteRunTimestamp(run.completed_at, "completed_at", true);
  const startedMillis = Date.parse(startedAt);
  const completedMillis = completedAt === null ? null : Date.parse(completedAt);
  if (completedMillis !== null && completedMillis < startedMillis) throw new Error("The route-verification response is malformed and cannot be displayed safely.");
  if ((runKind === "backfill") !== (operatorApprovalId !== null)) throw new Error("The route-verification response is malformed and cannot be displayed safely.");
  if (status === "running") {
    if (completedAt !== null) throw new Error("The route-verification response is malformed and cannot be displayed safely.");
  } else {
    if (completedAt === null || examinedCount !== activatedCount + alreadyActiveCount + failedCount) throw new Error("The route-verification response is malformed and cannot be displayed safely.");
  }
  if (status === "completed" && (failedCount !== 0 || failureCode !== null)) throw new Error("The route-verification response is malformed and cannot be displayed safely.");
  if (status === "completed_with_failures" && (failedCount === 0 || failureCode === null)) throw new Error("The route-verification response is malformed and cannot be displayed safely.");
  if (status === "failed" && failureCode === null) throw new Error("The route-verification response is malformed and cannot be displayed safely.");
  return {
    id: run.id,
    run_kind: runKind,
    status,
    batch_size: batchSize,
    examined_count: examinedCount,
    activated_count: activatedCount,
    already_active_count: alreadyActiveCount,
    failed_count: failedCount,
    failure_code: failureCode,
    operator_approval_id: operatorApprovalId,
    started_at: startedAt,
    completed_at: completedAt,
  };
}

export function parseRouteVerificationRunList(value: unknown): RouteVerificationRun[] {
  if (!Array.isArray(value) || value.length > MAX_ROUTE_VERIFICATION_RUNS) throw new Error("The route-verification response is malformed and cannot be displayed safely.");
  return value.map(parseRouteVerificationRun);
}

export function parseReplayGap(value: unknown): ReplayGap {
  if (typeof value !== "object" || value === null || Array.isArray(value)) throw malformed();
  const gap = value as Record<string, unknown>;
  if (typeof gap.id !== "string" || !UUID_V7.test(gap.id)) throw malformed();
  const status = parseStatus(gap.status);
  const attempts = Number(decimalString(gap.attempts, "attempts", BigInt(MAX_REPLAY_ATTEMPTS)));
  const startSlot = decimalString(gap.start_slot, "start_slot");
  const endSlot = decimalString(gap.end_slot, "end_slot");
  const observedFinalizedSlot = decimalString(gap.observed_finalized_slot, "observed_finalized_slot");
  if (BigInt(startSlot) > BigInt(endSlot) || BigInt(endSlot) > BigInt(observedFinalizedSlot)) throw malformed();
  const operatorAcknowledgedAt = optionalTimestamp(gap.operator_acknowledged_at, "operator_acknowledged_at");
  const operatorAcknowledgedBy = gap.operator_acknowledged_by === null
    ? null
    : boundedString(gap.operator_acknowledged_by, "operator_acknowledged_by", 128);
  const operatorAcknowledgement = gap.operator_acknowledgement === null
    ? null
    : boundedString(gap.operator_acknowledgement, "operator_acknowledgement", MAX_OPERATOR_NOTE_LENGTH);
  const acknowledgementPresent = operatorAcknowledgedAt !== null;
  if (acknowledgementPresent !== (operatorAcknowledgedBy !== null && operatorAcknowledgement !== null)) {
    throw malformed();
  }
  const deadLetteredAt = optionalTimestamp(gap.dead_lettered_at, "dead_lettered_at");
  if ((status === "dead_lettered") !== (deadLetteredAt !== null)) throw malformed();
  if (acknowledgementPresent && status !== "dead_lettered") throw malformed();
  return {
    id: gap.id,
    stream_key: boundedString(gap.stream_key, "stream_key", 128),
    start_slot: startSlot,
    end_slot: endSlot,
    observed_finalized_slot: observedFinalizedSlot,
    reason: boundedString(gap.reason, "reason", 128),
    attempts,
    status,
    last_error_code: gap.last_error_code === null ? null : boundedString(gap.last_error_code, "last_error_code", 128),
    created_at: timestamp(gap.created_at, "created_at"),
    dead_lettered_at: deadLetteredAt,
    operator_acknowledged_at: operatorAcknowledgedAt,
    operator_acknowledged_by: operatorAcknowledgedBy,
    operator_acknowledgement: operatorAcknowledgement,
  };
}

export function parseReplayGapList(value: unknown): ReplayGap[] {
  if (!Array.isArray(value) || value.length > MAX_REPLAY_LIST_LIMIT) throw malformed();
  return value.map(parseReplayGap);
}

export function validateOperatorNote(value: string): string {
  if (typeof value !== "string" || value.trim() === "" || utf8Length(value) > MAX_OPERATOR_NOTE_LENGTH) {
    throw new Error(`Operator note must contain 1–${MAX_OPERATOR_NOTE_LENGTH} UTF-8 bytes.`);
  }
  return value.trim();
}

export function validateRecoveryTarget(value: string, field: string): string {
  const normalized = value.trim();
  if (!normalized || utf8Length(normalized) > 128) throw new Error(`${field} must be a bounded non-empty value.`);
  return normalized;
}

function normalizedBase(apiBase: string): string {
  return apiBase.replace(/\/+$/, "");
}

export function buildReplayGapListUrl(
  apiBase: string,
  streamKey: string,
  status: ReplayGapStatus | "",
  limit: number,
): string {
  if (!streamKey.trim() || utf8Length(streamKey) > 128) throw new Error("Replay stream key is malformed.");
  if (!Number.isSafeInteger(limit) || limit < 1 || limit > MAX_REPLAY_LIST_LIMIT) throw new Error("Replay list limit is out of bounds.");
  const query = new URLSearchParams();
  query.set("stream_key", streamKey);
  if (status) query.set("status", status);
  query.set("limit", String(limit));
  return `${normalizedBase(apiBase)}/v1/indexer/replay-gaps?${query.toString()}`;
}

export function buildReplayGapActionUrl(apiBase: string, gapId: string, action: ReplayGapAction): string {
  if (!UUID_V7.test(gapId)) throw new Error("Replay gap ID is malformed.");
  if (!REPLAY_ACTIONS.includes(action)) throw new Error("Replay action is unsupported.");
  return `${normalizedBase(apiBase)}/v1/indexer/replay-gaps/${gapId}/${action}`;
}

export function buildTargetedRecoveryUrl(apiBase: string): string {
  return `${normalizedBase(apiBase)}/v1/indexer/targeted-recovery`;
}

export function buildRouteVerificationRunsUrl(apiBase: string, limit: number): string {
  if (!Number.isSafeInteger(limit) || limit < 1 || limit > MAX_ROUTE_VERIFICATION_RUNS) throw new Error("Route-verification run limit is out of bounds.");
  return `${normalizedBase(apiBase)}/v1/indexer/route-verification-runs?limit=${limit}`;
}

export function parseTargetedRecoveryResult(value: unknown): TargetedRecoveryResult {
  if (typeof value !== "object" || value === null || Array.isArray(value)) throw malformed();
  const result = value as Record<string, unknown>;
  if (result.status !== "recovered" || typeof result.audit_id !== "string" || !UUID_V7.test(result.audit_id)) throw malformed();
  const signature = boundedString(result.signature, "signature", 128);
  const pages = decimalString(result.pages_scanned, "pages_scanned", BigInt(MAX_TARGETED_RECOVERY_PAGES));
  const pageCount = Number(pages);
  const slot = decimalString(result.slot, "slot");
  const count = (field: string) => Number(decimalString(result[field], field, 1000n));
  const rawTransactionCount = count("raw_transaction_count");
  const rawEvidenceCount = count("raw_evidence_count");
  const receiptCount = count("receipt_count");
  if (pageCount < 1 || rawTransactionCount < 1 || rawEvidenceCount < 1 || receiptCount < 1) throw malformed();
  return {
    status: "recovered",
    audit_id: result.audit_id,
    signature,
    pages_scanned: pageCount,
    slot,
    raw_transaction_count: rawTransactionCount,
    raw_evidence_count: rawEvidenceCount,
    receipt_count: receiptCount,
  };
}
