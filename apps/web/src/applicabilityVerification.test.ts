import { describe, expect, it } from "vitest";
import { buildApplicabilityVerificationUrl, parseApplicabilityVerification } from "./applicabilityVerification";

const intentId = "0198f18a-7b4b-7d44-9a0f-4e4f5f2c1a11";

describe("applicability verification contract", () => {
  it("parses every explicit lifecycle state without inventing finality", () => {
    const result = parseApplicabilityVerification({ settlement_intent_id: intentId, status: "retrying", signature: null, event_index: 0, event_slot: "42", provider_slot: null, receipt: null, binding: null, merchant: null, mint: null, context_id_hex: null, attestation_digest_hex: null, event_evidence_digest_hex: null, verification_commitment: null, attempts: 2, last_error_code: "provider_transport_failed", verified_at: null, claimed_at: "2026-10-06T00:00:00.000Z", dead_lettered_at: null });
    expect(result.status).toBe("retrying");
    expect(result.provider_slot).toBeNull();
  });

  it("builds a bounded server endpoint URL and rejects non-UUIDv7 intents", () => {
    expect(buildApplicabilityVerificationUrl("/api/", intentId, "local/settlement")).toContain("stream_key=local%2Fsettlement");
    expect(() => buildApplicabilityVerificationUrl("/api", "00000000-0000-4000-8000-000000000000", "local/settlement")).toThrow();
  });
});
