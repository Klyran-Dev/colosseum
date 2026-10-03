import { OidcBrowserConfig } from "./authConfig";
import {
  buildAuthorizationUrl,
  createPkceTransaction,
  OidcProviderMetadata,
  parseTokenResponse,
  pkceCodeChallenge,
  PkceTransaction,
  validateCallback,
  validateProviderMetadata,
  BrowserToken,
} from "./oidcPkce";

export const PKCE_TRANSACTION_STORAGE_KEY = "solana-tax.oidc.pkce.v1";
const MAX_PROVIDER_RESPONSE_BYTES = 64 * 1024;
const REQUEST_TIMEOUT_MS = 5_000;

type Fetcher = (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>;

function boundedTransaction(value: unknown): PkceTransaction {
  if (typeof value !== "object" || value === null || Array.isArray(value)) throw new Error("The stored sign-in transaction is invalid.");
  const record = value as Record<string, unknown>;
  const fields = [record.state, record.nonce, record.codeVerifier];
  if (!fields.every((field) => typeof field === "string" && /^[A-Za-z0-9_-]{43}$/u.test(field))) throw new Error("The stored sign-in transaction is invalid.");
  if (!Number.isSafeInteger(record.createdAtUnix) || (record.createdAtUnix as number) < 0) throw new Error("The stored sign-in transaction clock is invalid.");
  if (record.redirectUri !== undefined && (typeof record.redirectUri !== "string" || record.redirectUri.length > 2_048)) throw new Error("The stored sign-in redirect is invalid.");
  return {
    state: record.state as string,
    nonce: record.nonce as string,
    codeVerifier: record.codeVerifier as string,
    createdAtUnix: record.createdAtUnix as number,
    ...(record.redirectUri ? { redirectUri: record.redirectUri as string } : {}),
  };
}

export function storePkceTransaction(storage: Storage, transaction: PkceTransaction): void {
  const encoded = JSON.stringify(transaction);
  if (encoded.length > 4_096) throw new Error("The sign-in transaction is too large.");
  try {
    storage.setItem(PKCE_TRANSACTION_STORAGE_KEY, encoded);
  } catch {
    throw new Error("The browser could not stage the sign-in request safely.");
  }
}

export function consumePkceTransaction(storage: Storage, nowUnix = Math.floor(Date.now() / 1000)): PkceTransaction | null {
  let encoded: string | null;
  try {
    encoded = storage.getItem(PKCE_TRANSACTION_STORAGE_KEY);
    storage.removeItem(PKCE_TRANSACTION_STORAGE_KEY);
  } catch {
    throw new Error("The browser could not read the sign-in request safely.");
  }
  if (!encoded) return null;
  let transaction: PkceTransaction;
  try {
    transaction = boundedTransaction(JSON.parse(encoded));
  } catch {
    throw new Error("The stored sign-in transaction is invalid; start again.");
  }
  if (!Number.isSafeInteger(nowUnix) || nowUnix < transaction.createdAtUnix || nowUnix - transaction.createdAtUnix > 600) throw new Error("The stored sign-in transaction has expired; start again.");
  return transaction;
}

async function fetchJson(fetcher: Fetcher, url: string, init: RequestInit): Promise<unknown> {
  const controller = new AbortController();
  const timeout = globalThis.setTimeout(() => controller.abort(), REQUEST_TIMEOUT_MS);
  try {
    const response = await fetcher(url, { ...init, signal: controller.signal });
    const body = await response.text();
    if (body.length > MAX_PROVIDER_RESPONSE_BYTES) throw new Error("The identity provider response is too large.");
    let parsed: unknown = null;
    try {
      parsed = body ? JSON.parse(body) : null;
    } catch {
      throw new Error("The identity provider response is not valid JSON.");
    }
    if (!response.ok) throw new Error("The identity provider could not complete sign-in.");
    return parsed;
  } catch (cause) {
    if (cause instanceof DOMException && cause.name === "AbortError") throw new Error("The identity provider did not respond in time.");
    throw cause;
  } finally {
    globalThis.clearTimeout(timeout);
  }
}

function metadataFromResponse(value: unknown): OidcProviderMetadata {
  if (typeof value !== "object" || value === null || Array.isArray(value)) throw new Error("The identity provider metadata is invalid.");
  const record = value as Record<string, unknown>;
  if (![record.issuer, record.authorization_endpoint, record.token_endpoint, record.jwks_uri].every((field) => typeof field === "string" && field.length > 0 && field.length <= 2_048)) throw new Error("The identity provider metadata is incomplete.");
  return {
    issuer: record.issuer as string,
    authorizationEndpoint: record.authorization_endpoint as string,
    tokenEndpoint: record.token_endpoint as string,
    jwksUri: record.jwks_uri as string,
  };
}

function discoveryUrl(issuer: string): string {
  const base = issuer.endsWith("/") ? issuer : `${issuer}/`;
  return new URL(".well-known/openid-configuration", base).toString();
}

export async function discoverOidcProvider(config: OidcBrowserConfig, fetcher: Fetcher = globalThis.fetch): Promise<OidcProviderMetadata> {
  const metadata = metadataFromResponse(await fetchJson(fetcher, discoveryUrl(config.issuer), {
    method: "GET",
    headers: { accept: "application/json" },
    credentials: "omit",
  }));
  validateProviderMetadata(config, metadata);
  return metadata;
}

export async function beginOidcAuthorization(
  config: OidcBrowserConfig,
  storage: Storage,
  fetcher: Fetcher = globalThis.fetch,
  nowUnix = Math.floor(Date.now() / 1000),
): Promise<string> {
  const metadata = await discoverOidcProvider(config, fetcher);
  const transaction = createPkceTransaction(nowUnix, config.redirectUri);
  const challenge = await pkceCodeChallenge(transaction.codeVerifier);
  storePkceTransaction(storage, transaction);
  return buildAuthorizationUrl(config, metadata, transaction, challenge);
}

async function exchangeAuthorizationCode(
  config: OidcBrowserConfig,
  metadata: OidcProviderMetadata,
  transaction: PkceTransaction,
  code: string,
  fetcher: Fetcher,
  nowUnix: number,
): Promise<BrowserToken> {
  const body = new URLSearchParams({
    grant_type: "authorization_code",
    code,
    client_id: config.clientId,
    redirect_uri: config.redirectUri,
    code_verifier: transaction.codeVerifier,
  });
  const tokenResponse = await fetchJson(fetcher, metadata.tokenEndpoint, {
    method: "POST",
    headers: { accept: "application/json", "content-type": "application/x-www-form-urlencoded" },
    credentials: "omit",
    body: body.toString(),
  });
  const requiresIdToken = config.scope.split(/\s+/u).includes("openid");
  const parsed = parseTokenResponse(tokenResponse, nowUnix, requiresIdToken);
  if (requiresIdToken) {
    if (typeof tokenResponse !== "object" || tokenResponse === null || Array.isArray(tokenResponse)) throw new Error("The identity provider returned an invalid token response.");
    const idToken = (tokenResponse as Record<string, unknown>).id_token;
    if (typeof idToken !== "string") throw new Error("The identity provider did not return the required ID token.");
    const { validateOidcIdToken } = await import("./oidcIdToken");
    await validateOidcIdToken(idToken, metadata, { clientId: config.clientId, nonce: transaction.nonce, nowUnix }, fetcher);
  }
  return parsed;
}

export async function completeOidcCallback(
  config: OidcBrowserConfig,
  callbackUrl: URL,
  storage: Storage,
  fetcher: Fetcher = globalThis.fetch,
  nowUnix = Math.floor(Date.now() / 1000),
): Promise<BrowserToken> {
  const transaction = consumePkceTransaction(storage, nowUnix);
  if (!transaction) throw new Error("No active sign-in request was found; start again.");
  const code = validateCallback(transaction, callbackUrl, nowUnix);
  const metadata = await discoverOidcProvider(config, fetcher);
  return exchangeAuthorizationCode(config, metadata, transaction, code, fetcher, nowUnix);
}
