// @vitest-environment happy-dom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import TaxQuotesPage from "./TaxQuotesPage";

vi.mock("@tanstack/react-router", () => ({ Link: ({ to, children }: { to: string; children: ReactNode }) => <a href={to}>{children}</a> }));

const session = { accessToken: "token", email: "operator@example.com", expiresAtUnix: 2_000_000_000 };
const capability = { id: "tax.quote.create", label: "Create tax quote", decision: "allow" as const, reason: "scope_and_role_satisfied", control: "server" as const };

function renderPage() {
  return render(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}><TaxQuotesPage session={session} capability={capability} /></QueryClientProvider>);
}

afterEach(() => { cleanup(); vi.unstubAllGlobals(); });

describe("TaxQuotesPage", () => {
  it("submits the production quote contract and exposes the server result trace", async () => {
    const fetchMock = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
      expect(String(input)).toBe("/api/v1/tax/quote");
      const body = JSON.parse(String(init?.body)) as Record<string, unknown>;
      expect(body.idempotency_key).toEqual(expect.any(String));
      expect(body.facts).toMatchObject({ gross_atomic: "1100", net_atomic: "1000", atomic_decimals: 2, currency: "USD", jurisdiction: "US-CA" });
      return new Response(JSON.stringify({ calculation_id: "0199d6f4-2b2a-7c31-8c6d-8b0b1b5c9013", status: "quoted", expires_at_unix: 1_800_000_060, result: { rule_set_id: "us-ca-sales-tax", rule_version: 7, engine_version: "tax-engine-v1", atomic_decimals: 2, currency: "USD", jurisdiction: "US-CA", facts_digest: Array(32).fill(1), rule_digest: Array(32).fill(2), gross_atomic: "1100", total_tax_atomic: "100", total_net_atomic: "1000", components: [{ id: "sales-tax", label: "Sales tax", base_atomic: "1000", effective_base_atomic: "1000", threshold_atomic: null, exemption_applied: false, raw_tax: { numerator: "100", denominator: "1" }, credit_atomic: "0", clamped_tax_atomic: "100", tax_atomic: "100", explanation: "10% applied to the taxable net base." }], treatment_outcome: { treatment: "Taxable", arithmetic_tax_atomic: "100", collected_tax_atomic: "100", self_assessed_tax_atomic: "0", withheld_tax_atomic: "0", taxable_base_atomic: "1000" } } }), { status: 200, headers: { "content-type": "application/json" } });
    });
    vi.stubGlobal("fetch", fetchMock);
    renderPage();

    fireEvent.change(screen.getByLabelText(/Gross amount · atomic units/iu), { target: { value: "1100" } });
    fireEvent.change(screen.getByLabelText(/Net amount · atomic units/iu), { target: { value: "1000" } });
    fireEvent.change(screen.getByLabelText(/Asset currency/iu), { target: { value: "USD" } });
    fireEvent.change(screen.getByLabelText(/Atomic decimals/iu), { target: { value: "2" } });
    fireEvent.change(screen.getByLabelText(/Published rule-set ID/iu), { target: { value: "us-ca-sales-tax" } });
    fireEvent.change(screen.getByLabelText(/Rule version/iu), { target: { value: "7" } });
    fireEvent.change(document.getElementById("quote-jurisdiction")!, { target: { value: "US-CA" } });
    fireEvent.change(screen.getByLabelText(/Tax regime/iu), { target: { value: "sales-tax" } });
    fireEvent.submit(screen.getByRole("button", { name: "Request verified quote" }).closest("form")!);

    expect(await screen.findByText("Verified quote result")).toBeTruthy();
    expect(screen.getByText("Why this tax?")).toBeTruthy();
    expect(screen.getByText("Sales tax")).toBeTruthy();
    expect(screen.getByText("Awaiting issuer attestation")).toBeTruthy();
    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(1));
  });

  it("does not enable quote mutation controls without a server allow decision", () => {
    render(<QueryClientProvider client={new QueryClient()}><TaxQuotesPage session={session} capability={{ ...capability, decision: "indeterminate" }} /></QueryClientProvider>);
    expect(screen.getByText("Fresh server permission is required")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Request verified quote" })).toHaveProperty("disabled", true);
  });
});
