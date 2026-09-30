// @vitest-environment happy-dom

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import DocumentQuotePanel from "./DocumentQuotePanel";

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

describe("authenticated document quote workspace", () => {
  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
  });

  it("posts server-owned document facts and hands an attested quote to checkout", async () => {
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

    fireEvent.change(screen.getByLabelText("Published rule set ID"), { target: { value: "rule-1" } });
    fireEvent.change(screen.getByRole("textbox", { name: /Provider-signed jurisdiction evidence/ }), {
      target: { value: '[{"fixture":"provider-signed-evidence"}]' },
    });
    fireEvent.click(screen.getByRole("button", { name: "Create document quote" }));

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

    fireEvent.change(screen.getByLabelText("Published rule set ID"), { target: { value: "rule-1" } });
    fireEvent.change(screen.getByRole("textbox", { name: /Provider-signed jurisdiction evidence/ }), {
      target: { value: "[]" },
    });
    fireEvent.submit(screen.getByRole("button", { name: "Create document quote" }).closest("form")!);

    await screen.findByText("Provider jurisdiction evidence must be a non-empty bounded JSON array.");
    expect(fetchMock).not.toHaveBeenCalled();
  });
});
