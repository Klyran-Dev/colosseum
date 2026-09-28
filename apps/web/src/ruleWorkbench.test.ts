import { describe, expect, it } from "vitest";
import {
  buildRuleSet,
  defaultComponentDraft,
  defaultRuleWorkbenchDraft,
  validateRuleWorkbench,
  RuleWorkbenchDraft,
} from "./ruleWorkbench";

describe("structured Tier-1 rule workbench contract", () => {
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
    component.floorAtomic = "3";
    component.capAtomic = "500";
    component.roundingMode = "HalfEven";
    component.thresholdEnabled = true;
    component.thresholdAtomic = "1000";
    component.thresholdBasis = "PerTransaction";
    component.thresholdMode = "ExemptUpTo";
    component.exemptionCodes = "resale\ncharity";

    const rule = buildRuleSet(draft);
    expect(rule.components[0]).toMatchObject({
      price_mode: "Exclusive",
      credit_atomic: "25",
      floor_atomic: "3",
      cap_atomic: "500",
      rounding: { mode: "HalfEven" },
      threshold: { amount_atomic: "1000", basis: "PerTransaction", mode: "ExemptUpTo" },
      exemption_codes: ["resale", "charity"],
    });
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
