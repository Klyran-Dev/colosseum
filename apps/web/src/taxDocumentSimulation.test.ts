import { describe, expect, it } from "vitest";
import {
  buildTaxDocumentFacts,
  defaultDocumentLines,
  parseTaxDocumentQuote,
  parseTaxDocumentSimulation,
  validateTaxDocumentLines,
} from "./taxDocumentSimulation";

const calculationId = "0199f8a8-0000-7abc-8def-0123456789ab";
const digest = Array.from({ length: 32 }, (_, index) => index);

describe("versioned document simulation browser boundary", () => {
  it("builds bounded facts without changing caller line identity", () => {
    const lines = defaultDocumentLines().reverse();
    expect(validateTaxDocumentLines(lines)).toEqual([]);
    expect(buildTaxDocumentFacts("invoice-1", "usd", "US-CA", lines)).toEqual({
      id: "invoice-1",
      currency: "USD",
      jurisdiction: "US-CA",
      jurisdiction_resolution: null,
      cumulative_period: null,
      lines: [
        { id: "line-b", gross_atomic: "2000", net_atomic: "2000", exemption_codes: [] },
        { id: "line-a", gross_atomic: "1000", net_atomic: "1000", exemption_codes: [] },
      ],
    });
  });

  it("rejects duplicate, zero, and non-conserving lines before network access", () => {
    expect(validateTaxDocumentLines([
      { id: "same", grossAtomic: "0", netAtomic: "0", exemptionCodes: "" },
      { id: "same", grossAtomic: "10", netAtomic: "11", exemptionCodes: "" },
    ])).toEqual(expect.arrayContaining([
      "Line IDs must be unique.",
      "Line 1 gross must be a positive u64 decimal string.",
      "Line 1 net must be a positive u64 decimal string.",
      "Line 2 net cannot exceed gross.",
    ]));
  });

  it("parses a complete unattested line trace and rejects numeric money", () => {
    const response = parseTaxDocumentSimulation({
      calculation_id: calculationId,
      status: "simulated_unattested",
      result: {
        document_id: "invoice-1",
        rule_set_id: "rule-1",
        rule_version: 1,
        engine_version: "tax-engine-v1",
        currency: "USD",
        jurisdiction: "US-CA",
        facts_digest: digest,
        rule_digest: digest,
        gross_atomic: "3000",
        total_tax_atomic: "247",
        total_net_atomic: "2753",
        lines: [{
          id: "line-a",
          gross_atomic: "1000",
          total_tax_atomic: "82",
          total_net_atomic: "918",
          components: [{ id: "sales-tax", label: "Sales tax", tax_atomic: "82", explanation: "rate" }],
        }],
      },
    });
    expect(response.result.lines[0].id).toBe("line-a");
    expect(() => parseTaxDocumentSimulation({
      calculation_id: calculationId,
      status: "simulated_unattested",
      result: { gross_atomic: 3000 },
    })).toThrow(/malformed/i);
  });

  it("parses only persisted document-quote statuses and expiry", () => {
    const response = parseTaxDocumentQuote({
      calculation_id: calculationId,
      status: "calculated_unattested",
      schema: "solana-tax-protocol/tax-document/v1",
      expires_at_unix: 1_800_000_000,
      result: {
        document_id: "invoice-1",
        rule_set_id: "rule-1",
        rule_version: 1,
        engine_version: "tax-engine-v1",
        currency: "USD",
        jurisdiction: "US-CA",
        facts_digest: digest,
        rule_digest: digest,
        gross_atomic: "1000",
        total_tax_atomic: "100",
        total_net_atomic: "900",
        lines: [],
      },
    });
    expect(response.status).toBe("calculated_unattested");
    expect(response.expires_at_unix).toBe(1_800_000_000);
    expect(() => parseTaxDocumentQuote({
      calculation_id: calculationId,
      status: "simulated_unattested",
      schema: "solana-tax-protocol/tax-document/v1",
      expires_at_unix: 1_800_000_000,
      result: {},
    })).toThrow(/malformed/i);
  });
});
