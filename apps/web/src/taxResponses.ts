import { validateUuidV7 } from "./settlementAdjustments";

const CALCULATION_STATUSES = ["calculated_unattested", "attested", "simulated_unattested"] as const;
const MAX_I128 = (1n << 127n) - 1n;
const MIN_I128 = -(1n << 127n);

export type CalculationStatus = typeof CALCULATION_STATUSES[number];

export type TaxComponent = {
  id: string;
  label: string;
  base_atomic: string;
  effective_base_atomic: string;
  threshold_atomic: string | null;
  exemption_applied: boolean;
  raw_tax: { numerator: string; denominator: string };
  credit_atomic: string;
  clamped_tax_atomic: string;
  tax_atomic: string;
  explanation: string;
};

export const TAX_TREATMENTS = ["Taxable", "ZeroRated", "Exempt", "ReverseCharge", "Withholding", "OutOfScope"] as const;
export type TaxTreatment = (typeof TAX_TREATMENTS)[number];

export type TreatmentOutcome = {
  treatment: TaxTreatment;
  arithmetic_tax_atomic: string;
  collected_tax_atomic: string;
  self_assessed_tax_atomic: string;
  withheld_tax_atomic: string;
  taxable_base_atomic: string;
};

export type JurisdictionResolution = {
  policy_id: string;
  policy_version: number;
  policy_digest: number[];
  evidence_digest: number[];
  jurisdiction_code: string;
  source: string;
  candidate_count: number;
  trace: Array<{
    source: string;
    jurisdiction_codes: string[];
    candidate_count: number;
    selected: boolean;
  }>;
};

export type FxConversionEvidence = {
  source_currency: string;
  target_currency: string;
  numerator: string;
  denominator: string;
  rounding_mode: "Floor" | "Ceil" | "HalfUp" | "HalfEven" | "AwayFromZero";
  quantum_atomic: string;
  provider: string;
  quote_id: string;
  observed_at_unix: number;
  evidence_digest: number[];
};

export type Calculation = {
  calculation_id: string;
  status: CalculationStatus;
  expires_at_unix?: number;
  result: {
    rule_set_id: string;
    rule_version: number;
    engine_version: string;
    atomic_decimals: number;
    currency: string;
    jurisdiction: string;
    facts_digest: number[];
    rule_digest: number[];
    gross_atomic: string;
    total_tax_atomic: string;
    total_net_atomic: string;
    treatment_outcome?: TreatmentOutcome;
    jurisdiction_resolution?: JurisdictionResolution;
    fx_conversion?: FxConversionEvidence;
    components: TaxComponent[];
  };
};

export type ReplayVerification = {
  status: "replay_verified";
  calculation_id: string;
  engine_version: string;
  facts_digest: number[];
  rule_digest: number[];
};

function objectValue(value: unknown, message: string): Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error(message);
  return value as Record<string, unknown>;
}

function boundedText(value: unknown, label: string, maxBytes: number, allowEmpty = false): string {
  if (typeof value !== "string" || (!allowEmpty && value.trim() === "") || new TextEncoder().encode(value).length > maxBytes) {
    throw new Error(`${label} is invalid or exceeds ${maxBytes} bytes.`);
  }
  return value;
}

function boundedArray(value: unknown, label: string, maximum: number): unknown[] {
  if (!Array.isArray(value) || value.length > maximum) throw new Error(`${label} is invalid or unbounded.`);
  return value;
}

function positiveVersion(value: unknown, label: string): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value <= 0) throw new Error(`${label} is invalid.`);
  return value;
}

function atomicDecimals(value: unknown): number {
  if (typeof value !== "number" || !Number.isInteger(value) || value < 0 || value > 9) {
    throw new Error("atomic decimals must be an integer between 0 and 9.");
  }
  return value;
}

function boundedCount(value: unknown, label: string, maximum: number): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0 || value > maximum) throw new Error(`${label} is invalid.`);
  return value;
}

function digest(value: unknown, label: string): number[] {
  if (!Array.isArray(value) || value.length !== 32 || value.some((byte) => typeof byte !== "number" || !Number.isInteger(byte) || byte < 0 || byte > 255)) {
    throw new Error(`${label} digest is invalid.`);
  }
  return [...value] as number[];
}

function signedI128(value: unknown, label: string): string {
  if (typeof value !== "string" || !/^-?(?:0|[1-9][0-9]*)$/.test(value) || value === "-0") {
    throw new Error(`${label} must be a canonical signed i128 decimal string.`);
  }
  try {
    const parsed = BigInt(value);
    if (parsed < MIN_I128 || parsed > MAX_I128) throw new Error("range");
  } catch {
    throw new Error(`${label} must be a canonical signed i128 decimal string.`);
  }
  return value;
}

function optionalSignedI128(value: unknown, label: string): string | null {
  if (value === null) return null;
  return signedI128(value, label);
}

