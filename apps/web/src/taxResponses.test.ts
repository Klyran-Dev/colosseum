import { describe, expect, it } from "vitest";
import { parseCalculationResponse, parseReplayVerificationResponse } from "./taxResponses";

const calculationId = "01992f1b-4a70-7e30-8ea2-b6e37a9d9d21";

function validCalculation(overrides: Record<string, unknown> = {}): Record<string, unknown> {
  return {
    calculation_id: calculationId,
    status: "calculated_unattested",
    expires_at_unix: 1_800_000_300,
    result: {
      rule_set_id: "published-sales-tax",
      rule_version: 3,
      engine_version: "tax-engine/v1",
      currency: "USD",
      jurisdiction: "US-CA",
      facts_digest: Array(32).fill(1),
      rule_digest: Array(32).fill(2),
      gross_atomic: "100000000000000000000",
      total_tax_atomic: "-25",
      total_net_atomic: "99999999999999999975",
      components: [{
        id: "sales-tax",
        label: "Sales tax",
        base_atomic: "100000000000000000000",
        effective_base_atomic: "100000000000000000000",
        threshold_atomic: null,
        exemption_applied: false,
        raw_tax: { numerator: "-25", denominator: "1" },
        credit_atomic: "0",
        clamped_tax_atomic: "-25",
        tax_atomic: "-25",
        explanation: "Published rule evidence",
      }],
    },
    ...overrides,
  };
}

describe("strict tax evidence responses", () => {
  it("preserves signed decimal atomic values without number conversion", () => {
    const parsed = parseCalculationResponse(validCalculation());
    expect(parsed.calculation_id).toBe(calculationId);
    expect(parsed.result.gross_atomic).toBe("100000000000000000000");
    expect(parsed.result.total_tax_atomic).toBe("-25");
    expect(parsed.result.components[0].tax_atomic).toBe("-25");
  });

  it("rejects numeric or non-canonical atomic response values", () => {
    expect(() => parseCalculationResponse(validCalculation({
      result: { ...validCalculation().result as Record<string, unknown>, gross_atomic: 100 },
    }))).toThrow(/gross_atomic/);
    expect(() => parseCalculationResponse(validCalculation({
      result: { ...validCalculation().result as Record<string, unknown>, gross_atomic: "01" },
    }))).toThrow(/gross_atomic/);
  });

  it("validates and preserves exact FX evidence", () => {
    const parsed = parseCalculationResponse(validCalculation({
      result: {
        ...validCalculation().result as Record<string, unknown>,
        currency: "EUR",
        fx_conversion: {
          source_currency: "USD",
          target_currency: "EUR",
          numerator: "3",
          denominator: "2",
          rounding_mode: "HalfUp",
          quantum_atomic: "1",
          provider: "regulated-fx",
          quote_id: "quote-1",
          observed_at_unix: 1_759_000_000,
          evidence_digest: Array(32).fill(7),
        },
      },
    }));
    expect(parsed.result.fx_conversion?.target_currency).toBe("EUR");
    expect(parsed.result.fx_conversion?.numerator).toBe("3");
    expect(() => parseCalculationResponse(validCalculation({
      result: {
        ...validCalculation().result as Record<string, unknown>,
        currency: "EUR",
        fx_conversion: {
          source_currency: "USD",
          target_currency: "USD",
          numerator: "3",
          denominator: "2",
          rounding_mode: "HalfUp",
          quantum_atomic: "1",
          provider: "regulated-fx",
          quote_id: "quote-1",
          observed_at_unix: 1_759_000_000,
          evidence_digest: Array(32).fill(7),
        },
      },
    }))).toThrow(/currencies must differ/);
  });

  it("rejects unsupported statuses, malformed identities, and invalid digests", () => {
    expect(() => parseCalculationResponse(validCalculation({ status: "finalized" }))).toThrow(/status/);
    expect(() => parseCalculationResponse(validCalculation({ calculation_id: "01992f1b-4a70-6e30-8ea2-b6e37a9d9d21" }))).toThrow(/UUIDv7/);
    expect(() => parseCalculationResponse(validCalculation({
      result: { ...validCalculation().result as Record<string, unknown>, rule_digest: [1, 2] },
    }))).toThrow(/rule digest/);
  });

  it("requires the exact replay verification envelope", () => {
    const parsed = parseReplayVerificationResponse({
      status: "replay_verified",
      calculation_id: calculationId,
      engine_version: "tax-engine/v1",
      facts_digest: Array(32).fill(1),
      rule_digest: Array(32).fill(2),
    });
    expect(parsed.status).toBe("replay_verified");
    expect(() => parseReplayVerificationResponse({
      status: "replay_verified",
      calculation_id: calculationId,
      engine_version: "",
      facts_digest: Array(32).fill(1),
      rule_digest: Array(32).fill(2),
    })).toThrow(/engine version/);
  });
});
