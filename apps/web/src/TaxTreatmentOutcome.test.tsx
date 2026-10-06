// @vitest-environment happy-dom
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import TaxTreatmentOutcome from "./TaxTreatmentOutcome";

afterEach(() => cleanup());

const outcome = {
  treatment: "ReverseCharge" as const,
  arithmetic_tax_atomic: "20",
  collected_tax_atomic: "0",
  self_assessed_tax_atomic: "20",
  withheld_tax_atomic: "0",
  taxable_base_atomic: "100",
};

describe("tax treatment outcome presentation", () => {
  it("shows separate settlement channels and the non-advice boundary", () => {
    render(
      <TaxTreatmentOutcome
        outcome={outcome}
        currency="USD"
        atomicDecimals={2}
        locale="en-US"
        formatAtomic={(value, currency) => `${value} ${currency}`}
      />,
    );
    expect(screen.getByRole("region", { name: "Tax treatment outcome" })).toBeTruthy();
    expect(screen.getByText("Reverse charge")).toBeTruthy();
    expect(screen.getByText("Self-assessed by the recipient")).toBeTruthy();
    expect(screen.getByText("This classification supports calculation and settlement review. It is not a filing, legal conclusion, or tax advice.")).toBeTruthy();
  });

  it("does not render when legacy evidence has no treatment outcome", () => {
    const { container } = render(
      <TaxTreatmentOutcome outcome={undefined} currency="USD" atomicDecimals={2} locale="en-US" formatAtomic={(value) => value} />,
    );
    expect(container.firstChild).toBeNull();
  });
});
