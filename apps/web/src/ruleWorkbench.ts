const MAX_ATOMIC = 18_446_744_073_709_551_615n;
const MAX_COMPONENTS = 128;
const MAX_CODE_BYTES = 64;
const MAX_EXEMPTION_CODES = 32;

export type RateMode = "flat" | "progressive";
export type PriceMode = "Exclusive" | "Inclusive";
export type RoundingMode = "Floor" | "Ceil" | "HalfUp" | "HalfEven" | "AwayFromZero";
export type RoundingScope = "Component" | "Settlement" | "Line" | "Invoice";
export type ThresholdBasis = "PerTransaction" | "CumulativePeriod";
export type ThresholdMode = "ExemptUpTo" | "TriggerAt";

export type RuleBracketDraft = {
  lowerAtomic: string;
  upperAtomic: string;
  numerator: string;
  denominator: string;
  openEnded: boolean;
};

export type RuleComponentDraft = {
  id: string;
  label: string;
  baseSources: string[];
  rateMode: RateMode;
  flatNumerator: string;
  flatDenominator: string;
  brackets: RuleBracketDraft[];
  priceMode: PriceMode;
  creditAtomic: string;
  floorAtomic: string;
  capAtomic: string;
  roundingMode: RoundingMode;
  roundingScope: RoundingScope;
  roundingQuantumAtomic: string;
  thresholdEnabled: boolean;
  thresholdAtomic: string;
  thresholdBasis: ThresholdBasis;
  thresholdMode: ThresholdMode;
  exemptionCodes: string;
};

export type RuleWorkbenchDraft = {
  id: string;
  version: string;
  components: RuleComponentDraft[];
};

type BuiltBaseSource = "Gross" | "Net" | { Component: string };

export type BuiltRuleSet = {
  id: string;
  version: number;
  components: Array<{
    id: string;
    label: string;
    base: { sources: BuiltBaseSource[] };
    rate:
      | { Flat: { numerator: string; denominator: string } }
      | { Progressive: Array<{ lower_atomic: string; upper_atomic: string | null; rate: { numerator: string; denominator: string } }> };
    price_mode: PriceMode;
    credit_atomic: string;
    floor_atomic: string | null;
    cap_atomic: string | null;
    rounding: { mode: RoundingMode; scope?: RoundingScope; quantum_atomic?: string };
    threshold: { amount_atomic: string; basis: ThresholdBasis; mode: ThresholdMode } | null;
    exemption_codes: string[];
  }>;
};

export function defaultComponentDraft(index = 1): RuleComponentDraft {
  return {
    id: index === 1 ? "sales-tax" : `component-${index}`,
    label: index === 1 ? "Sales tax" : `Tax component ${index}`,
    baseSources: ["Net"],
    rateMode: "flat",
    flatNumerator: "10",
    flatDenominator: "100",
    brackets: [
      { lowerAtomic: "0", upperAtomic: "", numerator: "10", denominator: "100", openEnded: true },
    ],
    priceMode: "Exclusive",
    creditAtomic: "0",
    floorAtomic: "",
    capAtomic: "",
    roundingMode: "HalfUp",
    roundingScope: "Component",
    roundingQuantumAtomic: "1",
    thresholdEnabled: false,
    thresholdAtomic: "0",
    thresholdBasis: "PerTransaction",
    thresholdMode: "ExemptUpTo",
    exemptionCodes: "",
  };
}

export function defaultRuleWorkbenchDraft(): RuleWorkbenchDraft {
  return { id: "new-sales-tax", version: "1", components: [defaultComponentDraft()] };
}

function nonNegativeInteger(value: string, name: string, allowEmpty = false): bigint | null {
  if (allowEmpty && value.trim() === "") return null;
  if (!/^(0|[1-9][0-9]*)$/.test(value.trim())) throw new Error(`${name} must be a non-negative decimal integer.`);
  const parsed = BigInt(value);
  if (parsed > MAX_ATOMIC) throw new Error(`${name} must fit an unsigned 64-bit atomic amount.`);
  return parsed;
}

function code(value: string, name: string): void {
  if (value.trim() === "" || value.length > MAX_CODE_BYTES) throw new Error(`${name} is required and must be at most ${MAX_CODE_BYTES} bytes.`);
}

