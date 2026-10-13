const HEX = Array.from({ length: 256 }, (_, value) => value.toString(16).padStart(2, "0"));

/**
 * Creates a client-only idempotency key without depending on secure-context
 * `randomUUID()`. The server remains the authority for UUIDv7 entity IDs.
 */
export function createClientUuid(): string {
  const cryptoApi = globalThis.crypto;
  if (typeof cryptoApi?.randomUUID === "function") return cryptoApi.randomUUID();
  if (typeof cryptoApi?.getRandomValues !== "function") throw new Error("A cryptographically secure random source is required.");

  const bytes = new Uint8Array(16);
  cryptoApi.getRandomValues(bytes);
  bytes[6] = (bytes[6] & 0x0f) | 0x40;
  bytes[8] = (bytes[8] & 0x3f) | 0x80;

  return [
    bytes.slice(0, 4),
    bytes.slice(4, 6),
    bytes.slice(6, 8),
    bytes.slice(8, 10),
    bytes.slice(10, 16),
  ].map((part) => Array.from(part, (byte) => HEX[byte]).join("")).join("-");
}

/** Creates a client idempotency key with the repository's UUIDv7 contract. */
export function createClientUuidV7(): string {
  const cryptoApi = globalThis.crypto;
  if (typeof cryptoApi?.getRandomValues !== "function") throw new Error("A cryptographically secure random source is required.");
  const bytes = new Uint8Array(16);
  cryptoApi.getRandomValues(bytes);
  const milliseconds = BigInt(Date.now());
  for (let index = 5; index >= 0; index -= 1) {
    bytes[index] = Number((milliseconds >> BigInt((5 - index) * 8)) & 0xffn);
  }
  bytes[6] = (bytes[6] & 0x0f) | 0x70;
  bytes[8] = (bytes[8] & 0x3f) | 0x80;
  return [
    bytes.slice(0, 4),
    bytes.slice(4, 6),
    bytes.slice(6, 8),
    bytes.slice(8, 10),
    bytes.slice(10, 16),
  ].map((part) => Array.from(part, (byte) => HEX[byte]).join("")).join("-");
}

/**
 * Creates a positive canonical Solana u64 nonce for a wallet-bound request.
 * It is an opaque replay identity, not a user-entered financial value.
 */
export function createClientNonce(): string {
  const cryptoApi = globalThis.crypto;
  if (typeof cryptoApi?.getRandomValues !== "function") throw new Error("A cryptographically secure random source is required.");
  const words = new Uint32Array(2);
  cryptoApi.getRandomValues(words);
  const value = (BigInt(words[0]) << 32n) | BigInt(words[1]);
  return (value === 0n ? 1n : value).toString();
}
