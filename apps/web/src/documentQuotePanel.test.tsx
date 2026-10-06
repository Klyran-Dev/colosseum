// @vitest-environment happy-dom

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import DocumentQuotePanel from "./DocumentQuotePanel";
import { documentQuoteMessagesFor } from "./documentQuoteMessages";

const quote = {
  calculation_id: "0199f8a8-0000-7abc-8def-0123456789ab",
  status: "calculated_unattested",
  expires_at_unix: 1_800_000_000,
  schema: "solana-tax-protocol/tax-document/v1",
  result: {
    document_id: "invoice-1",
    rule_set_id: "rule-1",
    rule_version: 1,
    engine_version: "tax-engine-v1",
    atomic_decimals: 2,
    currency: "USD",
    jurisdiction: "US-CA",
    facts_digest: new Array(32).fill(1),
    rule_digest: new Array(32).fill(2),
    gross_atomic: "3000",
    total_tax_atomic: "247",
    total_net_atomic: "2753",
    lines: [],
  },
};

function fillRequiredInvoiceFields() {
  fireEvent.change(screen.getByLabelText("Document ID"), { target: { value: "invoice-1" } });
  fireEvent.change(screen.getByLabelText("Currency"), { target: { value: "usd" } });
  fireEvent.change(screen.getByRole("textbox", { name: /Asset precision/ }), { target: { value: "2" } });
  fireEvent.change(screen.getByLabelText("Jurisdiction"), { target: { value: "US-CA" } });
  fireEvent.change(screen.getByLabelText("Tax category"), { target: { value: "sales-tax" } });
  fireEvent.change(screen.getByLabelText("Approved tax rule"), { target: { value: "rule-1" } });
  fireEvent.change(screen.getByLabelText("Rule version"), { target: { value: "1" } });
  fireEvent.change(screen.getByLabelText("Line ID"), { target: { value: "line-1" } });
  fireEvent.change(screen.getByLabelText("Amount before tax"), { target: { value: "3000" } });
  fireEvent.change(screen.getByLabelText("Amount after tax"), { target: { value: "3000" } });
}