function splitCodes(value: string): string[] {
  return value.split(/[\n,]+/).map((entry) => entry.trim()).filter(Boolean);
}

function validateComponent(component: RuleComponentDraft, ids: Set<string>, allIds: Set<string>): string[] {
  const errors: string[] = [];
  try { code(component.id, "Component ID"); } catch (error) { errors.push(errorMessage(error)); }
  try { code(component.label, `Component ${component.id || "label"}`); } catch (error) { errors.push(errorMessage(error)); }
  if (ids.has(component.id)) errors.push("Component IDs must be unique.");
  ids.add(component.id);
  if (component.baseSources.length === 0 || component.baseSources.length > MAX_COMPONENTS) errors.push("Each component must have between 1 and 128 base sources.");
  for (const source of component.baseSources) {
    if (source !== "Gross" && source !== "Net" && !source.startsWith("Component:")) {
      errors.push(`Component ${component.id} has an unsupported base source.`);
    } else if (source.startsWith("Component:")) {
      const dependency = source.slice("Component:".length).trim();
      if (!dependency || !allIds.has(dependency)) errors.push(`Component ${component.id} references missing component ${dependency || "(empty)"}.`);
    }
  }
  try {
    const credit = nonNegativeInteger(component.creditAtomic, `Component ${component.id} credit`);
    const floor = nonNegativeInteger(component.floorAtomic, `Component ${component.id} floor`, true);
    const cap = nonNegativeInteger(component.capAtomic, `Component ${component.id} cap`, true);
    const quantum = nonNegativeInteger(component.roundingQuantumAtomic, `Component ${component.id} rounding quantum`);
    if (floor !== null && cap !== null && floor > cap) errors.push(`Component ${component.id} floor cannot exceed cap.`);
    if (credit === null) errors.push(`Component ${component.id} credit is required.`);
    if (quantum === 0n) errors.push(`Component ${component.id} rounding quantum must be positive.`);
    if (quantum !== null && quantum > 0n) {
      if (floor !== null && floor % quantum !== 0n) errors.push(`Component ${component.id} floor must be an exact multiple of the rounding quantum.`);
      if (cap !== null && cap % quantum !== 0n) errors.push(`Component ${component.id} cap must be an exact multiple of the rounding quantum.`);
    }
  } catch (error) { errors.push(errorMessage(error)); }
  if (component.rateMode === "flat") {
    validateRate(component.flatNumerator, component.flatDenominator, `Component ${component.id} flat rate`, errors);
  } else {
    if (component.brackets.length === 0 || component.brackets.length > MAX_COMPONENTS) errors.push(`Component ${component.id} progressive brackets are outside the supported bound.`);
    let expectedLower = 0n;
    component.brackets.forEach((bracket, index) => {
      try {
        const lower = nonNegativeInteger(bracket.lowerAtomic, `Component ${component.id} bracket ${index + 1} lower bound`);
        if (lower !== expectedLower) errors.push("Progressive brackets must start at 0 and remain contiguous.");
        const upper = bracket.openEnded ? null : nonNegativeInteger(bracket.upperAtomic, `Component ${component.id} bracket ${index + 1} upper bound`);
        if (upper !== null && upper <= (lower ?? 0n)) errors.push("Progressive bracket upper bounds must exceed their lower bounds.");
        if (bracket.openEnded && index !== component.brackets.length - 1) errors.push("Only the final progressive bracket may be open-ended.");
        if (upper !== null) expectedLower = upper;
      } catch (error) { errors.push(errorMessage(error)); }
      validateRate(bracket.numerator, bracket.denominator, `Component ${component.id} bracket ${index + 1} rate`, errors);
    });
  }
  if (component.thresholdEnabled) {
    try { nonNegativeInteger(component.thresholdAtomic, `Component ${component.id} threshold`); } catch (error) { errors.push(errorMessage(error)); }
    if (component.priceMode === "Inclusive") errors.push(`Component ${component.id} cannot combine an inclusive price mode with a threshold in rule DSL v1.`);
  }
  const exemptions = splitCodes(component.exemptionCodes);
  if (exemptions.length > MAX_EXEMPTION_CODES) errors.push(`Component ${component.id} has too many exemption codes.`);
  if (new Set(exemptions).size !== exemptions.length) errors.push(`Component ${component.id} exemption codes must be unique.`);
  return errors;
}

