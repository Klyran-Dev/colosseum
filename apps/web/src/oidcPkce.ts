import { OidcBrowserConfig } from "./authConfig";

const MAX_TRANSACTION_AGE_SECONDS = 600;
const MAX_ACCESS_TOKEN_LENGTH = 16 * 1024;
const MIN_TOKEN_LIFETIME_SECONDS = 30;
const MAX_TOKEN_LIFETIME_SECONDS = 3_600;

export interface PkceTransaction {
  state: string;
  nonce: string;
  codeVerifier: string;
  createdAtUnix: number;
  redirectUri?: string;
}

export interface OidcProviderMetadata {
  issuer: string;
  authorizationEndpoint: string;
  tokenEndpoint: string;
  jwksUri: string;
}

export interface BrowserToken {
  accessToken: string;
  expiresAtUnix: number;
}

function base64Url(bytes: Uint8Array): string {
  let binary = "";
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return btoa(binary).replaceAll("+", "-").replaceAll("/", "_").replaceAll("=", "");
}

function randomBytes(length: number): Uint8Array {
  if (!Number.isSafeInteger(length) || length <= 0 || length > 64) throw new Error("The random value length is outside its bound.");
  const cryptoApi = globalThis.crypto;
  if (typeof cryptoApi?.getRandomValues !== "function") throw new Error("A cryptographically secure random source is required for sign-in.");
  return cryptoApi.getRandomValues(new Uint8Array(length));
}

function validUnix(value: number): void {
  if (!Number.isSafeInteger(value) || value < 0) throw new Error("The sign-in clock is invalid.");
}

function stripTrailingSlash(value: string): string {
  return value.replace(/\/+$/u, "");
}

function safeEndpoint(value: string, issuer: string): boolean {
  try {
    const endpoint = new URL(value);
    const identity = new URL(issuer);
    if (endpoint.username || endpoint.password || endpoint.hash) return false;
    if (endpoint.origin !== identity.origin) return false;
    if (identity.protocol === "http:") {
      return endpoint.protocol === "http:" && ["localhost", "127.0.0.1", "[::1]"].includes(endpoint.hostname);
    }
    return endpoint.protocol === "https:";
  } catch {
    return false;
  }
}

export function createPkceTransaction(createdAtUnix = Math.floor(Date.now() / 1000), redirectUri?: string): PkceTransaction {
  validUnix(createdAtUnix);
  return {
    state: base64Url(randomBytes(32)),
    nonce: base64Url(randomBytes(32)),
    codeVerifier: base64Url(randomBytes(32)),
    createdAtUnix,
    ...(redirectUri ? { redirectUri } : {}),
  };
}

export async function pkceCodeChallenge(codeVerifier: string): Promise<string> {
  if (!/^[A-Za-z0-9_-]{43}$/u.test(codeVerifier)) throw new Error("The PKCE verifier is malformed.");
  const cryptoApi = globalThis.crypto;
  if (typeof cryptoApi?.subtle?.digest !== "function") throw new Error("Web Crypto is required for PKCE sign-in.");
  const digest = await cryptoApi.subtle.digest("SHA-256", new TextEncoder().encode(codeVerifier));
  return base64Url(new Uint8Array(digest));
}

export function validateProviderMetadata(config: OidcBrowserConfig, metadata: OidcProviderMetadata): void {
  if (stripTrailingSlash(metadata.issuer) !== stripTrailingSlash(config.issuer)) throw new Error("The identity provider issuer does not match the configured issuer.");
  if (!safeEndpoint(metadata.authorizationEndpoint, config.issuer)) throw new Error("The identity provider authorization endpoint is unsafe.");
  if (!safeEndpoint(metadata.tokenEndpoint, config.issuer)) throw new Error("The identity provider token endpoint is unsafe.");
  if (!safeEndpoint(metadata.jwksUri, config.issuer)) throw new Error("The identity provider JWKS endpoint is unsafe.");
}

