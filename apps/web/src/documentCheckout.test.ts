import { describe, expect, it } from "vitest";
import {
  buildDocumentAttestationStatusUrl,
  parseDocumentAttestationStatus,
  parseDocumentScopeLineIds,
} from "./documentCheckout";

describe("document checkout boundary", () => {
  it("accepts only a bounded server-owned evidence status response", () => {
    const response = parseDocumentAttestationStatus({
      status: "verified",
      calculation_id: "018f2d20-7f18-7abc-8c9a-9d2e2e1f1a11",
      payload_digest: new Array(32).fill(8),
      issuer_public_key: new Array(32).fill(9),
      expires_at_unix: 1_800_000_000,
      checked_at_unix: 1_799_999_900,
    });
    expect(response.status).toBe("verified");
    expect(response.checked_at_unix).toBe(1_799_999_900);
    expect(() => parseDocumentAttestationStatus({ status: "verified" })).toThrow(/calculation_id/);
    expect(() => parseDocumentAttestationStatus({
      status: "verified",
      calculation_id: "018f2d20-7f18-7abc-8c9a-9d2e2e1f1a11",
      payload_digest: [],
      issuer_public_key: new Array(32).fill(9),
      expires_at_unix: 1_800_000_000,
      checked_at_unix: 1_799_999_900,
    })).toThrow(/payload.*digest/);
  });

  it("builds a route-bound status URL without accepting protocol fields", () => {
    expect(buildDocumentAttestationStatusUrl(
      "/api/",
      "018f2d20-7f18-7abc-8c9a-9d2e2e1f1a11",
      "018f2d20-7f18-7abc-8c9a-9d2e2e1f1a12",
    )).toBe("/api/v1/tax/document-quotes/018f2d20-7f18-7abc-8c9a-9d2e2e1f1a11/attestation/status?route_id=018f2d20-7f18-7abc-8c9a-9d2e2e1f1a12");
    expect(() => buildDocumentAttestationStatusUrl(
      "/api",
      "018f2d20-7f18-7abc-8c9a-9d2e2e1f1a11",
      "not-a-route",
    )).toThrow(/UUIDv7/);
  });

  it("canonicalizes bounded line scopes and rejects duplicates", () => {
    expect(parseDocumentScopeLineIds("line-c, line-a")).toEqual(["line-a", "line-c"]);
    expect(() => parseDocumentScopeLineIds("line-a,line-a")).toThrow(/unique/);
    expect(() => parseDocumentScopeLineIds(" ")).toThrow(/between/);
  });
});
