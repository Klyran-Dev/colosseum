import { createLocalJWKSet, decodeProtectedHeader, jwtVerify, type JWTPayload } from "jose";
import type { OidcProviderMetadata } from "./oidcPkce";

const MAX_JWKS_RESPONSE_BYTES = 64 * 1024;
const MAX_JWKS_KEYS = 16;
const MAX_KEY_ID_LENGTH = 128;
const MAX_SUBJECT_LENGTH = 256;
const MAX_TOKEN_LENGTH = 16 * 1024;
const CLOCK_SKEW_SECONDS = 60;
const REQUEST_TIMEOUT_MS = 5_000;
const VALID_ALGORITHMS = ["EdDSA", "RS256"] as const;

type Fetcher = (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>;
type SupportedAlgorithm = typeof VALID_ALGORITHMS[number];

export interface OidcIdTokenValidationContext {
  clientId: string;
  nonce: string;
  nowUnix: number;
}

export interface OidcIdTokenClaims {
  subject: string;
  issuer: string;
  audience: string[];
  issuedAtUnix: number;
  expiresAtUnix: number;
}

function supportedAlgorithm(value: unknown): value is SupportedAlgorithm {
  return typeof value === "string" && (VALID_ALGORITHMS as readonly string[]).includes(value);
}

function boundedString(value: unknown, label: string, maxLength: number): string {
  if (typeof value !== "string" || value.length === 0 || value.length > maxLength) throw new Error(`The ID token ${label} is invalid.`);
  return value;
}

function parseJwks(value: unknown): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) throw new Error("The identity provider JWKS is invalid.");
  const record = value as Record<string, unknown>;
  if (!Array.isArray(record.keys) || record.keys.length === 0 || record.keys.length > MAX_JWKS_KEYS) throw new Error("The identity provider JWKS key set is outside its bound.");
  const seen = new Set<string>();
  for (const candidate of record.keys) {
    if (typeof candidate !== "object" || candidate === null || Array.isArray(candidate)) throw new Error("The identity provider JWKS contains an invalid key.");
    const key = candidate as Record<string, unknown>;
    const kid = boundedString(key.kid, "key identifier", MAX_KEY_ID_LENGTH);
    if (seen.has(kid)) throw new Error("The identity provider JWKS contains duplicate key identifiers.");
    seen.add(kid);
    if ("d" in key || "p" in key || "q" in key || "dp" in key || "dq" in key || "qi" in key || "oth" in key) throw new Error("The identity provider JWKS contains private key material.");
    if (!supportedAlgorithm(key.alg)) throw new Error("The identity provider JWKS contains an unsupported algorithm.");
    if (key.alg === "RS256" && key.kty !== "RSA") throw new Error("The identity provider JWKS RSA key type is invalid.");
    if (key.alg === "EdDSA" && (key.kty !== "OKP" || key.crv !== "Ed25519")) throw new Error("The identity provider JWKS EdDSA key type is invalid.");
  }
  return record;
}

async function fetchJwks(fetcher: Fetcher, url: string): Promise<Record<string, unknown>> {
  const controller = new AbortController();
  const timeout = globalThis.setTimeout(() => controller.abort(), REQUEST_TIMEOUT_MS);
  try {
    const response = await fetcher(url, { method: "GET", headers: { accept: "application/json" }, credentials: "omit", signal: controller.signal });
    const body = await response.text();
    if (body.length > MAX_JWKS_RESPONSE_BYTES) throw new Error("The identity provider JWKS response is too large.");
    if (!response.ok) throw new Error("The identity provider JWKS could not be loaded.");
    let parsed: unknown;
    try {
      parsed = JSON.parse(body);
    } catch {
      throw new Error("The identity provider JWKS is not valid JSON.");
    }
    return parseJwks(parsed);
  } catch (cause) {
    if (cause instanceof DOMException && cause.name === "AbortError") throw new Error("The identity provider JWKS did not respond in time.");
    throw cause;
  } finally {
    globalThis.clearTimeout(timeout);
  }
}

function claimNumber(payload: JWTPayload, key: "iat" | "exp"): number {
  const value = payload[key];
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0) throw new Error(`The ID token ${key} claim is invalid.`);
  return value;
}

function audienceValues(value: JWTPayload["aud"]): string[] {
  const values = typeof value === "string" ? [value] : value;
  if (!Array.isArray(values) || values.length === 0 || values.some((item) => typeof item !== "string" || item.length === 0 || item.length > 256)) throw new Error("The ID token audience claim is invalid.");
  return values;
}

function tokenHeader(token: string): { alg: SupportedAlgorithm; kid: string } {
  try {
    const header = decodeProtectedHeader(token);
    if (!supportedAlgorithm(header.alg)) throw new Error("The ID token algorithm is not allowed.");
    return { alg: header.alg, kid: boundedString(header.kid, "key identifier", MAX_KEY_ID_LENGTH) };
  } catch (cause) {
    if (cause instanceof Error && /not allowed|key identifier|algorithm/u.test(cause.message)) throw cause;
    throw new Error("The ID token header is invalid.");
  }
}

function claimsFromPayload(payload: JWTPayload, context: OidcIdTokenValidationContext, issuer: string): OidcIdTokenClaims {
  const subject = boundedString(payload.sub, "subject", MAX_SUBJECT_LENGTH);
  const audience = audienceValues(payload.aud);
  const issuedAtUnix = claimNumber(payload, "iat");
  const expiresAtUnix = claimNumber(payload, "exp");
  if (payload.nonce !== context.nonce) throw new Error("The ID token nonce does not match the sign-in request.");
  if (issuedAtUnix > context.nowUnix + CLOCK_SKEW_SECONDS || expiresAtUnix <= context.nowUnix - CLOCK_SKEW_SECONDS) throw new Error("The ID token time claims are outside the allowed window.");
  return { subject, issuer, audience, issuedAtUnix, expiresAtUnix };
}

export async function validateOidcIdToken(
  token: string,
  metadata: OidcProviderMetadata,
  context: OidcIdTokenValidationContext,
  fetcher: Fetcher = globalThis.fetch,
): Promise<OidcIdTokenClaims> {
  if (typeof token !== "string" || token.length === 0 || token.length > MAX_TOKEN_LENGTH) throw new Error("The identity provider returned an invalid ID token.");
  if (!Number.isSafeInteger(context.nowUnix) || context.nowUnix < 0) throw new Error("The ID token validation clock is invalid.");
  const header = tokenHeader(token);
  let lastError: unknown = new Error("The ID token could not be verified.");
  for (let attempt = 0; attempt < 2; attempt += 1) {
    try {
      const jwks = await fetchJwks(fetcher, metadata.jwksUri);
      const matching = (jwks.keys as Array<Record<string, unknown>>).find((key) => key.kid === header.kid);
      if (!matching || matching.alg !== header.alg) throw new Error("The identity provider JWKS does not contain the requested signing key.");
      const keySet = createLocalJWKSet(jwks as { keys: Array<Record<string, unknown>> });
      const verified = await jwtVerify(token, keySet, {
        algorithms: [header.alg],
        issuer: metadata.issuer,
        audience: context.clientId,
        clockTolerance: CLOCK_SKEW_SECONDS,
        requiredClaims: ["iss", "sub", "aud", "iat", "exp", "nonce"],
      });
      return claimsFromPayload(verified.payload, context, metadata.issuer);
    } catch (cause) {
      lastError = cause;
    }
  }
  throw lastError instanceof Error ? lastError : new Error("The ID token could not be verified.");
}
