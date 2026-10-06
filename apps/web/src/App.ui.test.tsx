// @vitest-environment happy-dom

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import App from "./App";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  window.history.replaceState({}, "", "/");
});

describe("aggregate tax result workspace", () => {
  it("renders the component trace once inside the progressive explanation", async () => {
    window.history.replaceState({}, "", "/operations/tax-simulator");
    const calculation = {
      calculation_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d13",
      status: "simulated_unattested",
      result: {
        rule_set_id: "operator-simulator-flat-rate",
        rule_version: 1,
        engine_version: "tax-engine/1",
        atomic_decimals: 2,
        currency: "USD",
        jurisdiction: "US-CA",
        facts_digest: Array.from({ length: 32 }, () => 1),
        rule_digest: Array.from({ length: 32 }, () => 2),
        gross_atomic: "10000",
        total_tax_atomic: "825",
        total_net_atomic: "10000",
        components: [{
          id: "sales-tax",
          label: "Sales tax",
          base_atomic: "10000",
          effective_base_atomic: "10000",
          threshold_atomic: null,
          exemption_applied: false,
          raw_tax: { numerator: "825", denominator: "100" },
          credit_atomic: "0",
          clamped_tax_atomic: "825",
          tax_atomic: "825",
          explanation: "Applied to the taxable amount.",
        }],
      },
    };

    vi.stubGlobal("fetch", vi.fn(async (input: RequestInfo | URL) => {
      const url = String(input);
      if (url.endsWith("/healthz")) return new Response(null, { status: 204 });
      if (url.endsWith("/v1/tax/calculate")) return new Response(JSON.stringify(calculation), { status: 200 });
      return new Response(null, { status: 404 });
    }));

    render(<App />);
    await waitFor(() => expect((screen.getByRole("button", { name: "Calculate scenario" }) as HTMLButtonElement).disabled).toBe(false));

    fireEvent.change(screen.getByLabelText("Total before tax"), { target: { value: "10000" } });
    fireEvent.change(screen.getByLabelText("Amount subject to tax"), { target: { value: "10000" } });
    fireEvent.change(screen.getByLabelText("Currency"), { target: { value: "USD" } });
    const fieldInput = (label: string) => screen.getByText(label).closest("label")?.querySelector("input") as HTMLInputElement;
    fireEvent.change(fieldInput("Asset precision"), { target: { value: "2" } });
    fireEvent.change(fieldInput("Tax rate (%)"), { target: { value: "8.25" } });
    fireEvent.change(fieldInput("Jurisdiction"), { target: { value: "US-CA" } });
    fireEvent.click(screen.getByRole("button", { name: "Calculate scenario" }));

    await waitFor(() => expect(document.querySelectorAll(".trace-row").length).toBeGreaterThan(0));
    expect(document.querySelectorAll(".trace-row")).toHaveLength(1);
    expect(screen.getByText("Why this tax?")).toBeTruthy();
  });
});
