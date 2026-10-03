import { generateKeyPair, exportJWK, SignJWT } from "jose";
import { describe, expect, it, vi } from "vitest";
import { validateOidcIdToken } from "./oidcIdToken";

const metadata = {
  issuer: "https://identity.example.test/",
  authorizationEndpoint: "https://identity.example.test/oauth2/authorize",
  tokenEndpoint: "https://identity.example.test/oauth2/token",
  jwksUri: "https://identity.example.test/.well-known/jwks.json",
};

async function signedToken(overrides: Record<string, unknown> = {}): Promise<{ token: string; jwks: Record<string, unknown> }> {
  const { privateKey, publicKey } = await generateKeyPair("RS256");
  const jwk = await exportJWK(publicKey);
  jwk.kid = "key-1";
  jwk.alg = "RS256";
  const now = 1_800_000_000;
  const issuer = typeof overrides.issuer === "string" ? overrides.issuer : metadata.issuer;
  const audience = typeof overrides.audience === "string" ? overrides.audience : "tax-console";
  const { issuer: _issuer, audience: _audience, ...claims } = overrides;
  const token = await new SignJWT({
    sub: "operator-1",
    nonce: "nonce-1",
    ...claims,
  }).setProtectedHeader({ alg: "RS256", typ: "JWT", kid: "key-1" })
    .setIssuer(issuer)
    .setAudience(audience)
    .setIssuedAt(now)
    .setExpirationTime(now + 300)
    .sign(privateKey);
  return { token, jwks: { keys: [jwk] } };
}

function response(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), { status, headers: { "content-type": "application/json" } });
}

describe("OIDC ID-token validation boundary", () => {
  it("accepts a signed token with the configured issuer, client, nonce, and bounded time claims", async () => {
    const { token, jwks } = await signedToken();
    const fetcher = vi.fn(async () => response(jwks));
    await expect(validateOidcIdToken(token, metadata, { clientId: "tax-console", nonce: "nonce-1", nowUnix: 1_800_000_001 }, fetcher)).resolves.toMatchObject({ subject: "operator-1" });
    expect(fetcher).toHaveBeenCalledTimes(1);
  });

  it.each([
    ["issuer", { issuer: "https://other.example.test/" }, /iss|issuer/i],
    ["audience", { audience: "other-client" }, /aud|audience/i],
    ["nonce", { nonce: "wrong-nonce" }, /nonce/i],
    ["subject", { sub: "" }, /subject/i],
  ])("rejects a token with an invalid %s", async (_name, override, expected) => {
    const { token, jwks } = await signedToken(override);
    await expect(validateOidcIdToken(token, metadata, { clientId: "tax-console", nonce: "nonce-1", nowUnix: 1_800_000_001 }, async () => response(jwks))).rejects.toThrow(expected);
  });

  it("rejects expired tokens and unsupported signing algorithms", async () => {
    const { token, jwks } = await signedToken();
    await expect(validateOidcIdToken(token, metadata, { clientId: "tax-console", nonce: "nonce-1", nowUnix: 1_800_000_400 }, async () => response(jwks))).rejects.toThrow(/expired|timestamp|time/i);
    const unsigned = `${token.split(".")[0]}.${token.split(".")[1]}.`;
    await expect(validateOidcIdToken(unsigned, metadata, { clientId: "tax-console", nonce: "nonce-1", nowUnix: 1_800_000_001 }, async () => response(jwks))).rejects.toThrow(/algorithm|signature|token/i);
  });

  it("fails closed for malformed or oversized key sets", async () => {
    const { token } = await signedToken();
    await expect(validateOidcIdToken(token, metadata, { clientId: "tax-console", nonce: "nonce-1", nowUnix: 1_800_000_001 }, async () => response({ keys: [{ kty: "RSA", kid: "key-1" }, { kty: "RSA", kid: "key-1" }] }))).rejects.toThrow(/duplicate|key|JWKS/i);
    await expect(validateOidcIdToken(token, metadata, { clientId: "tax-console", nonce: "nonce-1", nowUnix: 1_800_000_001 }, async () => new Response("x".repeat(70_000), { status: 200 }))).rejects.toThrow(/large|size/i);
  });

  it("refreshes the key set once when the signing key has rotated, then stops", async () => {
    const first = await signedToken();
    const second = await signedToken();
    const fetcher = vi.fn()
      .mockResolvedValueOnce(response(first.jwks))
      .mockResolvedValueOnce(response(second.jwks));
    await expect(validateOidcIdToken(second.token, metadata, { clientId: "tax-console", nonce: "nonce-1", nowUnix: 1_800_000_001 }, fetcher)).resolves.toMatchObject({ subject: "operator-1" });
    expect(fetcher).toHaveBeenCalledTimes(2);
  });
});
