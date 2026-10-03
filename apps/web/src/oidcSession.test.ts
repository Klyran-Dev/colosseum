import { describe, expect, it, vi } from "vitest";
import { exportJWK, generateKeyPair, SignJWT } from "jose";
import { parseBrowserAuthConfig } from "./authConfig";
import {
  beginOidcAuthorization,
  completeOidcCallback,
  consumePkceTransaction,
  discoverOidcProvider,
  storePkceTransaction,
} from "./oidcSession";
import { createPkceTransaction } from "./oidcPkce";

const config = parseBrowserAuthConfig({
  VITE_DEPLOYMENT_ENVIRONMENT: "staging",
  VITE_OIDC_ISSUER: "https://identity.example.test/",
  VITE_OIDC_CLIENT_ID: "tax-console",
  VITE_OIDC_REDIRECT_URI: "https://console.example.test/auth/callback",
  VITE_OIDC_SCOPE: "openid profile",
});

function memoryStorage(): Storage {
  const values = new Map<string, string>();
  return {
    get length() { return values.size; },
    clear: () => values.clear(),
    getItem: (key) => values.get(key) ?? null,
    key: (index) => Array.from(values.keys())[index] ?? null,
    removeItem: (key) => values.delete(key),
    setItem: (key, value) => values.set(key, value),
  };
}

