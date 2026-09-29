import { PublicKey } from "@solana/web3.js";
import {
  parseCanonicalU64,
  validateByteArray,
  validateUuidV7,
} from "./settlementAdjustments";

export const VAULT_RELEASE_STATUSES = [
  "proposed",
  "approved",
  "onchain_created",
  "executed",
  "cancelled",
  "unknown",
] as const;

export type VaultReleaseStatus = typeof VAULT_RELEASE_STATUSES[number];

export type VaultReleaseClaim = {
  id: string;
  merchant_id: string;
  idempotency_key: string;
  request_digest: number[];
  mint: string;
  token_program: string;
  tax_vault: string;
  destination: string;
  claim_pda: string | null;
  nonce: string;
  amount_atomic: string;
  reference_digest: number[];
  status: VaultReleaseStatus;
  proposer_subject: string;
  approver_subject: string | null;
  execute_after_unix: number;
  expires_at_unix: number;
  transaction_signature: string | null;
  slot: string | null;
  last_error_code: string | null;
};

export type VaultReleaseTransitionResponse = {
  status: "transitioned";
  claim_id: string;
  from_status: VaultReleaseStatus;
  to_status: VaultReleaseStatus;
  approver_subject: string | null;
};

function objectValue(value: unknown, message: string): Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error(message);
  return value as Record<string, unknown>;
}

function boundedText(value: unknown, label: string, maxBytes: number): string {
  if (typeof value !== "string" || value.trim() === "" || new TextEncoder().encode(value).length > maxBytes) {
    throw new Error(`${label} is invalid.`);
  }
  return value;
}

function optionalText(value: unknown, label: string, maxBytes: number): string | null {
  return value === null || value === undefined ? null : boundedText(value, label, maxBytes);
}

function publicKey(value: unknown, label: string): string {
  if (typeof value !== "string") throw new Error(`${label} is invalid.`);
  try {
    return new PublicKey(value).toBase58();
  } catch {
    throw new Error(`${label} is invalid.`);
  }
}

function optionalPublicKey(value: unknown, label: string): string | null {
  return value === null || value === undefined ? null : publicKey(value, label);
}

function status(value: unknown, label: string): VaultReleaseStatus {
  if (typeof value !== "string" || !VAULT_RELEASE_STATUSES.includes(value as VaultReleaseStatus)) {
    throw new Error(`${label} is invalid.`);
  }
  return value as VaultReleaseStatus;
}

function requiredU64(value: unknown, label: string): string {
  const parsed = parseCanonicalU64(value, label);
  if (parsed === "0") throw new Error(`${label} must be greater than zero.`);
  return parsed;
}

function optionalU64(value: unknown, label: string): string | null {
  return value === null || value === undefined ? null : parseCanonicalU64(value, label);
}

function unixSeconds(value: unknown, label: string): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value <= 0) throw new Error(`${label} is invalid.`);
  return value;
}

function parseClaimRecord(value: unknown): VaultReleaseClaim {
  const record = objectValue(value, "vault release claim is invalid.");
  const executeAfter = unixSeconds(record.execute_after_unix, "execute-after time");
  const expiresAt = unixSeconds(record.expires_at_unix, "expiry time");
  if (expiresAt <= executeAfter) throw new Error("vault release claim window is invalid.");
  return {
    id: validateUuidV7(record.id as string, "vault release claim ID"),
    merchant_id: validateUuidV7(record.merchant_id as string, "vault release merchant ID"),
    idempotency_key: boundedText(record.idempotency_key, "vault release idempotency key", 256),
    request_digest: validateByteArray(record.request_digest, "vault release request", false),
    mint: publicKey(record.mint, "vault release mint"),
    token_program: publicKey(record.token_program, "vault release token program"),
    tax_vault: publicKey(record.tax_vault, "vault release tax vault"),
    destination: publicKey(record.destination, "vault release destination"),
    claim_pda: optionalPublicKey(record.claim_pda, "vault release claim PDA"),
    nonce: requiredU64(record.nonce, "vault release nonce"),
    amount_atomic: requiredU64(record.amount_atomic, "vault release amount"),
    reference_digest: validateByteArray(record.reference_digest, "vault release reference", true),
    status: status(record.status, "vault release status"),
    proposer_subject: boundedText(record.proposer_subject, "vault release proposer subject", 256),
    approver_subject: optionalText(record.approver_subject, "vault release approver subject", 256),
    execute_after_unix: executeAfter,
    expires_at_unix: expiresAt,
    transaction_signature: optionalText(record.transaction_signature, "vault release transaction signature", 128),
    slot: optionalU64(record.slot, "vault release slot"),
    last_error_code: optionalText(record.last_error_code, "vault release error code", 128),
  };
}

export function parseVaultReleaseClaim(value: unknown): VaultReleaseClaim {
  return parseClaimRecord(value);
}

export function parseVaultReleaseTransitionResponse(value: unknown): VaultReleaseTransitionResponse {
  const record = objectValue(value, "vault release transition response is invalid.");
  if (record.status !== "transitioned") throw new Error("vault release transition status is invalid.");
  return {
    status: "transitioned",
    claim_id: validateUuidV7(record.claim_id as string, "vault release transition claim ID"),
    from_status: status(record.from_status, "vault release transition source status"),
    to_status: status(record.to_status, "vault release transition target status"),
    approver_subject: optionalText(record.approver_subject, "vault release transition approver subject", 256),
  };
}
