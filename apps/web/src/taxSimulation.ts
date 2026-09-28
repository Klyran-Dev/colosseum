const UUID_V7 = /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
const ATOMIC = /^(0|[1-9][0-9]{0,19})$/;

export type TaxSimulation = {
  calculation_id: string;
  status: "simulated_unattested";
  result: {
    rule_set_id: string;
    rule_version: number;
    engine_version: string;
    currency: string;
    jurisdiction: string;
    facts_digest: number[];
    rule_digest: number[];
    gross_atomic: string;
    total_tax_atomic: string;
    total_net_atomic: string;
    components: Array<{
      id: string;
      label: string;
      base_atomic: string;
      tax_atomic: string;
      explanation: string;
    }>;
  };
};

function malformed(): Error {
  return new Error("The server-owned tax simulation response is malformed and cannot be displayed safely.");
}

function digest(value: unknown): number[] {
  if (!Array.isArray(value) || value.length !== 32 || value.some((byte) => typeof byte !== "number" || !Number.isInteger(byte) || byte < 0 || byte > 255)) throw malformed();
  return value;
}

function atomic(value: unknown): string {
  if (typeof value !== "string" || !ATOMIC.test(value)) throw malformed();
  return value;
}

export function parseTaxSimulation(value: unknown): TaxSimulation {
  if (typeof value !== "object" || value === null) throw malformed();
  const response = value as Record<string, unknown>;
  if (typeof response.calculation_id !== "string" || !UUID_V7.test(response.calculation_id) || response.status !== "simulated_unattested" || typeof response.result !== "object" || response.result === null || Array.isArray(response.result)) throw malformed();
  const result = response.result as Record<string, unknown>;
  if (
    typeof result.rule_set_id !== "string" || result.rule_set_id.trim() === ""
    || typeof result.rule_version !== "number" || !Number.isSafeInteger(result.rule_version) || result.rule_version < 1
    || typeof result.engine_version !== "string" || result.engine_version.trim() === ""
    || typeof result.currency !== "string" || result.currency.trim() === ""
    || typeof result.jurisdiction !== "string" || result.jurisdiction.trim() === ""
    || typeof result.components !== "object" || !Array.isArray(result.components)
  ) throw malformed();
  if (result.components.length > 128) throw malformed();
  const components = result.components.map((value) => {
    if (typeof value !== "object" || value === null || Array.isArray(value)) throw malformed();
    const component = value as Record<string, unknown>;
    if (typeof component.id !== "string" || component.id.trim() === "" || typeof component.label !== "string" || typeof component.explanation !== "string") throw malformed();
    return {
      id: component.id,
      label: component.label,
      base_atomic: atomic(component.base_atomic),
      tax_atomic: atomic(component.tax_atomic),
      explanation: component.explanation,
    };
  });
  return {
    calculation_id: response.calculation_id,
    status: "simulated_unattested",
    result: {
      rule_set_id: result.rule_set_id,
      rule_version: result.rule_version,
      engine_version: result.engine_version,
      currency: result.currency,
      jurisdiction: result.jurisdiction,
      facts_digest: digest(result.facts_digest),
      rule_digest: digest(result.rule_digest),
      gross_atomic: atomic(result.gross_atomic),
      total_tax_atomic: atomic(result.total_tax_atomic),
      total_net_atomic: atomic(result.total_net_atomic),
      components,
    },
  };
}
