import { parseFxConversionEvidence, type FxConversionEvidence } from "./taxResponses";

const MAX_LINES = 256;
const MAX_ATOMIC = 18_446_744_073_709_551_615n;
const UUID_V7 = /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
const ATOMIC = /^(0|[1-9][0-9]{0,19})$/;

export type TaxDocumentLineDraft = {
  id: string;
  grossAtomic: string;
  netAtomic: string;
  exemptionCodes: string;
};

export type TaxDocumentFactsRequest = {
  id: string;
  currency: string;
  jurisdiction: string;
  jurisdiction_resolution: null;
  cumulative_period: null;
  lines: Array<{
    id: string;
    gross_atomic: string;
    net_atomic: string;
    exemption_codes: string[];
  }>;
};

export type TaxDocumentCalculationStatus = "simulated_unattested" | "calculated_unattested" | "attested";

export type TaxDocumentCalculation = {
  calculation_id: string;
  status: TaxDocumentCalculationStatus;
  result: {
    document_id: string;
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
    fx_conversion?: FxConversionEvidence;
    lines: Array<{
      id: string;
      gross_atomic: string;
      total_tax_atomic: string;
      total_net_atomic: string;
      components: Array<{
        id: string;
        label: string;
        tax_atomic: string;
        explanation: string;
      }>;
    }>;
  };
};

export type TaxDocumentSimulation = TaxDocumentCalculation & { status: "simulated_unattested" };

export type TaxDocumentQuote = TaxDocumentCalculation & {
  schema: "solana-tax-protocol/tax-document/v1";
  expires_at_unix: number;
};

function invalid(): Error {
  return new Error("The document simulation response is malformed and cannot be displayed safely.");
}

function atomic(value: unknown, allowZero = true): string {
  if (typeof value !== "string" || !ATOMIC.test(value)) throw invalid();
  const parsed = BigInt(value);
  if ((!allowZero && parsed === 0n) || parsed > MAX_ATOMIC) throw invalid();
  return value;
}

function boundedText(value: unknown, label: string, max = 64): string {
  if (typeof value !== "string" || value.trim() === "" || value.length > max) throw new Error(`${label} is required and must be at most ${max} characters.`);
  return value.trim();
}

export function defaultDocumentLines(): TaxDocumentLineDraft[] {
  return [
    { id: "line-a", grossAtomic: "1000", netAtomic: "1000", exemptionCodes: "" },
    { id: "line-b", grossAtomic: "2000", netAtomic: "2000", exemptionCodes: "" },
  ];
}

export function emptyDocumentLines(): TaxDocumentLineDraft[] {
  return [{ id: "", grossAtomic: "", netAtomic: "", exemptionCodes: "" }];
}

export function validateTaxDocumentLines(lines: TaxDocumentLineDraft[]): string[] {
  const errors: string[] = [];
  if (lines.length < 1 || lines.length > MAX_LINES) errors.push(`A document must contain between 1 and ${MAX_LINES} lines.`);
  const ids = new Set<string>();
  for (const [index, line] of lines.entries()) {
    try { const id = boundedText(line.id, `Line ${index + 1} ID`); if (ids.has(id)) errors.push("Line IDs must be unique."); ids.add(id); } catch (error) { errors.push(error instanceof Error ? error.message : "Line ID is invalid."); }
    try { atomic(line.grossAtomic, false); } catch { errors.push(`Line ${index + 1} gross must be a positive u64 decimal string.`); }
    try { atomic(line.netAtomic, false); } catch { errors.push(`Line ${index + 1} net must be a positive u64 decimal string.`); }
    try {
      const gross = BigInt(line.grossAtomic);
      const net = BigInt(line.netAtomic);
      if (net > gross) errors.push(`Line ${index + 1} net cannot exceed gross.`);
    } catch { /* the atomic errors above are the user-facing validation */ }
    const exemptions = line.exemptionCodes.split(/[\n,]+/).map((value) => value.trim()).filter(Boolean);
    if (exemptions.length > 32 || new Set(exemptions).size !== exemptions.length) errors.push(`Line ${index + 1} exemption codes must be unique and bounded to 32.`);
    if (exemptions.some((value) => value.length > 64)) errors.push(`Line ${index + 1} exemption codes must be at most 64 characters.`);
  }
  return [...new Set(errors)];
}

export function buildTaxDocumentFacts(documentId: string, currency: string, jurisdiction: string, lines: TaxDocumentLineDraft[]): TaxDocumentFactsRequest {
  const id = boundedText(documentId, "Document ID");
  const normalizedCurrency = boundedText(currency, "Currency", 12).toUpperCase();
  const normalizedJurisdiction = boundedText(jurisdiction, "Jurisdiction", 64);
  const errors = validateTaxDocumentLines(lines);
  if (errors.length > 0) throw new Error(`Document facts are invalid: ${errors.join(" ")}`);
  return {
    id,
    currency: normalizedCurrency,
    jurisdiction: normalizedJurisdiction,
    jurisdiction_resolution: null,
    cumulative_period: null,
    lines: lines.map((line) => ({
      id: line.id.trim(),
      gross_atomic: line.grossAtomic,
      net_atomic: line.netAtomic,
      exemption_codes: line.exemptionCodes.split(/[\n,]+/).map((value) => value.trim()).filter(Boolean),
    })),
  };
}