describe("OIDC session transport", () => {
  it("stores and consumes one bounded PKCE transaction exactly once", () => {
    const storage = memoryStorage();
    const transaction = createPkceTransaction(1_800_000_000, "https://console.example.test/auth/callback");
    storePkceTransaction(storage, transaction);
    expect(consumePkceTransaction(storage, 1_800_000_001)).toEqual(transaction);
    expect(consumePkceTransaction(storage, 1_800_000_002)).toBeNull();
  });

  it("discovers and validates only the configured provider endpoints", async () => {
    if (config.mode !== "oidc" || !config.oidc) throw new Error("test OIDC config did not parse");
    const fetcher = vi.fn(async () => new Response(JSON.stringify({
      issuer: "https://identity.example.test/",
      authorization_endpoint: "https://identity.example.test/oauth2/authorize",
      token_endpoint: "https://identity.example.test/oauth2/token",
      jwks_uri: "https://identity.example.test/.well-known/jwks.json",
    }), { status: 200, headers: { "content-type": "application/json" } }));
    await expect(discoverOidcProvider(config.oidc, fetcher)).resolves.toMatchObject({
      authorizationEndpoint: "https://identity.example.test/oauth2/authorize",
    });
    expect(fetcher).toHaveBeenCalledWith("https://identity.example.test/.well-known/openid-configuration", expect.any(Object));
  });

  it("creates a redirect URL only after the transaction is durably staged", async () => {
    if (config.mode !== "oidc" || !config.oidc) throw new Error("test OIDC config did not parse");
    const storage = memoryStorage();
    const fetcher = vi.fn(async () => new Response(JSON.stringify({
      issuer: "https://identity.example.test/",
      authorization_endpoint: "https://identity.example.test/oauth2/authorize",
      token_endpoint: "https://identity.example.test/oauth2/token",
      jwks_uri: "https://identity.example.test/.well-known/jwks.json",
    }), { status: 200 }));
    const redirect = await beginOidcAuthorization(config.oidc, storage, fetcher, 1_800_000_000);
    expect(new URL(redirect).pathname).toBe("/oauth2/authorize");
    expect(storage.getItem("solana-tax.oidc.pkce.v1")).toContain("codeVerifier");
  });

  it("exchanges a validated callback once and rejects a second callback", async () => {
    if (config.mode !== "oidc" || !config.oidc) throw new Error("test OIDC config did not parse");
    const storage = memoryStorage();
    const transaction = createPkceTransaction(1_800_000_000, config.oidc.redirectUri);
    storePkceTransaction(storage, transaction);
    const { privateKey, publicKey } = await generateKeyPair("RS256");
    const jwk = await exportJWK(publicKey);
    jwk.kid = "key-1";
    jwk.alg = "RS256";
    const idToken = await new SignJWT({ sub: "operator-1", nonce: transaction.nonce })
      .setProtectedHeader({ alg: "RS256", typ: "JWT", kid: "key-1" })
      .setIssuer("https://identity.example.test/")
      .setAudience("tax-console")
      .setIssuedAt(1_800_000_000)
      .setExpirationTime(1_800_000_300)
      .sign(privateKey);
    const fetcher = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
      const url = String(input);
      if (url.endsWith("openid-configuration")) return new Response(JSON.stringify({
        issuer: "https://identity.example.test/",
        authorization_endpoint: "https://identity.example.test/oauth2/authorize",
        token_endpoint: "https://identity.example.test/oauth2/token",
        jwks_uri: "https://identity.example.test/.well-known/jwks.json",
      }), { status: 200 });
      if (url.endsWith("jwks.json")) return new Response(JSON.stringify({ keys: [jwk] }), { status: 200 });
      expect(url).toBe("https://identity.example.test/oauth2/token");
      expect(init?.method).toBe("POST");
      expect(String(init?.body)).toContain("grant_type=authorization_code");
      expect(String(init?.body)).toContain("client_id=tax-console");
      return new Response(JSON.stringify({ access_token: "opaque", token_type: "Bearer", expires_in: 300, id_token: idToken }), { status: 200 });
    });
    await expect(completeOidcCallback(config.oidc, new URL(`https://console.example.test/auth/callback?code=one&state=${transaction.state}`), storage, fetcher, 1_800_000_001)).resolves.toEqual({ accessToken: "opaque", expiresAtUnix: 1_800_000_301 });
    await expect(completeOidcCallback(config.oidc, new URL(`https://console.example.test/auth/callback?code=one&state=${transaction.state}`), storage, fetcher, 1_800_000_002)).rejects.toThrow(/expired|start again|transaction/i);
  });

  it("does not establish a session when the returned ID token nonce is wrong", async () => {
    if (config.mode !== "oidc" || !config.oidc) throw new Error("test OIDC config did not parse");
    const storage = memoryStorage();
    const transaction = createPkceTransaction(1_800_000_000, config.oidc.redirectUri);
    storePkceTransaction(storage, transaction);
    const { privateKey, publicKey } = await generateKeyPair("RS256");
    const jwk = await exportJWK(publicKey);
    jwk.kid = "key-1";
    jwk.alg = "RS256";
    const idToken = await new SignJWT({ sub: "operator-1", nonce: "wrong-nonce" })
      .setProtectedHeader({ alg: "RS256", typ: "JWT", kid: "key-1" })
      .setIssuer("https://identity.example.test/")
      .setAudience("tax-console")
      .setIssuedAt(1_800_000_000)
      .setExpirationTime(1_800_000_300)
      .sign(privateKey);
    const fetcher = vi.fn(async (input: RequestInfo | URL) => {
      const url = String(input);
      if (url.endsWith("openid-configuration")) return new Response(JSON.stringify({ issuer: "https://identity.example.test/", authorization_endpoint: "https://identity.example.test/oauth2/authorize", token_endpoint: "https://identity.example.test/oauth2/token", jwks_uri: "https://identity.example.test/.well-known/jwks.json" }), { status: 200 });
      if (url.endsWith("jwks.json")) return new Response(JSON.stringify({ keys: [jwk] }), { status: 200 });
      return new Response(JSON.stringify({ access_token: "opaque", token_type: "Bearer", expires_in: 300, id_token: idToken }), { status: 200 });
    });
    await expect(completeOidcCallback(config.oidc, new URL(`https://console.example.test/auth/callback?code=one&state=${transaction.state}`), storage, fetcher, 1_800_000_001)).rejects.toThrow(/nonce/i);
  });
});
