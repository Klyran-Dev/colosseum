import { describe, expect, it } from "vitest";
import {
  buildSettlementAdjustmentUrl,
  nextAdjustmentStatuses,
  parseSettlementAdjustment,
  validateAdjustmentDelta,
  validateByteArray,
} from "./settlementAdjustments";

const validRecord = {
  id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d13",
  merchant_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d11",
  idempotency_key: "refund-01",
  request_digest: Array(32).fill(1),
  adjustment_digest: Array(32).fill(2),
  original_settlement_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d14",
  asset_id: Array(32).fill(3),
  kind: "Refund",
  delta: { gross_atomic: "-250", tax_atomic: "-25", net_atomic: "-225" },
  reason_code: "customer_refund",
  evidence_digest: Array(32).fill(4),
  source_calculation_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d15",
  replacement_calculation_id: null,
  status: "proposed",
  proposer_subject: "proposer",
  approver_subject: null,
};

describe("settlement adjustment UI contract", () => {
  it("parses a server-owned record without coercing financial strings", () => {
    const record = parseSettlementAdjustment(validRecord);
    expect(record.delta.gross_atomic).toBe("-250");
    expect(record.asset_id).toHaveLength(32);
    expect(record.status).toBe("proposed");
  });

  it("rejects unsafe numeric financial fields and malformed byte arrays", () => {
    expect(() => parseSettlementAdjustment({ ...validRecord, delta: { gross_atomic: -250, tax_atomic: "-25", net_atomic: "-225" } })).toThrow(/decimal string/i);
    expect(() => validateByteArray([0, 1, 256], "asset")).toThrow(/32 bytes/i);
    expect(() => validateByteArray(Array(32).fill(0), "asset")).toThrow(/non-zero/i);
  });

  it("enforces exact conservation and negative-kind semantics", () => {
    expect(() => validateAdjustmentDelta("Refund", { gross_atomic: "-100", tax_atomic: "-20", net_atomic: "-70" })).toThrow(/conserv/i);
    expect(() => validateAdjustmentDelta("Refund", { gross_atomic: "100", tax_atomic: "0", net_atomic: "100" })).toThrow(/non-positive/i);
    expect(() => validateAdjustmentDelta("Correction", { gross_atomic: "0", tax_atomic: "0", net_atomic: "0" })).toThrow(/non-zero/i);
    expect(() => validateAdjustmentDelta("Correction", { gross_atomic: "100", tax_atomic: "1", net_atomic: "99" })).not.toThrow();
  });

  it("exposes only the governed lifecycle and builds tenant-neutral URLs", () => {
    expect(nextAdjustmentStatuses("proposed")).toEqual(["approved", "rejected"]);
    expect(nextAdjustmentStatuses("approved")).toEqual(["voided"]);
    expect(nextAdjustmentStatuses("rejected")).toEqual([]);
    expect(nextAdjustmentStatuses("posted")).toEqual([]);
    expect(buildSettlementAdjustmentUrl("/api/", validRecord.id)).toBe(`/api/v1/settlement/adjustments/${validRecord.id}`);
  });
});