function positiveI128(value: unknown, label: string): string {
  const parsed = signedI128(value, label);
  if (BigInt(parsed) <= 0n) throw new Error(`${label} must be positive.`);
  return parsed;
}

function nonNegativeI128(value: unknown, label: string): string {
  const parsed = signedI128(value, label);
  if (BigInt(parsed) < 0n) throw new Error(`${label} must be non-negative.`);
  return parsed;
}

export function parseTreatmentOutcome(value: unknown, totalTaxAtomic?: string): TreatmentOutcome | undefined {
  if (value === undefined || value === null) return undefined;
  const record = objectValue(value, "tax treatment outcome is invalid.");
  if (typeof record.treatment !== "string" || !TAX_TREATMENTS.includes(record.treatment as TaxTreatment)) {
    throw new Error("tax treatment outcome is unknown.");
  }
  const outcome = {
    treatment: record.treatment as TaxTreatment,
    arithmetic_tax_atomic: nonNegativeI128(record.arithmetic_tax_atomic, "arithmetic_tax_atomic"),
    collected_tax_atomic: nonNegativeI128(record.collected_tax_atomic, "collected_tax_atomic"),
    self_assessed_tax_atomic: nonNegativeI128(record.self_assessed_tax_atomic, "self_assessed_tax_atomic"),
    withheld_tax_atomic: nonNegativeI128(record.withheld_tax_atomic, "withheld_tax_atomic"),
    taxable_base_atomic: nonNegativeI128(record.taxable_base_atomic, "taxable_base_atomic"),
  };
  if (totalTaxAtomic !== undefined && BigInt(outcome.collected_tax_atomic) !== BigInt(totalTaxAtomic)) {
    throw new Error("treatment collected tax does not match total tax.");
  }
  return outcome;
}

function optionalExpiry(value: unknown): number | undefined {
  if (value === undefined) return undefined;
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value <= 0) throw new Error("quote expiry is invalid.");
  return value;
}

function parseComponent(value: unknown, index: number): TaxComponent {
  const record = objectValue(value, `tax component ${index} is invalid.`);
  const rawTax = objectValue(record.raw_tax, `tax component ${index} raw tax is invalid.`);
  return {
    id: boundedText(record.id, `tax component ${index} id`, 128),
    label: boundedText(record.label, `tax component ${index} label`, 256),
    base_atomic: signedI128(record.base_atomic, `tax component ${index} base_atomic`),
    effective_base_atomic: signedI128(record.effective_base_atomic, `tax component ${index} effective_base_atomic`),
    threshold_atomic: optionalSignedI128(record.threshold_atomic, `tax component ${index} threshold_atomic`),
    exemption_applied: record.exemption_applied === true || record.exemption_applied === false
      ? record.exemption_applied
      : (() => { throw new Error(`tax component ${index} exemption_applied is invalid.`); })(),
    raw_tax: {
      numerator: signedI128(rawTax.numerator, `tax component ${index} raw numerator`),
      denominator: signedI128(rawTax.denominator, `tax component ${index} raw denominator`),
    },
    credit_atomic: signedI128(record.credit_atomic, `tax component ${index} credit_atomic`),
    clamped_tax_atomic: signedI128(record.clamped_tax_atomic, `tax component ${index} clamped_tax_atomic`),
    tax_atomic: signedI128(record.tax_atomic, `tax component ${index} tax_atomic`),
    explanation: boundedText(record.explanation, `tax component ${index} explanation`, 2_048),
  };
}

function parseJurisdictionResolution(value: unknown): JurisdictionResolution {
  const record = objectValue(value, "jurisdiction resolution is invalid.");
  const trace = boundedArray(record.trace, "jurisdiction resolution trace", 128).map((entry, index) => {
    const item = objectValue(entry, `jurisdiction trace ${index} is invalid.`);
    return {
      source: boundedText(item.source, `jurisdiction trace ${index} source`, 128),
      jurisdiction_codes: boundedArray(item.jurisdiction_codes, `jurisdiction trace ${index} codes`, 64)
        .map((code, codeIndex) => boundedText(code, `jurisdiction trace ${index} code ${codeIndex}`, 64)),
      candidate_count: boundedCount(item.candidate_count, `jurisdiction trace ${index} candidate count`, 128),
      selected: item.selected === true || item.selected === false
        ? item.selected
        : (() => { throw new Error(`jurisdiction trace ${index} selected flag is invalid.`); })(),
    };
  });
  return {
    policy_id: boundedText(record.policy_id, "jurisdiction policy ID", 128),
    policy_version: positiveVersion(record.policy_version, "jurisdiction policy version"),
    policy_digest: digest(record.policy_digest, "jurisdiction policy"),
    evidence_digest: digest(record.evidence_digest, "jurisdiction evidence"),
    jurisdiction_code: boundedText(record.jurisdiction_code, "resolved jurisdiction", 64),
    source: boundedText(record.source, "jurisdiction source", 128),
    candidate_count: boundedCount(record.candidate_count, "jurisdiction candidate count", 128),
    trace,
  };
}

