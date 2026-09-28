import { describe, expect, it } from "vitest";
import { parseTaxSimulation } from "./taxSimulation";

const response = {
  calculation_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d15",
  status: "simulated_unattested",
  result: {
    rule_set_id: "new-sales-tax",
    rule_version: 1,
    engine_version: "0.1.0",
    currency: "USD",
    jurisdiction: "US-CA",
    facts_digest: Array.from({ length: 32 }, (_, index) => index),
    rule_digest: Array.from({ length: 32 }, (_, index) => 255 - index),
    gross_atomic: "1100",
    total_tax_atomic: "100",
    total_net_atomic: "1000",
    components: [{ id: "sales-tax", label: "Sales tax", base_atomic: "1000", tax_atomic: "100", explanation: "10% of net" }],
  },
};

describe("server-owned tax simulation response", () => {
  it("accepts the bounded calculation trace", () => {
    expect(parseTaxSimulation(response).result.components[0].tax_atomic).toBe("100");
  });

  it("rejects a caller-shaped or unsafe response", () => {
    expect(() => parseTaxSimulation({ ...response, status: "attested" })).toThrow(/malformed/);
    expect(() => parseTaxSimulation({ ...response, result: { ...response.result, total_tax_atomic: "-1" } })).toThrow(/malformed/);
  });
});
