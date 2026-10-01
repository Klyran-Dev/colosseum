const DOMAIN_SEPARATOR = new TextEncoder().encode(
  "solana-tax-protocol/calculation-attestation/v1",
);
const MAX_TEXT_BYTES = 128;
const MAX_U64 = (1n << 64n) - 1n;
const MIN_I64 = -(1n << 63n);
const MAX_I64 = (1n << 63n) - 1n;

type RecordValue = Record<string, unknown>;

function recordValue(value: unknown): RecordValue {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new Error("Calculation attestation unsigned payload must be an object.");
  }
  return value as RecordValue;
}

function fixedBytes(value: unknown, field: string): Uint8Array {
  if (!Array.isArray(value) || value.length !== 32
      || value.some((byte) => typeof byte !== "number"
        || !Number.isInteger(byte) || byte < 0 || byte > 255)) {
    throw new Error(`${field} must be exactly 32 bytes.`);
  }
  return Uint8Array.from(value as number[]);
}

function positiveU32(value: unknown, field: string): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value <= 0
      || value > 0xffff_ffff) {
    throw new Error(`${field} must be a positive u32.`);
  }
  return value;
}

function schemaVersion(value: unknown): number {
  if (typeof value !== "number" || !Number.isInteger(value) || value !== 1) {
    throw new Error("Calculation attestation schema version is unsupported.");
  }
  return value;
}

function canonicalU64(value: unknown, field: string): bigint {
  if (typeof value !== "string" || !/^(0|[1-9][0-9]*)$/.test(value)) {
    throw new Error(`${field} must be a canonical unsigned decimal string.`);
  }
  const parsed = BigInt(value);
  if (parsed > MAX_U64) {
    throw new Error(`${field} exceeds u64 range.`);
  }
  return parsed;
}

function signedI64(value: unknown, field: string): bigint {
  const parsed = typeof value === "number"
    ? Number.isSafeInteger(value) ? BigInt(value) : null
    : typeof value === "string" && /^-?(0|[1-9][0-9]*)$/.test(value) ? BigInt(value) : null;
  if (parsed === null || parsed < MIN_I64 || parsed > MAX_I64) {
    throw new Error(`${field} must be a signed i64.`);
  }
  return parsed;
}

function boundedText(value: unknown, field: string): Uint8Array {
  if (typeof value !== "string" || value.length === 0) {
    throw new Error(`${field} must be non-empty text.`);
  }
  const encoded = new TextEncoder().encode(value);
  if (encoded.length > MAX_TEXT_BYTES) {
    throw new Error(`${field} must be at most ${MAX_TEXT_BYTES} bytes.`);
  }
  return encoded;
}

function u16(value: number): Uint8Array {
  const bytes = new Uint8Array(2);
  new DataView(bytes.buffer).setUint16(0, value, true);
  return bytes;
}

function u32(value: number): Uint8Array {
  const bytes = new Uint8Array(4);
  new DataView(bytes.buffer).setUint32(0, value, true);
  return bytes;
}

function u64(value: bigint): Uint8Array {
  const bytes = new Uint8Array(8);
  new DataView(bytes.buffer).setBigUint64(0, value, true);
  return bytes;
}

function i64(value: bigint): Uint8Array {
  const bytes = new Uint8Array(8);
  new DataView(bytes.buffer).setBigInt64(0, value, true);
  return bytes;
}

function lengthPrefixedText(value: Uint8Array): Uint8Array {
  return concat(u16(value.length), value);
}

function concat(...parts: Uint8Array[]): Uint8Array {
  const result = new Uint8Array(parts.reduce((length, part) => length + part.length, 0));
  let offset = 0;
  for (const part of parts) {
    result.set(part, offset);
    offset += part.length;
  }
  return result;
}

/**
 * Builds the exact issuer-signed calculation-attestation payload.
 *
 * This is a compatibility primitive for displaying and testing server-owned
 * evidence. It does not verify an issuer, create a signature, or authorize a
 * Solana transaction.
 */
export function buildCalculationAttestationPayload(value: unknown): Uint8Array {
  const unsigned = recordValue(value);
  const version = schemaVersion(unsigned.schema_version);
  const merchantId = fixedBytes(unsigned.merchant_id, "merchant_id");
  const ruleSetId = boundedText(unsigned.rule_set_id, "rule_set_id");
  const ruleVersion = positiveU32(unsigned.rule_version, "rule_version");
  const jurisdiction = boundedText(unsigned.jurisdiction, "jurisdiction");
  const currency = boundedText(unsigned.currency, "currency");
  if (currency.length !== 3 || !new TextDecoder().decode(currency).match(/^[A-Z]{3}$/)) {
    throw new Error("currency must be a three-letter uppercase code.");
  }
  const assetMint = fixedBytes(unsigned.asset_mint, "asset_mint");
  const factsDigest = fixedBytes(unsigned.facts_digest, "facts_digest");
  const ruleDigest = fixedBytes(unsigned.rule_digest, "rule_digest");
  const gross = canonicalU64(unsigned.gross_atomic, "gross_atomic");
  const tax = canonicalU64(unsigned.tax_atomic, "tax_atomic");
  const net = canonicalU64(unsigned.net_atomic, "net_atomic");
  if (tax + net !== gross) {
    throw new Error("tax plus net must equal gross.");
  }
  const issuedAt = signedI64(unsigned.issued_at_unix, "issued_at_unix");
  const expiresAt = signedI64(unsigned.expires_at_unix, "expires_at_unix");
  if (issuedAt < 0n || expiresAt <= issuedAt || expiresAt - issuedAt > 86_400n) {
    throw new Error("attestation validity window is invalid or too long.");
  }
  const sequence = canonicalU64(unsigned.sequence, "sequence");
  if (sequence === 0n) {
    throw new Error("sequence must be positive.");
  }
  const issuerKeyId = fixedBytes(unsigned.issuer_key_id, "issuer_key_id");
  return concat(
    DOMAIN_SEPARATOR,
    u16(version),
    merchantId,
    lengthPrefixedText(ruleSetId),
    u32(ruleVersion),
    lengthPrefixedText(jurisdiction),
    lengthPrefixedText(currency),
    assetMint,
    factsDigest,
    ruleDigest,
    u64(gross),
    u64(tax),
    u64(net),
    i64(issuedAt),
    i64(expiresAt),
    u64(sequence),
    issuerKeyId,
  );
}

export async function sha256Digest(value: Uint8Array): Promise<Uint8Array> {
  if (!globalThis.crypto?.subtle) {
    throw new Error("Web Crypto SHA-256 is unavailable.");
  }
  const input = new ArrayBuffer(value.byteLength);
  new Uint8Array(input).set(value);
  return new Uint8Array(await globalThis.crypto.subtle.digest("SHA-256", input));
}
