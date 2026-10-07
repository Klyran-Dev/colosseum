// @vitest-environment happy-dom

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import CheckoutPanel from "./CheckoutPanel";

describe("payment review language boundary", () => {
  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
  });

  it("keeps the operator path plain and the protocol evidence explicitly advanced", () => {
    vi.stubGlobal("fetch", vi.fn(async () => new Response(null, { status: 200 })));
    render(
      <CheckoutPanel
        apiBase="/api"
        serviceOnline
        accessToken=""
        calculation={null}
        formatAtomic={(value, currency) => `${currency} ${value}`}
      />,
    );

    expect(screen.getByRole("heading", { name: "Review payment before signing" })).toBeTruthy();
    expect(screen.getByText("Review the amount, tax, destination, and policy status before your wallet approves the payment. The service checks the payment again before it is sent to the network.")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Prepare payment for review" })).toBeTruthy();

    expect(screen.queryByText("Program ID")).toBeNull();
    expect(screen.queryByLabelText("Policy version")).toBeNull();
    expect(screen.queryByLabelText("Policy reference")).toBeNull();
    expect(screen.getByText(/verified quote supplies the policy/i)).toBeTruthy();
    expect(screen.getByText(/secure payment reference is assigned for safe retry/i)).toBeTruthy();
  });

  it("localizes aggregate payment review without translating financial or protocol values", () => {
    vi.stubGlobal("fetch", vi.fn(async () => new Response(null, { status: 200 })));
    render(
      <CheckoutPanel
        apiBase="/api"
        serviceOnline={false}
        accessToken=""
        calculation={null}
        locale="de-DE"
        formatAtomic={(value, currency) => `${currency} ${value}`}
      />,
    );

    expect(screen.getByRole("heading", { name: "Zahlung vor der Signatur prüfen" })).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Zahlungsprüfung" })).toBeTruthy();
    expect(screen.getByText("Wallet-Bestätigung erforderlich", { exact: true })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Zahlung zur Prüfung vorbereiten" })).toBeTruthy();
    expect(screen.getByText("Derzeit ist kein kompatibles Solana-Wallet verfügbar.", { exact: true })).toBeTruthy();
    expect(screen.queryByText(/Tokenkeg|0199|1000/)).toBeNull();
  });

  it("warns when a recipient plan belongs to a different merchant account and accepts a matching one", async () => {
    const route = {
      id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d22",
      route_key: "local-usdc",
      version: 1,
      merchant_account: "8qbHbw2BbbTHBW1sbeqakYXVKRQM8Ne7pLK7m6CVfeR",
      mint: "CktRuQ2mttgRGkXJtyksdKHjUdc2C4TgDzyB98oEzy8",
      mint_decimals: 6,
      tax_vault: "8nGRRJuGoubpUKHVa5paPCvGLQU39nnu3zFWYREbSeNN",
      net_destination: "QWmroo4YnnMqYW3cnxWkFdaTxGD3P7vMSzwMHGbUzwF",
      token_program: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
    };
    const plan = (id: string, key: string, merchant: string) => ({
      id,
      plan_key: key,
      version: 1,
      merchant_account: merchant,
      mint: route.mint,
      plan_digest: Array.from({ length: 32 }, (_, index) => index),
      recipients: [
        { token_account: route.net_destination, basis_points: 7000 },
        { token_account: route.tax_vault, basis_points: 3000 },
      ],
    });
    const plans = [
      plan("01992f1b-4a70-7e30-8ea2-b6e37a9d9d41", "other-merchant-plan", route.net_destination),
      plan("01992f1b-4a70-7e30-8ea2-b6e37a9d9d42", "matching-plan", route.merchant_account),
    ];
    vi.stubGlobal("fetch", vi.fn(async (input: RequestInfo | URL) => {
      const url = String(input);
      if (url.endsWith("/v1/settlement/routes")) return new Response(JSON.stringify([route]), { status: 200 });
      if (url.endsWith("/v1/settlement/split-plans")) return new Response(JSON.stringify(plans), { status: 200 });
      return new Response(null, { status: 404 });
    }));
    const calculation = {
      calculation_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d21",
      status: "calculated_unattested",
      expires_at_unix: Math.floor(Date.now() / 1000) + 3600,
      result: {
        rule_set_id: "published-sales-tax", rule_version: 3, engine_version: "tax-engine/v1", atomic_decimals: 2, currency: "USD", jurisdiction: "US-CA",
        facts_digest: Array(32).fill(1), rule_digest: Array(32).fill(2), gross_atomic: "1000", total_tax_atomic: "100", total_net_atomic: "900",
        components: [],
      },
    };
    const { container } = render(
      <CheckoutPanel apiBase="/api" serviceOnline accessToken="token" calculation={calculation as never} formatAtomic={(value, currency) => `${currency} ${value}`} />,
    );

    const mode = await waitFor(() => {
      const select = container.querySelector('select option[value="split"]')?.parentElement as HTMLSelectElement | null;
      if (!select) throw new Error("mode select not ready");
      return select;
    });
    fireEvent.change(mode, { target: { value: "split" } });

    await waitFor(() => expect(container.querySelector(".route-evidence-warning")).toBeTruthy());
    const planSelect = container.querySelector('select option[value="01992f1b-4a70-7e30-8ea2-b6e37a9d9d42"]')?.parentElement as HTMLSelectElement;
    fireEvent.change(planSelect, { target: { value: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d42" } });
    await waitFor(() => expect(container.querySelector(".route-evidence-warning")).toBeNull());
    expect(container.querySelector(".route-evidence.full-field")).toBeTruthy();
  });
});
