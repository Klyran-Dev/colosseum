import { describe, expect, it } from "vitest";
import {
  GOVERNANCE_ROLES,
  nextGovernanceStatuses,
  parseCreatedRuleDraft,
  parseGovernanceState,
  transitionLabel,
} from "./governance";

describe("rule governance operator boundary", () => {
  it("exposes only the server-defined next lifecycle action", () => {
    expect(nextGovernanceStatuses("draft")).toEqual(["simulated"]);
    expect(nextGovernanceStatuses("approved")).toEqual(["published"]);
    expect(nextGovernanceStatuses("published")).toEqual(["retired"]);
    expect(nextGovernanceStatuses("retired")).toEqual([]);
    expect(transitionLabel("reviewed", "approved")).toBe("Approve for publication");
  });

  it("parses a complete governance snapshot without inventing defaults", () => {
    const state = parseGovernanceState({
      rule_set_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d13",
      external_id: "sales-tax",
      version: 3,
      jurisdiction: "US-CA",
      regime: "sales-tax",
      status: "reviewed",
      rule_digest_hex: "ab".repeat(32),
      author_subject: "author",
      reviewer_subject: "reviewer",
      approver_subject: null,
      governance_revision: 2,
      effective_from: "2026-09-26 00:00:00+00",
      effective_until: null,
      events: [],
    });
    expect(state.status).toBe("reviewed");
    expect(state.governance_revision).toBe(2);
    expect(state.approver_subject).toBeNull();
    expect(GOVERNANCE_ROLES).toContain("tax_rule_auditor");
  });

  it("rejects malformed snapshots instead of rendering fabricated governance state", () => {
    expect(() => parseGovernanceState({ status: "published" })).toThrow(/governance snapshot/i);
    expect(() => parseGovernanceState({
      rule_set_id: "not-a-uuid",
      external_id: "sales-tax",
      version: 1,
      jurisdiction: "US-CA",
      regime: "sales-tax",
      status: "draft",
      rule_digest_hex: "00".repeat(32),
      author_subject: null,
      reviewer_subject: null,
      approver_subject: null,
      governance_revision: 0,
      effective_from: "now",
      effective_until: null,
      events: [],
    })).toThrow(/governance snapshot/i);
  });

  it("validates the server-owned draft creation response", () => {
    const draft = parseCreatedRuleDraft({
      rule_set_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d15",
      external_id: "sales-tax",
      version: 1,
      jurisdiction: "US-CA",
      regime: "sales-tax",
      status: "draft",
      rule_digest: Array.from({ length: 32 }, (_, index) => index),
      effective_from: "2026-09-27T00:00:00.000Z",
      effective_until: null,
      governance_revision: 0,
    });
    expect(draft.status).toBe("draft");
    expect(() => parseCreatedRuleDraft({
      ...draft,
      rule_digest: [1, 2],
    })).toThrow(/malformed/i);
  });
});