describe("authenticated document quote workspace", () => {
  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
  });

  it("posts invoice facts and hands the saved quote to checkout", async () => {
    const fetchMock = vi.fn(async () => new Response(JSON.stringify(quote), { status: 200 }));
    vi.stubGlobal("fetch", fetchMock);
    const onOpenCheckout = vi.fn();
    render(
      <DocumentQuotePanel
        apiBase="/api"
        serviceOnline
        accessToken="signed-token"
        formatAtomic={(value, currency) => `${currency} ${value}`}
        onOpenCheckout={onOpenCheckout}
      />,
    );

    expect(screen.getByRole("status", { name: "Sign-in status" }).textContent).toContain("Signed-in session ready");
    expect(screen.queryByDisplayValue("signed-token")).toBeNull();

    fillRequiredInvoiceFields();
    fireEvent.change(screen.getByRole("textbox", { name: /Provider-signed location record \(advanced\)/ }), {
      target: { value: '[{"fixture":"provider-signed-evidence"}]' },
    });
    fireEvent.click(screen.getByRole("button", { name: "Calculate invoice tax" }));

    await screen.findByText("Persisted document evidence returned by the authenticated API. It is not attested or settled yet.");
    expect(fetchMock).toHaveBeenCalledWith(
      "/api/v1/tax/document-quote",
      expect.objectContaining({
        method: "POST",
        headers: expect.objectContaining({ authorization: "Bearer signed-token" }),
      }),
    );
    const checkoutButton = screen.getByRole("button", { name: "Open document checkout" });
    expect(checkoutButton.getAttribute("disabled")).toBeNull();
    fireEvent.click(checkoutButton);
    expect(onOpenCheckout).toHaveBeenCalledWith(quote.calculation_id);
  });

  it("localizes invoice entry without introducing document or financial values", () => {
    render(
      <DocumentQuotePanel
        apiBase="/api"
        serviceOnline
        accessToken="session"
        locale="de-DE"
        formatAtomic={(value, currency) => `${currency} ${value}`}
        onOpenCheckout={() => undefined}
      />,
    );

    expect(screen.getByRole("heading", { name: "Steuer für eine Rechnung berechnen" })).toBeTruthy();
    expect(screen.getByLabelText("Dokument-ID")).toBeTruthy();
    expect(screen.getByPlaceholderText("Zum Beispiel Umsatzsteuer")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Position hinzufügen" })).toBeTruthy();
    expect(screen.queryByText("invoice-1")).toBeNull();
    expect(screen.queryByText("USD")).toBeNull();
  });

  it("rejects an empty jurisdiction-evidence set before network submission", async () => {
    const fetchMock = vi.fn();
    vi.stubGlobal("fetch", fetchMock);
    render(
      <DocumentQuotePanel
        apiBase="/api"
        serviceOnline
        accessToken="signed-token"
        formatAtomic={(value, currency) => `${currency} ${value}`}
        onOpenCheckout={vi.fn()}
      />,
    );

    fillRequiredInvoiceFields();
    fireEvent.change(screen.getByRole("textbox", { name: /Provider-signed location record \(advanced\)/ }), {
      target: { value: "[]" },
    });
    fireEvent.submit(screen.getByRole("button", { name: "Calculate invoice tax" }).closest("form")!);

    await screen.findByText("Provider jurisdiction evidence must be a non-empty bounded JSON array.");
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it("reuses retry identity for unchanged facts and rotates it after a fact changes", async () => {
    const fetchMock = vi.fn(async () => new Response(JSON.stringify(quote), { status: 200 }));
    vi.stubGlobal("fetch", fetchMock);
    render(
      <DocumentQuotePanel
        apiBase="/api"
        serviceOnline
        accessToken="signed-token"
        formatAtomic={(value, currency) => `${currency} ${value}`}
        onOpenCheckout={vi.fn()}
      />,
    );

    fillRequiredInvoiceFields();
    fireEvent.change(screen.getByRole("textbox", { name: /Provider-signed location record \(advanced\)/ }), { target: { value: '[{"fixture":"provider-signed-evidence"}]' } });
    const submit = () => fireEvent.click(screen.getByRole("button", { name: "Calculate invoice tax" }));
    const requestBody = (index: number): { idempotency_key: string } => {
      const call = fetchMock.mock.calls[index] as unknown as [RequestInfo | URL, RequestInit | undefined] | undefined;
      if (!call || !call[1] || typeof call[1].body !== "string") throw new Error(`Missing request body at call ${index}`);
      return JSON.parse(call[1].body) as { idempotency_key: string };
    };
    submit();
    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(1));
    submit();
    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(2));
    const firstKey = requestBody(0).idempotency_key;
    const retryKey = requestBody(1).idempotency_key;
    expect(retryKey).toBe(firstKey);

    fireEvent.change(screen.getByLabelText("Currency"), { target: { value: "eur" } });
    submit();
    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(3));
    const changedFactsKey = requestBody(2).idempotency_key;
    expect(changedFactsKey).not.toBe(firstKey);
  });

  it("keeps implementation vocabulary out of the primary invoice form", () => {
    render(<DocumentQuotePanel apiBase="/api" serviceOnline accessToken="session" formatAtomic={(value, currency) => `${value} ${currency}`} onOpenCheckout={() => undefined} />);

    expect(screen.getByLabelText("Tax category")).toBeTruthy();
    expect(screen.getByLabelText("Approved tax rule")).toBeTruthy();
    const locationSummary = screen.getByText("Location verification");
    expect(locationSummary.closest("details")?.hasAttribute("open")).toBe(false);
    expect(screen.getByRole("textbox", { name: /Provider-signed location record \(advanced\)/ })).toBeTruthy();
    expect(screen.queryByLabelText("Tax regime")).toBeNull();
    expect(screen.queryByLabelText("Published rule set ID")).toBeNull();
    expect(screen.queryByRole("textbox", { name: /Location evidence/ })).toBeNull();
  });

  it("redacts provider and transport exceptions from the invoice quote surface", async () => {
    const providerDetail = "provider-internal document quote diagnostic";
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error(providerDetail)));
    render(
      <DocumentQuotePanel
        apiBase="/api"
        serviceOnline
        accessToken="signed-token"
        formatAtomic={(value, currency) => `${currency} ${value}`}
        onOpenCheckout={vi.fn()}
      />,
    );

    fillRequiredInvoiceFields();
    fireEvent.change(screen.getByRole("textbox", { name: /Provider-signed location record \(advanced\)/ }), {
      target: { value: '[{"fixture":"provider-signed-evidence"}]' },
    });
    fireEvent.click(screen.getByRole("button", { name: "Calculate invoice tax" }));

    await screen.findByText(documentQuoteMessagesFor("en-US").errors.quoteFailed);
    expect(screen.queryByText(providerDetail)).toBeNull();
  });
});
