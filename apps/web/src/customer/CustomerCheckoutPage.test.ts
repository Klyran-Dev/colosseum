import { describe, expect, it } from "vitest";
import { parsePublicCheckout } from "./CustomerCheckoutPage";

const calculationId = "0199d6f4-2b2a-7c31-8c6d-8b0b1b5c9013";
const routeId = "0199d6f4-2b2a-7c31-8c6d-8b0b1b5c9014";

function payload() {
  return {
    schema: "public-checkout-session/v1",
    session_id: "0199d6f4-2b2a-7c31-8c6d-8b0b1b5c9015",
    status: "active",
    calculation_id: calculationId,
    route_id: routeId,
    expires_at_unix: Math.floor(Date.now() / 1000) + 900,
    settlement_intent_id: null,
    route: { id: routeId, route_key: "devnet-usdc", version: 1, mint: "So11111111111111111111111111111111111111112", mint_decimals: 9, token_program: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA" },
    quote: {
      calculation_id: calculationId,
      status: "attested",
      expires_at_unix: Math.floor(Date.now() / 1000) + 900,
      result: {
        rule_set_id: "demo-rule", rule_version: 1, engine_version: "tax-engine-v1", atomic_decimals: 2, currency: "USD", jurisdiction: "US-CA", facts_digest: Array(32).fill(1), rule_digest: Array(32).fill(2), gross_atomic: "1100", total_tax_atomic: "100", total_net_atomic: "1000", components: [{ id: "sales-tax", label: "Sales tax", base_atomic: "1000", effective_base_atomic: "1000", tax_atomic: "100", explanation: "10% applied.", exemption_applied: false }], treatment_outcome: { treatment: "Taxable", arithmetic_tax_atomic: "100", collected_tax_atomic: "100", self_assessed_tax_atomic: "0", withheld_tax_atomic: "0", taxable_base_atomic: "1000" },
      },
    },
  };
}

describe("public checkout session contract", () => {
  it("accepts a session only when the quote and route identities match", () => {
    const parsed = parsePublicCheckout(payload());
    expect(parsed.sessionId).toBe("0199d6f4-2b2a-7c31-8c6d-8b0b1b5c9015");
    expect(parsed.quote.result.totalTaxAtomic).toBe("100");
    expect(parsed.route.routeKey).toBe("devnet-usdc");
  });

  it("rejects a client-visible quote rebound to another calculation", () => {
    const value = payload();
    value.quote.calculation_id = "0199d6f4-2b2a-7c31-8c6d-8b0b1b5c9016";
    expect(() => parsePublicCheckout(value)).toThrow(/quote is expired or is not bound/iu);
  });
});
