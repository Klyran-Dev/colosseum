import { describe, expect, it } from "vitest";
import { parseBrowserAuthConfig } from "./authConfig";
import {
  buildAuthorizationUrl,
  createPkceTransaction,
  parseTokenResponse,
  pkceCodeChallenge,
  validateCallback,
  validateProviderMetadata,
} from "./oidcPkce";

const config = parseBrowserAuthConfig({
  VITE_DEPLOYMENT_ENVIRONMENT: "staging",
  VITE_OIDC_ISSUER: "https://identity.example.test/",
  VITE_OIDC_CLIENT_ID: "tax-console",
  VITE_OIDC_REDIRECT_URI: "https://console.example.test/auth/callback",
  VITE_OIDC_SCOPE: "openid profile",
  VITE_OIDC_AUDIENCE: "tax-api",
});

describe("OIDC Authorization Code + PKCE boundary", () => {
  it("generates bounded CSPRNG transaction values and the RFC 7636 challenge", async () => {
    const transaction = createPkceTransaction(1_800_000_000);
    expect(transaction.state).toMatch(/^[A-Za-z0-9_-]{43}$/);
    expect(transaction.nonce).toMatch(/^[A-Za-z0-9_-]{43}$/);
    expect(transaction.codeVerifier).toMatch(/^[A-Za-z0-9_-]{43}$/);
    expect(transaction.createdAtUnix).toBe(1_800_000_000);
    await expect(pkceCodeChallenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk")).resolves.toBe("E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
  });

  it("builds an explicit authorization request without a client secret", async () => {
    if (config.mode !== "oidc" || !config.oidc) throw new Error("test OIDC config did not parse");
    const transaction = createPkceTransaction(1_800_000_000);
    const challenge = await pkceCodeChallenge(transaction.codeVerifier);
    const authorizationUrl = buildAuthorizationUrl(config.oidc, {
      issuer: "https://identity.example.test/",
      authorizationEndpoint: "https://identity.example.test/oauth2/authorize",
      tokenEndpoint: "https://identity.example.test/oauth2/token",
      jwksUri: "https://identity.example.test/.well-known/jwks.json",
    }, transaction, challenge);
    const parsed = new URL(authorizationUrl);

    expect(parsed.searchParams.get("response_type")).toBe("code");
    expect(parsed.searchParams.get("client_id")).toBe("tax-console");
    expect(parsed.searchParams.get("code_challenge_method")).toBe("S256");
    expect(parsed.searchParams.get("code_challenge")).toBe(challenge);
    expect(parsed.searchParams.get("audience")).toBe("tax-api");
    expect(parsed.searchParams.get("client_secret")).toBeNull();
  });

  it("rejects provider metadata substitution and callback state mismatch", () => {
    if (config.mode !== "oidc" || !config.oidc) throw new Error("test OIDC config did not parse");
    const oidc = config.oidc;
    expect(() => validateProviderMetadata(oidc, {
      issuer: "https://evil.example.test/",
      authorizationEndpoint: "https://evil.example.test/authorize",
      tokenEndpoint: "https://evil.example.test/token",
      jwksUri: "https://evil.example.test/.well-known/jwks.json",
    })).toThrow(/issuer/i);
    expect(() => validateProviderMetadata(oidc, {
      issuer: "https://identity.example.test/",
      authorizationEndpoint: "https://login.example.test/authorize",
      tokenEndpoint: "https://identity.example.test/token",
      jwksUri: "https://identity.example.test/.well-known/jwks.json",
    })).toThrow(/unsafe|origin/i);
    const transaction = createPkceTransaction(1_800_000_000);
    expect(() => validateCallback(transaction, new URL("https://console.example.test/auth/callback?code=abc&state=wrong"), 1_800_000_001)).toThrow(/state/i);
    expect(() => validateCallback(transaction, new URL("https://console.example.test/auth/callback?error=access_denied&state=" + transaction.state), 1_800_000_001)).toThrow(/denied/i);
  });

  it("accepts only a bounded bearer token response and never accepts a refresh token", () => {
    expect(parseTokenResponse({ access_token: "opaque-access-token", token_type: "Bearer", expires_in: 300 }, 1_800_000_000)).toEqual({
      accessToken: "opaque-access-token",
      expiresAtUnix: 1_800_000_300,
    });
    expect(() => parseTokenResponse({ access_token: "opaque", token_type: "Bearer", expires_in: 300, refresh_token: "secret" }, 1_800_000_000)).toThrow(/refresh/i);
    expect(() => parseTokenResponse({ access_token: "opaque", token_type: "bearer", expires_in: 300 }, 1_800_000_000)).toThrow(/token response/i);
    expect(() => parseTokenResponse({ access_token: "opaque", token_type: "Bearer", expires_in: 0 }, 1_800_000_000)).toThrow(/expiry/i);
  });
});