export function parseFxConversionEvidence(value: unknown, expectedTargetCurrency?: string): FxConversionEvidence {
  const record = objectValue(value, "FX conversion evidence is invalid.");
  const roundingModes = ["Floor", "Ceil", "HalfUp", "HalfEven", "AwayFromZero"] as const;
  if (typeof record.rounding_mode !== "string" || !roundingModes.includes(record.rounding_mode as typeof roundingModes[number])) {
    throw new Error("FX rounding mode is invalid.");
  }
  if (typeof record.observed_at_unix !== "number" || !Number.isSafeInteger(record.observed_at_unix) || record.observed_at_unix <= 0) {
    throw new Error("FX observation timestamp is invalid.");
  }
  const sourceCurrency = boundedText(record.source_currency, "FX source currency", 16);
  const targetCurrency = boundedText(record.target_currency, "FX target currency", 16);
  if (sourceCurrency === targetCurrency) throw new Error("FX source and target currencies must differ.");
  if (expectedTargetCurrency !== undefined && targetCurrency !== expectedTargetCurrency) {
    throw new Error("FX target currency does not match calculation currency.");
  }
  return {
    source_currency: sourceCurrency,
    target_currency: targetCurrency,
    numerator: positiveI128(record.numerator, "FX numerator"),
    denominator: positiveI128(record.denominator, "FX denominator"),
    rounding_mode: record.rounding_mode as typeof roundingModes[number],
    quantum_atomic: positiveI128(record.quantum_atomic, "FX quantum"),
    provider: boundedText(record.provider, "FX provider", 64),
    quote_id: boundedText(record.quote_id, "FX quote ID", 64),
    observed_at_unix: record.observed_at_unix,
    evidence_digest: digest(record.evidence_digest, "FX evidence"),
  };
}

export function parseCalculationResponse(value: unknown): Calculation {
  const record = objectValue(value, "calculation response is invalid.");
  if (typeof record.status !== "string" || !CALCULATION_STATUSES.includes(record.status as CalculationStatus)) {
    throw new Error("calculation status is invalid.");
  }
  const result = objectValue(record.result, "calculation result is invalid.");
  const components = boundedArray(result.components, "tax components", 128);
  const currency = boundedText(result.currency, "currency", 16);
  const grossAtomic = signedI128(result.gross_atomic, "gross_atomic");
  const totalTaxAtomic = signedI128(result.total_tax_atomic, "total_tax_atomic");
  const totalNetAtomic = signedI128(result.total_net_atomic, "total_net_atomic");
  const treatmentOutcome = parseTreatmentOutcome(result.treatment_outcome, totalTaxAtomic);
  if (components.length === 0 && treatmentOutcome === undefined) throw new Error("tax components or treatment outcome are required.");
  const fxConversion = result.fx_conversion === undefined || result.fx_conversion === null
    ? undefined
    : parseFxConversionEvidence(result.fx_conversion, currency);
  return {
    calculation_id: validateUuidV7(record.calculation_id as string, "calculation ID"),
    status: record.status as CalculationStatus,
    expires_at_unix: optionalExpiry(record.expires_at_unix),
    result: {
      rule_set_id: boundedText(result.rule_set_id, "rule set ID", 128),
      rule_version: positiveVersion(result.rule_version, "rule version"),
      engine_version: boundedText(result.engine_version, "engine version", 128, true),
      atomic_decimals: atomicDecimals(result.atomic_decimals),
      currency,
      jurisdiction: boundedText(result.jurisdiction, "jurisdiction", 64),
      facts_digest: digest(result.facts_digest, "facts"),
      rule_digest: digest(result.rule_digest, "rule"),
      gross_atomic: grossAtomic,
      total_tax_atomic: totalTaxAtomic,
      total_net_atomic: totalNetAtomic,
      jurisdiction_resolution: result.jurisdiction_resolution === undefined
        ? undefined
        : parseJurisdictionResolution(result.jurisdiction_resolution),
      fx_conversion: fxConversion,
      treatment_outcome: treatmentOutcome,
      components: components.map(parseComponent),
    },
  };
}

export function parseReplayVerificationResponse(value: unknown): ReplayVerification {
  const record = objectValue(value, "replay verification response is invalid.");
  if (record.status !== "replay_verified") throw new Error("replay verification status is invalid.");
  return {
    status: "replay_verified",
    calculation_id: validateUuidV7(record.calculation_id as string, "replay calculation ID"),
    engine_version: boundedText(record.engine_version, "replay engine version", 128),
    facts_digest: digest(record.facts_digest, "replay facts"),
    rule_digest: digest(record.rule_digest, "replay rule"),
  };
}