function digest(value: unknown): number[] {
  if (!Array.isArray(value) || value.length !== 32 || value.some((byte) => typeof byte !== "number" || !Number.isInteger(byte) || byte < 0 || byte > 255)) throw invalid();
  return [...value];
}

function parseTaxDocumentCalculation(value: unknown, allowedStatuses: TaxDocumentCalculationStatus[]): TaxDocumentCalculation {
  if (typeof value !== "object" || value === null || Array.isArray(value)) throw invalid();
  const response = value as Record<string, unknown>;
  if (typeof response.calculation_id !== "string" || !UUID_V7.test(response.calculation_id) || typeof response.status !== "string" || !allowedStatuses.includes(response.status as TaxDocumentCalculationStatus) || typeof response.result !== "object" || response.result === null || Array.isArray(response.result)) throw invalid();
  const raw = response.result as Record<string, unknown>;
  if (typeof raw.document_id !== "string" || raw.document_id.trim() === "" || typeof raw.rule_set_id !== "string" || raw.rule_set_id.trim() === "" || typeof raw.rule_version !== "number" || !Number.isSafeInteger(raw.rule_version) || raw.rule_version < 1 || typeof raw.engine_version !== "string" || raw.engine_version.trim() === "" || typeof raw.currency !== "string" || raw.currency.trim() === "" || typeof raw.jurisdiction !== "string" || raw.jurisdiction.trim() === "" || !Array.isArray(raw.lines) || raw.lines.length > MAX_LINES) throw invalid();
  const lines = raw.lines.map((value) => {
    if (typeof value !== "object" || value === null || Array.isArray(value)) throw invalid();
    const line = value as Record<string, unknown>;
    if (typeof line.id !== "string" || line.id.trim() === "" || !Array.isArray(line.components)) throw invalid();
    const components = line.components.map((componentValue) => {
      if (typeof componentValue !== "object" || componentValue === null || Array.isArray(componentValue)) throw invalid();
      const component = componentValue as Record<string, unknown>;
      if (typeof component.id !== "string" || component.id.trim() === "" || typeof component.label !== "string" || typeof component.explanation !== "string") throw invalid();
      return { id: component.id, label: component.label, tax_atomic: atomic(component.tax_atomic), explanation: component.explanation };
    });
    return { id: line.id, gross_atomic: atomic(line.gross_atomic, false), total_tax_atomic: atomic(line.total_tax_atomic), total_net_atomic: atomic(line.total_net_atomic), components };
  });
  return {
    calculation_id: response.calculation_id,
    status: response.status as TaxDocumentCalculationStatus,
    result: {
      document_id: raw.document_id,
      rule_set_id: raw.rule_set_id,
      rule_version: raw.rule_version,
      engine_version: raw.engine_version,
      currency: raw.currency,
      jurisdiction: raw.jurisdiction,
      facts_digest: digest(raw.facts_digest),
      rule_digest: digest(raw.rule_digest),
      gross_atomic: atomic(raw.gross_atomic, false),
      total_tax_atomic: atomic(raw.total_tax_atomic),
      total_net_atomic: atomic(raw.total_net_atomic),
      fx_conversion: raw.fx_conversion === undefined || raw.fx_conversion === null
        ? undefined
        : parseFxConversionEvidence(raw.fx_conversion, raw.currency),
      lines,
    },
  };
}

export function parseTaxDocumentSimulation(value: unknown): TaxDocumentSimulation {
  const parsed = parseTaxDocumentCalculation(value, ["simulated_unattested"]);
  return { ...parsed, status: "simulated_unattested" };
}

export function parseTaxDocumentQuote(value: unknown): TaxDocumentQuote {
  const parsed = parseTaxDocumentCalculation(value, ["calculated_unattested", "attested"]);
  if (typeof value !== "object" || value === null || Array.isArray(value)) throw invalid();
  const response = value as Record<string, unknown>;
  if (response.schema !== "solana-tax-protocol/tax-document/v1"
      || typeof response.expires_at_unix !== "number"
      || !Number.isSafeInteger(response.expires_at_unix)) {
    throw invalid();
  }
  return {
    ...parsed,
    schema: "solana-tax-protocol/tax-document/v1",
    expires_at_unix: response.expires_at_unix,
  };
}

export const MAX_DOCUMENT_LINE_COUNT = MAX_LINES;
