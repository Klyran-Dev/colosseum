// @vitest-environment happy-dom

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
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
});