function validateRate(numerator: string, denominator: string, name: string, errors: string[]): void {
  try {
    nonNegativeInteger(numerator, `${name} numerator`);
    const parsedDenominator = nonNegativeInteger(denominator, `${name} denominator`);
    if (parsedDenominator === 0n) errors.push(`${name} denominator must be positive.`);
  } catch (error) { errors.push(errorMessage(error)); }
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : "Invalid rule workbench value.";
}

export function validateRuleWorkbench(draft: RuleWorkbenchDraft): string[] {
  const errors: string[] = [];
  try { code(draft.id, "Rule-set ID"); } catch (error) { errors.push(errorMessage(error)); }
  if (!/^[1-9][0-9]*$/.test(draft.version) || Number(draft.version) > 4_294_967_295) errors.push("Rule-set version must be a positive u32 integer.");
  if (draft.components.length === 0 || draft.components.length > MAX_COMPONENTS) errors.push("The rule set must contain between 1 and 128 components.");
  const allIds = new Set(draft.components.map((component) => component.id));
  const ids = new Set<string>();
  for (const component of draft.components) errors.push(...validateComponent(component, ids, allIds));
  const scopes = new Set(draft.components.map((component) => component.roundingScope));
  if (scopes.size > 1) errors.push("All components must use the same rounding scope.");
  if (draft.components.some((component) => ["Settlement", "Line", "Invoice"].includes(component.roundingScope) && component.baseSources.some((source) => source.startsWith("Component:")))) {
    errors.push("Aggregate, line, and invoice rounding scopes cannot be used with tax-on-tax component dependencies.");
  }
  if (draft.components.some((component) => ["Settlement", "Line", "Invoice"].includes(component.roundingScope) && component.roundingMode !== draft.components[0]?.roundingMode)) {
    errors.push("Aggregate, line, and invoice rounding scopes require one rounding mode for the complete rule set.");
  }
  return [...new Set(errors)];
}

export function buildRuleSet(draft: RuleWorkbenchDraft): BuiltRuleSet {
  const errors = validateRuleWorkbench(draft);
  if (errors.length > 0) throw new Error(`Rule workbench cannot build: ${errors.join(" ")}`);
  return {
    id: draft.id.trim(),
    version: Number(draft.version),
    components: draft.components.map((component) => ({
      id: component.id.trim(),
      label: component.label.trim(),
      base: { sources: component.baseSources.map((source) => source === "Gross" || source === "Net" ? source : { Component: source.slice("Component:".length).trim() }) },
      rate: component.rateMode === "flat"
        ? { Flat: { numerator: component.flatNumerator.trim(), denominator: component.flatDenominator.trim() } }
        : { Progressive: component.brackets.map((bracket) => ({
          lower_atomic: bracket.lowerAtomic.trim(),
          upper_atomic: bracket.openEnded ? null : bracket.upperAtomic.trim(),
          rate: { numerator: bracket.numerator.trim(), denominator: bracket.denominator.trim() },
        })) },
      price_mode: component.priceMode,
      credit_atomic: component.creditAtomic.trim(),
      floor_atomic: component.floorAtomic.trim() === "" ? null : component.floorAtomic.trim(),
      cap_atomic: component.capAtomic.trim() === "" ? null : component.capAtomic.trim(),
      rounding: {
        mode: component.roundingMode,
        ...(component.roundingScope !== "Component" ? { scope: component.roundingScope } : {}),
        ...(component.roundingQuantumAtomic.trim() === "1" ? {} : { quantum_atomic: component.roundingQuantumAtomic.trim() }),
      },
      threshold: component.thresholdEnabled ? {
        amount_atomic: component.thresholdAtomic.trim(),
        basis: component.thresholdBasis,
        mode: component.thresholdMode,
      } : null,
      exemption_codes: splitCodes(component.exemptionCodes),
    })),
  };
}
