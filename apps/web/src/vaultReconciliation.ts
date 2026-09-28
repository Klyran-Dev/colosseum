const UUID_V7 = /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
const DECIMAL_U64 = /^(0|[1-9][0-9]*)$/;

export type VaultReconciliationRun = {
  id: string;
  status: "matched" | "mismatch" | "error";
  opening_balance_atomic: string;
  receipt_tax_atomic: string;
  release_atomic: string;
  expected_balance_atomic: string | null;
  observed_balance_atomic: string | null;
  observed_slot: string | null;
  error_code: string | null;
  created_at: string;
};

export type VaultReconciliationTarget = {
  id: string;
  merchant_id: string;
  route_id: string;
  stream_key: string;
  mint: string;
  token_program: string;
  tax_vault: string;
  opening_balance_atomic: string;
  opening_balance_slot: string;
  opening_reference: string;
  receipt_tax_atomic: string;
  release_atomic: string;
};

export type VaultReconciliationTargetView = {
  target: VaultReconciliationTarget;
  latest_run: VaultReconciliationRun | null;
};

function malformed(message: string): Error {
  return new Error(`The vault reconciliation response is malformed: ${message}`);
}

function record(value: unknown, name: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) throw malformed(`${name} must be an object`);
  return value as Record<string, unknown>;
}

function uuid(value: unknown, name: string): string {
  if (typeof value !== "string" || !UUID_V7.test(value)) throw malformed(`${name} must be UUIDv7`);
  return value;
}

function text(value: unknown, name: string): string {
  if (typeof value !== "string" || value.trim() === "") throw malformed(`${name} must be non-empty text`);
  return value;
}

function atomic(value: unknown, name: string): string {
  if (typeof value !== "string" || !DECIMAL_U64.test(value)) throw malformed(`${name} must be an exact decimal string`);
  try {
    if (BigInt(value) > 18_446_744_073_709_551_615n) throw malformed(`${name} exceeds u64`);
  } catch (cause) {
    if (cause instanceof Error && cause.message.includes("malformed")) throw cause;
    throw malformed(`${name} is not a valid u64`);
  }
  return value;
}

function optionalAtomic(value: unknown, name: string): string | null {
  return value === null ? null : atomic(value, name);
}

function run(value: unknown): VaultReconciliationRun {
  const source = record(value, "run");
  if (source.status !== "matched" && source.status !== "mismatch" && source.status !== "error") throw malformed("run status is unsupported");
  return {
    id: uuid(source.id, "run.id"),
    status: source.status,
    opening_balance_atomic: atomic(source.opening_balance_atomic, "run.opening_balance_atomic"),
    receipt_tax_atomic: atomic(source.receipt_tax_atomic, "run.receipt_tax_atomic"),
    release_atomic: atomic(source.release_atomic, "run.release_atomic"),
    expected_balance_atomic: optionalAtomic(source.expected_balance_atomic, "run.expected_balance_atomic"),
    observed_balance_atomic: optionalAtomic(source.observed_balance_atomic, "run.observed_balance_atomic"),
    observed_slot: optionalAtomic(source.observed_slot, "run.observed_slot"),
    error_code: source.error_code === null ? null : text(source.error_code, "run.error_code"),
    created_at: text(source.created_at, "run.created_at"),
  };
}

function target(value: unknown): VaultReconciliationTarget {
  const source = record(value, "target");
  return {
    id: uuid(source.id, "target.id"),
    merchant_id: uuid(source.merchant_id, "target.merchant_id"),
    route_id: uuid(source.route_id, "target.route_id"),
    stream_key: text(source.stream_key, "target.stream_key"),
    mint: text(source.mint, "target.mint"),
    token_program: text(source.token_program, "target.token_program"),
    tax_vault: text(source.tax_vault, "target.tax_vault"),
    opening_balance_atomic: atomic(source.opening_balance_atomic, "target.opening_balance_atomic"),
    opening_balance_slot: atomic(source.opening_balance_slot, "target.opening_balance_slot"),
    opening_reference: text(source.opening_reference, "target.opening_reference"),
    receipt_tax_atomic: atomic(source.receipt_tax_atomic, "target.receipt_tax_atomic"),
    release_atomic: atomic(source.release_atomic, "target.release_atomic"),
  };
}

export function parseVaultReconciliationTargetViews(value: unknown): VaultReconciliationTargetView[] {
  if (!Array.isArray(value)) throw malformed("target response must be an array");
  if (value.length > 64) throw malformed("target response exceeds the bounded limit");
  return value.map((entry) => {
    const source = record(entry, "target view");
    return {
      target: target(source.target),
      latest_run: source.latest_run === null ? null : run(source.latest_run),
    };
  });
}

export function parseVaultReconciliationRuns(value: unknown): VaultReconciliationRun[] {
  if (!Array.isArray(value)) throw malformed("run response must be an array");
  if (value.length > 64) throw malformed("run response exceeds the bounded limit");
  return value.map(run);
}

export function buildVaultReconciliationTargetsUrl(apiBase: string, limit = 50): string {
  if (!Number.isSafeInteger(limit) || limit < 1 || limit > 64) throw new RangeError("reconciliation target limit must be between 1 and 64");
  const base = apiBase.replace(/\/$/, "");
  return `${base}/v1/vault/reconciliation/targets?limit=${limit}`;
}

export function buildVaultReconciliationRunsUrl(apiBase: string, targetId: string, limit = 50): string {
  if (!UUID_V7.test(targetId)) throw new Error("reconciliation target ID must be UUIDv7");
  if (!Number.isSafeInteger(limit) || limit < 1 || limit > 64) throw new RangeError("reconciliation run limit must be between 1 and 64");
  const base = apiBase.replace(/\/$/, "");
  return `${base}/v1/vault/reconciliation/targets/${encodeURIComponent(targetId)}/runs?limit=${limit}`;
}
