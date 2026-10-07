// @vitest-environment happy-dom

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import TaxDocumentSimulationPanel from "./TaxDocumentSimulationPanel";

afterEach(cleanup);

describe("document simulation workspace", () => {
  it("localizes the form and fail-closed service state without changing jurisdiction data", () => {
    render(
      <TaxDocumentSimulationPanel
        apiBase="/api"
        serviceOnline={false}
        ruleSet={{ id: "rule-1" }}
        jurisdiction="US-CA"
        locale="de-DE"
      />,
    );

    expect(screen.getByText("Steuersimulation für Zeilen und Rechnung")).toBeTruthy();
    expect(screen.getByLabelText("Dokument-ID")).toBeTruthy();
    expect(screen.getByLabelText("Betrag vor Steuer")).toBeTruthy();
    expect(screen.getByText("Die Jurisdiktion folgt der geladenen Regel: US-CA.")).toBeTruthy();
    expect(screen.getByText("Der Dienst ist nicht verfügbar. Diese Arbeitsfläche zeigt keine erfundene lokale Summe an.")).toBeTruthy();
    expect(screen.queryByText("Line and invoice tax simulation")).toBeNull();
  });

  it("does not assume a two-decimal asset precision before the operator supplies it", () => {
    render(
      <TaxDocumentSimulationPanel
        apiBase="/api"
        serviceOnline
        ruleSet={{ id: "rule-1" }}
        jurisdiction="US-CA"
      />,
    );

    expect((screen.getByRole("textbox", { name: /Asset precision/ }) as HTMLInputElement).value).toBe("");
  });

  it("redacts provider and transport exceptions from the document simulation surface", async () => {
    const providerDetail = "provider-internal document simulation diagnostic";
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error(providerDetail)));
    render(
      <TaxDocumentSimulationPanel
        apiBase="/api"
        serviceOnline
        ruleSet={{ id: "rule-1" }}
        jurisdiction="US-CA"
      />,
    );

    fireEvent.change(screen.getByLabelText("Document ID"), { target: { value: "invoice-1" } });
    fireEvent.change(screen.getByLabelText("Currency"), { target: { value: "USD" } });
    fireEvent.change(screen.getByRole("textbox", { name: /Asset precision/ }), { target: { value: "2" } });
    fireEvent.change(screen.getByLabelText("Line ID"), { target: { value: "line-1" } });
    fireEvent.change(screen.getByLabelText("Amount before tax"), { target: { value: "1000" } });
    fireEvent.change(screen.getByLabelText("Amount after tax"), { target: { value: "1000" } });
    fireEvent.click(screen.getByRole("button", { name: "Simulate document tax" }));

    await screen.findByText("Document simulation failed");
    expect(screen.queryByText(providerDetail)).toBeNull();
  });

  it("renders the line-level trace and totals of a successful unattested simulation", async () => {
    const digest = Array.from({ length: 32 }, (_, index) => index);
    const payload = {
      calculation_id: "0199f8a8-0000-7abc-8def-0123456789ab",
      status: "simulated_unattested",
      result: {
        document_id: "invoice-1",
        rule_set_id: "rule-1",
        rule_version: 1,
        engine_version: "tax-engine-v1",
        atomic_decimals: 2,
        currency: "USD",
        jurisdiction: "US-CA",
        facts_digest: digest,
        rule_digest: digest,
        gross_atomic: "3000",
        total_tax_atomic: "247",
        total_net_atomic: "2753",
        treatment_outcome: {
          treatment: "Withholding",
          arithmetic_tax_atomic: "247",
          collected_tax_atomic: "247",
          self_assessed_tax_atomic: "0",
          withheld_tax_atomic: "247",
          taxable_base_atomic: "3000",
        },
        lines: [
          { id: "line-a", gross_atomic: "1000", total_tax_atomic: "82", total_net_atomic: "918", components: [{ id: "sales-tax", label: "Sales tax", tax_atomic: "82", explanation: "rate" }] },
          { id: "line-b", gross_atomic: "2000", total_tax_atomic: "165", total_net_atomic: "1835", components: [{ id: "sales-tax", label: "Sales tax", tax_atomic: "165", explanation: "rate" }] },
        ],
      },
    };
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response(JSON.stringify(payload), { status: 200 })));
    const { container } = render(
      <TaxDocumentSimulationPanel apiBase="/api" serviceOnline ruleSet={{ id: "rule-1" }} jurisdiction="US-CA" />,
    );

    fireEvent.change(screen.getByLabelText("Document ID"), { target: { value: "invoice-1" } });
    fireEvent.change(screen.getByLabelText("Currency"), { target: { value: "USD" } });
    fireEvent.change(screen.getByRole("textbox", { name: /Asset precision/ }), { target: { value: "2" } });
    fireEvent.change(screen.getByLabelText("Line ID"), { target: { value: "line-a" } });
    fireEvent.change(screen.getByLabelText("Amount before tax"), { target: { value: "1000" } });
    fireEvent.change(screen.getByLabelText("Amount after tax"), { target: { value: "1000" } });
    fireEvent.click(screen.getByRole("button", { name: "Simulate document tax" }));

    await waitFor(() => expect(container.querySelectorAll(".document-result-line")).toHaveLength(2));
    expect(container.querySelector(".document-result .total-card")).toBeTruthy();
    expect(container.querySelector(".document-result")?.textContent).toMatch(/line-b/);
  });
});