export function buildAuthorizationUrl(
  config: OidcBrowserConfig,
  metadata: OidcProviderMetadata,
  transaction: PkceTransaction,
  codeChallenge: string,
): string {
  validateProviderMetadata(config, metadata);
  if (!/^[A-Za-z0-9_-]{43}$/u.test(codeChallenge)) throw new Error("The PKCE challenge is malformed.");
  const url = new URL(metadata.authorizationEndpoint);
  url.searchParams.set("response_type", "code");
  url.searchParams.set("client_id", config.clientId);
  url.searchParams.set("redirect_uri", config.redirectUri);
  url.searchParams.set("scope", config.scope);
  url.searchParams.set("state", transaction.state);
  url.searchParams.set("nonce", transaction.nonce);
  url.searchParams.set("code_challenge_method", "S256");
  url.searchParams.set("code_challenge", codeChallenge);
  if (config.audience) url.searchParams.set("audience", config.audience);
  return url.toString();
}

export function validateCallback(transaction: PkceTransaction, callbackUrl: URL, nowUnix = Math.floor(Date.now() / 1000)): string {
  validUnix(nowUnix);
  if (nowUnix < transaction.createdAtUnix || nowUnix - transaction.createdAtUnix > MAX_TRANSACTION_AGE_SECONDS) throw new Error("The sign-in request has expired; start again.");
  if (transaction.redirectUri && callbackUrl.toString().split("?", 1)[0] !== transaction.redirectUri) throw new Error("The sign-in callback URI does not match the sign-in request.");
  if (callbackUrl.hash) throw new Error("The sign-in callback contains an unsafe fragment.");
  if (callbackUrl.searchParams.get("state") !== transaction.state) throw new Error("The sign-in callback state does not match.");
  const providerError = callbackUrl.searchParams.get("error");
  if (providerError) throw new Error(providerError === "access_denied" ? "The identity provider denied sign-in." : "The identity provider returned a sign-in error.");
  const code = callbackUrl.searchParams.get("code");
  if (!code || code.length > 4096) throw new Error("The sign-in callback has no valid authorization code.");
  return code;
}

export function parseTokenResponse(value: unknown, nowUnix = Math.floor(Date.now() / 1000), requireIdToken = false): BrowserToken {
  validUnix(nowUnix);
  if (typeof value !== "object" || value === null || Array.isArray(value)) throw new Error("The identity provider returned an invalid token response.");
  const record = value as Record<string, unknown>;
  const allowed = new Set(["access_token", "token_type", "expires_in", "scope", "id_token"]);
  for (const key of Object.keys(record)) if (!allowed.has(key)) throw new Error(`The identity provider returned an unsupported token response field: ${key}.`);
  if (typeof record.access_token !== "string" || record.access_token.length === 0 || record.access_token.length > MAX_ACCESS_TOKEN_LENGTH) throw new Error("The identity provider returned an invalid access token.");
  if (record.token_type !== "Bearer") throw new Error("The identity provider returned an invalid token response type.");
  if (!Number.isSafeInteger(record.expires_in) || (record.expires_in as number) < MIN_TOKEN_LIFETIME_SECONDS || (record.expires_in as number) > MAX_TOKEN_LIFETIME_SECONDS) throw new Error("The identity provider returned an invalid token expiry.");
  if ("refresh_token" in record) throw new Error("Refresh tokens are not accepted by this browser session boundary.");
  if (requireIdToken && (typeof record.id_token !== "string" || record.id_token.length === 0 || record.id_token.length > MAX_ACCESS_TOKEN_LENGTH)) throw new Error("The identity provider did not return the required ID token.");
  return { accessToken: record.access_token, expiresAtUnix: nowUnix + (record.expires_in as number) };
}

export function isTokenUsable(token: BrowserToken, nowUnix = Math.floor(Date.now() / 1000)): boolean {
  return Number.isSafeInteger(nowUnix) && nowUnix >= 0 && token.expiresAtUnix > nowUnix;
}
