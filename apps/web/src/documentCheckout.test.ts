import { describe, expect, it } from "vitest";
import {
  parseDocumentAttestationJson,
  parseDocumentAttestationVerification,
  parseDocumentScopeLineIds,
} from "./documentCheckout";

describe("document checkout boundary", () => {
  it("keeps the pasted attestation untrusted JSON until the API verifies it", () => {
    expect(parseDocumentAttestationJson('{"unsigned":{},"signature":[]}')).toEqual({ unsigned: {}, signature: [] });
    expect(() => parseDocumentAttestationJson("[]")).toThrow(/JSON object/);
    expect(() => parseDocumentAttestationJson("not-json")).toThrow(/valid JSON/);
  });

  it("accepts only the server-owned attested-unsettled response", () => {
    const response = parseDocumentAttestationVerification({
      status: "document_attested_unsettled",
      calculation_id: "018f2d20-7f18-7abc-8c9a-9d2e2e1f1a11",
      payload_digest: new Array(32).fill(8),
      issuer_public_key: new Array(32).fill(9),
      expires_at_unix: 1_800_000_000,
    });
    expect(response.calculation_id).toBe("018f2d20-7f18-7abc-8c9a-9d2e2e1f1a11");
    expect(() => parseDocumentAttestationVerification({ status: "verified" })).toThrow(/active/);
  });

  it("canonicalizes bounded line scopes and rejects duplicates", () => {
    expect(parseDocumentScopeLineIds("line-c, line-a")).toEqual(["line-a", "line-c"]);
    expect(() => parseDocumentScopeLineIds("line-a,line-a")).toThrow(/unique/);
    expect(() => parseDocumentScopeLineIds(" ")).toThrow(/between/);
  });
});
