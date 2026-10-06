import { describe, expect, it } from "vitest";
import {
  buildReplayGapActionUrl,
  buildReplayGapListUrl,
  buildTargetedRecoveryUrl,
  buildRouteVerificationRunsUrl,
  parseTargetedRecoveryResult,
  parseReplayGapList,
  parseRouteVerificationRunList,
  validateRecoveryTarget,
  validateOperatorNote,
} from "./replayOps";

const id = "01992f1b-4a70-7e30-8ea2-b6e37a9d9d13";

function verificationRun(overrides: Record<string, unknown> = {}) {
  return {
    id,
    run_kind: "backfill",
    status: "completed",
    batch_size: 8,
    examined_count: 2,
    activated_count: 1,
    already_active_count: 1,
    failed_count: 0,
    failure_code: null,
    operator_approval_id: "01992f1b-4a70-7e31-8ea2-b6e37a9d9d13",
    started_at: "2026-10-05T10:00:00.000Z",
    completed_at: "2026-10-05T10:00:01.000Z",
    ...overrides,
  };
}

function gap(overrides: Record<string, unknown> = {}) {
  return {
    id,
    stream_key: "mainnet/settlement",
    start_slot: 900,
    end_slot: 901,
    observed_finalized_slot: 901,
    reason: "provider_disagreement",
    attempts: 8,
    status: "dead_lettered",
    last_error_code: "FINALITY_DISAGREEMENT",
    created_at: "2026-09-26T10:00:00.000Z",
    dead_lettered_at: "2026-09-26T10:01:00.000Z",
    operator_acknowledged_at: null,
    operator_acknowledged_by: null,
    operator_acknowledgement: null,
    ...overrides,
  };
}

describe("replay-gap operator contract", () => {
  it("parses a bounded server projection and preserves slot precision as strings", () => {
    const [parsed] = parseReplayGapList([gap({ start_slot: "9007199254740991", end_slot: "9007199254740992", observed_finalized_slot: "9007199254740992" })]);
    expect(parsed.start_slot).toBe("9007199254740991");
    expect(parsed.status).toBe("dead_lettered");
  });

  it("rejects unsafe numeric slots and inconsistent acknowledgement evidence", () => {
    expect(() => parseReplayGapList([gap({ start_slot: Number.MAX_SAFE_INTEGER + 1 })])).toThrow(/malformed/i);
    expect(() => parseReplayGapList([gap({ operator_acknowledged_at: "2026-09-26T10:02:00.000Z" })])).toThrow(/malformed/i);
    expect(() => parseReplayGapList([gap({ end_slot: 902 })])).toThrow(/malformed/i);
    expect(() => parseReplayGapList([gap({ dead_lettered_at: null })])).toThrow(/malformed/i);
  });

  it("bounds note input before a mutation can be sent", () => {
    expect(validateOperatorNote("incident INC-1 reviewed")).toBe("incident INC-1 reviewed");
    expect(() => validateOperatorNote(" ")).toThrow(/note/i);
    expect(() => validateOperatorNote("x".repeat(1025))).toThrow(/note/i);
    expect(validateOperatorNote("💸".repeat(256))).toHaveLength(512);
    expect(() => validateOperatorNote("💸".repeat(257))).toThrow(/UTF-8/i);
  });

  it("builds encoded list and action URLs without accepting arbitrary paths", () => {
    expect(buildReplayGapListUrl("/api/", "mainnet/settlement", "dead_lettered", 25))
      .toBe("/api/v1/indexer/replay-gaps?stream_key=mainnet%2Fsettlement&status=dead_lettered&limit=25");
    expect(buildReplayGapActionUrl("/api", id, "acknowledge"))
      .toBe(`/api/v1/indexer/replay-gaps/${id}/acknowledge`);
    expect(() => buildReplayGapActionUrl("/api", "not-a-uuid", "requeue")).toThrow(/gap/i);
  });

  it("validates targeted recovery responses without coercing unsafe slots", () => {
    const result = parseTargetedRecoveryResult({
      status: "recovered",
      audit_id: id,
      signature: "5".repeat(88),
      pages_scanned: 2,
      slot: "9007199254740993",
      raw_transaction_count: 1,
      raw_evidence_count: 1,
      receipt_count: 1,
    });
    expect(result.slot).toBe("9007199254740993");
    expect(() => parseTargetedRecoveryResult({ status: "recovered", audit_id: id, signature: "sig", pages_scanned: 65, slot: "1", raw_transaction_count: 1, raw_evidence_count: 1, receipt_count: 1 })).toThrow(/malformed/i);
    expect(() => validateRecoveryTarget(" ", "Address")).toThrow(/bounded/i);
    expect(buildTargetedRecoveryUrl("/api/")).toBe("/api/v1/indexer/targeted-recovery");
  });
});

describe("route-verification operator evidence contract", () => {
  it("parses bounded terminal evidence and preserves approval identity", () => {
    const [parsed] = parseRouteVerificationRunList([verificationRun()]);
    expect(parsed.id).toBe(id);
    expect(parsed.operator_approval_id).toBe("01992f1b-4a70-7e31-8ea2-b6e37a9d9d13");
    expect(parsed.activated_count + parsed.already_active_count + parsed.failed_count).toBe(parsed.examined_count);
  });

  it("rejects malformed lifecycle evidence, unsafe counts, and approval drift", () => {
    expect(() => parseRouteVerificationRunList([verificationRun({ failed_count: 1 })])).toThrow(/malformed/i);
    expect(() => parseRouteVerificationRunList([verificationRun({ status: "completed_with_failures", examined_count: 2, activated_count: 1, already_active_count: 0, failed_count: 1, failure_code: "route_attempts_failed" })])).not.toThrow();
    expect(() => parseRouteVerificationRunList([verificationRun({ status: "backfill", run_kind: "scheduled", operator_approval_id: null })])).toThrow(/malformed/i);
    expect(() => parseRouteVerificationRunList([verificationRun({ batch_size: Number.MAX_SAFE_INTEGER + 1 })])).toThrow(/malformed/i);
    expect(() => parseRouteVerificationRunList([verificationRun({ status: "running", completed_at: "2026-10-05T10:00:01.000Z" })])).toThrow(/malformed/i);
  });

  it("bounds the explicit read request and never accepts a route or merchant selector", () => {
    expect(buildRouteVerificationRunsUrl("/api/", 20)).toBe("/api/v1/indexer/route-verification-runs?limit=20");
    expect(() => buildRouteVerificationRunsUrl("/api", 51)).toThrow(/out of bounds/i);
  });
});
