import { describe, expect, it } from "vitest";
import {
  buildReplayGapActionUrl,
  buildReplayGapListUrl,
  buildTargetedRecoveryUrl,
  parseTargetedRecoveryResult,
  parseReplayGapList,
  validateRecoveryTarget,
  validateOperatorNote,
} from "./replayOps";

const id = "01992f1b-4a70-7e30-8ea2-b6e37a9d9d13";

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
