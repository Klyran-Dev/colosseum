import { describe, expect, it } from "vitest";
import {
  MAX_EVIDENCE_ACCESS_AUDIT_LIST_LIMIT,
  buildEvidenceAccessAuditUrl,
  parseEvidenceAccessAuditList,
} from "./evidenceAccess";

const id = "0199d6f4-2b2a-7c31-8c6d-8b0b1b5c9012";

function event(overrides: Record<string, unknown> = {}) {
  return {
    id,
    calculation_id: "0199d6f4-2b2a-7c31-8c6d-8b0b1b5c9013",
    actor_subject: "operator@example.test",
    request_id: "0199d6f4-2b2a-7c31-8c6d-8b0b1b5c9014",
    purpose: "tax_quote_replay",
    outcome: "allowed",
    facts_digest: Array.from({ length: 32 }, (_, index) => index),
    occurred_at: "2026-10-05T10:00:00.000Z",
    ...overrides,
  };
}

describe("evidence-access explorer contract", () => {
  it("parses a redacted UUIDv7 audit event and preserves the digest commitment", () => {
    const [parsed] = parseEvidenceAccessAuditList([event()]);
    expect(parsed.id).toBe(id);
    expect(parsed.facts_digest).toHaveLength(32);
    expect(parsed.outcome).toBe("allowed");
  });

  it("rejects malformed identity, digest, timestamp, and oversized responses", () => {
    expect(() => parseEvidenceAccessAuditList([event({ id: "not-a-uuid" })])).toThrow(/malformed/i);
    expect(() => parseEvidenceAccessAuditList([event({ facts_digest: [1, 2] })])).toThrow(/malformed/i);
    expect(() => parseEvidenceAccessAuditList([event({ occurred_at: "not-a-time" })])).toThrow(/malformed/i);
    expect(() => parseEvidenceAccessAuditList(Array.from({ length: MAX_EVIDENCE_ACCESS_AUDIT_LIST_LIMIT + 1 }, () => event()))).toThrow(/malformed/i);
  });

  it("builds a bounded tenant-scoped read URL without merchant identifiers", () => {
    expect(buildEvidenceAccessAuditUrl("/api/", 25, "0199d6f4-2b2a-7c31-8c6d-8b0b1b5c9013"))
      .toBe("/api/v1/audit/evidence-access?limit=25&calculation_id=0199d6f4-2b2a-7c31-8c6d-8b0b1b5c9013");
    expect(() => buildEvidenceAccessAuditUrl("/api", 0)).toThrow(/out of bounds/i);
    expect(() => buildEvidenceAccessAuditUrl("/api", MAX_EVIDENCE_ACCESS_AUDIT_LIST_LIMIT + 1)).toThrow(/out of bounds/i);
  });
});
