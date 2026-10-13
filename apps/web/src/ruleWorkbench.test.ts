import { describe, expect, it } from "vitest";
import {
  buildRuleSet,
  defaultComponentDraft,
  defaultRuleWorkbenchDraft,
  validateRuleWorkbench,
  RuleWorkbenchDraft,
} from "./ruleWorkbench";

describe("structured Tax Engine rule workbench contract", () => {
  it("builds a canonical flat rule with explicit monetary strings", () => {
    const draft = defaultRuleWorkbenchDraft();
    draft.id = "sales-tax";
    draft.version = "7";
    draft.components[0].id = "sales-tax";
    draft.components[0].flatNumerator = "10";
    draft.components[0].flatDenominator = "100";
    draft.components[0].creditAtomic = "0";

    expect(validateRuleWorkbench(draft)).toEqual([]);
    expect(buildRuleSet(draft)).toEqual({
      id: "sales-tax",
      version: 7,
      components: [{
        id: "sales-tax",
        label: "Sales tax",
        base: { sources: ["Net"] },
        rate: { Flat: { numerator: "10", denominator: "100" } },
        price_mode: "Exclusive",
        credit_atomic: "0",
        floor_atomic: null,
        cap_atomic: null,
        rounding: { mode: "HalfUp" },
        threshold: null,
        exemption_codes: [],
      }],
    });
  });

  it("preserves progressive brackets and tax-on-tax component dependencies", () => {
    const draft = defaultRuleWorkbenchDraft();
    draft.components[0].rateMode = "progressive";
    draft.components[0].brackets = [
      { lowerAtomic: "0", upperAtomic: "1000", numerator: "5", denominator: "100", openEnded: false },
      { lowerAtomic: "1000", upperAtomic: "", numerator: "10", denominator: "100", openEnded: true },
    ];
    const dependent = defaultComponentDraft(2);
    dependent.id = "local-tax";
    dependent.label = "Local tax";
    dependent.baseSources = ["Net", "Component:sales-tax"];
    draft.components.push(dependent);

    expect(validateRuleWorkbench(draft)).toEqual([]);
    const rule = buildRuleSet(draft);
    expect(rule.components[0].rate).toEqual({ Progressive: [
      { lower_atomic: "0", upper_atomic: "1000", rate: { numerator: "5", denominator: "100" } },
      { lower_atomic: "1000", upper_atomic: null, rate: { numerator: "10", denominator: "100" } },
    ] });
    expect(rule.components[1].base.sources).toEqual(["Net", { Component: "sales-tax" }]);
  });

  it("represents thresholds, exemptions, clamps, and declared rounding", () => {
    const draft = defaultRuleWorkbenchDraft();
    const component = draft.components[0];
    component.priceMode = "Exclusive";
    component.creditAtomic = "25";
    component.floorAtomic = "20";
    component.capAtomic = "500";
    component.roundingMode = "HalfEven";
    component.roundingScope = "Settlement";
    component.roundingQuantumAtomic = "10";
    component.thresholdEnabled = true;
    component.thresholdAtomic = "1000";
    component.thresholdBasis = "PerTransaction";
    component.thresholdMode = "ExemptUpTo";
    component.exemptionCodes = "resale\ncharity";

    const rule = buildRuleSet(draft);
    expect(rule.components[0]).toMatchObject({
      price_mode: "Exclusive",
      credit_atomic: "25",
      floor_atomic: "20",
      cap_atomic: "500",
      rounding: { mode: "HalfEven", scope: "Settlement", quantum_atomic: "10" },
      threshold: { amount_atomic: "1000", basis: "PerTransaction", mode: "ExemptUpTo" },
      exemption_codes: ["resale", "charity"],
    });
  });

  it("requires and serializes the explicit inclusive gross-excess equation", () => {
    const draft = defaultRuleWorkbenchDraft();
    const component = draft.components[0];
    component.priceMode = "Inclusive";
    component.thresholdEnabled = true;
    component.thresholdAtomic = "1100";

    expect(validateRuleWorkbench(draft)).toContain(
      "Component sales-tax inclusive thresholds require an explicit equation.",
    );

    component.inclusiveThresholdEquation = "GrossExcess";
    expect(validateRuleWorkbench(draft)).toEqual([]);
    expect(buildRuleSet(draft).components[0].threshold).toEqual({
      amount_atomic: "1100",
      basis: "PerTransaction",
      mode: "ExemptUpTo",
      inclusive_equation: "GrossExcess",
    });
  });

  it("rejects repeated additive base sources before server submission", () => {
    const draft = defaultRuleWorkbenchDraft();
    draft.components[0].baseSources = ["Net", "Net"];
    expect(validateRuleWorkbench(draft)).toContain(
      "Component sales-tax base sources must be distinct.",
    );
  });

  it("builds exact weighted and subtractive base terms without client arithmetic", () => {
    const draft = defaultRuleWorkbenchDraft();
    const component = draft.components[0];
    component.baseSources = ["Net", "Gross"];
    component.baseNumerators = ["3", "-1"];
    component.baseDenominators = ["4", "10"];

    expect(validateRuleWorkbench(draft)).toEqual([]);
    expect(buildRuleSet(draft).components[0].base.sources).toEqual([
      { Weighted: { source: "Net", numerator: "3", denominator: "4" } },
      { Weighted: { source: "Gross", numerator: "-1", denominator: "10" } },
    ]);
  });

  it("builds bounded taxable-base clamps without confusing them with tax clamps", () => {
    const draft = defaultRuleWorkbenchDraft();
    const component = draft.components[0];
    component.baseFloorAtomic = "503";
    component.baseCapAtomic = "900";
    component.roundingQuantumAtomic = "10";

    expect(validateRuleWorkbench(draft)).toEqual([]);
    expect(buildRuleSet(draft).components[0].base).toEqual({
      sources: ["Net"],
      floor_atomic: "503",
      cap_atomic: "900",
    });
  });

  it("rejects a taxable-base floor above its cap", () => {
    const draft = defaultRuleWorkbenchDraft();
    draft.components[0].baseFloorAtomic = "901";
    draft.components[0].baseCapAtomic = "900";

    expect(validateRuleWorkbench(draft)).toContain(
      "Component sales-tax taxable-base floor cannot exceed cap.",
    );
  });

  it("rejects zero or negative-denominator base shares and unsafe inclusive weighting", () => {
    const draft = defaultRuleWorkbenchDraft();
    draft.components[0].baseNumerators = ["0"];
    expect(validateRuleWorkbench(draft)).toContain("Component sales-tax base share numerator must be non-zero and fit the bounded coefficient range.");
    draft.components[0].baseNumerators = ["1"];
    draft.components[0].baseDenominators = ["-1"];
    expect(validateRuleWorkbench(draft)).toContain("Component sales-tax base share denominator must be positive.");
    draft.components[0].baseDenominators = ["2"];
    draft.components[0].priceMode = "Inclusive";
    expect(validateRuleWorkbench(draft)).toContain("Component sales-tax inclusive prices require a whole base amount; weighted bases need an explicit approved equation.");
  });

  it("rejects settlement rounding for mixed scopes, dependencies, and modes", () => {
    const draft = defaultRuleWorkbenchDraft();
    draft.components[0].roundingScope = "Settlement";
    const dependent = defaultComponentDraft(2);
    dependent.baseSources = ["Net", "Component:sales-tax"];
    dependent.roundingScope = "Settlement";
    dependent.roundingMode = "Floor";
    draft.components.push(dependent);

    expect(validateRuleWorkbench(draft)).toEqual(expect.arrayContaining([
      "Aggregate, line, and invoice rounding scopes cannot be used with tax-on-tax component dependencies.",
      "Aggregate, line, and invoice rounding scopes require one rounding mode for the complete rule set.",
    ]));
  });

  it("rejects zero or malformed rounding quanta and preserves the default omission", () => {
    const draft = defaultRuleWorkbenchDraft();
    draft.components[0].roundingQuantumAtomic = "0";
    expect(validateRuleWorkbench(draft)).toContain("Component sales-tax rounding quantum must be positive.");
    draft.components[0].roundingQuantumAtomic = "1.0";
    expect(validateRuleWorkbench(draft)).toContain("Component sales-tax rounding quantum must be a non-negative decimal integer.");
    draft.components[0].roundingQuantumAtomic = "10";
    draft.components[0].floorAtomic = "3";
    expect(validateRuleWorkbench(draft)).toContain("Component sales-tax floor must be an exact multiple of the rounding quantum.");
    draft.components[0].roundingQuantumAtomic = "1";
    draft.components[0].floorAtomic = "";
    expect(buildRuleSet(draft).components[0].rounding).toEqual({ mode: "HalfUp" });
  });

  it("builds explicit line and invoice scopes for document simulation", () => {
    const draft = defaultRuleWorkbenchDraft();
    draft.components[0].roundingScope = "Invoice";
    expect(validateRuleWorkbench(draft)).toEqual([]);
    expect(buildRuleSet(draft).components[0].rounding).toEqual({ mode: "HalfUp", scope: "Invoice" });
    draft.components[0].roundingScope = "Line";
    expect(buildRuleSet(draft).components[0].rounding).toEqual({ mode: "HalfUp", scope: "Line" });
  });

  it("rejects duplicate identifiers, missing dependencies, and non-contiguous brackets", () => {
    const draft = defaultRuleWorkbenchDraft();
    const duplicate = defaultComponentDraft(2);
    duplicate.id = draft.components[0].id;
    draft.components.push(duplicate);
    draft.components[0].baseSources = ["Component:missing"];
    draft.components[0].rateMode = "progressive";
    draft.components[0].brackets = [{
      lowerAtomic: "1",
      upperAtomic: "0",
      numerator: "1",
      denominator: "100",
      openEnded: false,
    }];

    expect(validateRuleWorkbench(draft)).toEqual(expect.arrayContaining([
      "Component IDs must be unique.",
      "Component sales-tax references missing component missing.",
      "Progressive brackets must start at 0 and remain contiguous.",
    ]));
    expect(() => buildRuleSet(draft)).toThrow(/cannot build/i);
  });
});
