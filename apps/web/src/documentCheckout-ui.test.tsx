// @vitest-environment happy-dom

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { vi } from "vitest";
import DocumentCheckoutPanel from "./DocumentCheckoutPanel";

describe("document payment operator language", () => {
  afterEach(() => cleanup());

  it("keeps technical protocol vocabulary out of the primary decision path", () => {
    render(<DocumentCheckoutPanel apiBase="/api" serviceOnline={false} accessToken="" />);

    expect(screen.getByRole("heading", { name: "Review and authorize this payment" })).toBeTruthy();
    expect(screen.getByText("Evidence required", { exact: true })).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Confirm amount and recipient" })).toBeTruthy();
    expect(screen.getByText("Safeguards checked", { exact: true })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Check invoice evidence" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Prepare payment for review" })).toBeTruthy();
    expect(screen.queryByLabelText("Invoice evidence (signed record)")).toBeNull();
    expect(screen.getByText("Invoice evidence will be checked by the service", { exact: true })).toBeTruthy();
    expect(screen.getByText("Sign in required to load payment data", { exact: true })).toBeTruthy();
    expect(screen.getByRole("textbox", { name: "Invoice reference" })).toHaveProperty("disabled", true);
    expect(screen.getByRole("combobox", { name: "Payment scope" })).toHaveProperty("disabled", true);

    expect(screen.queryByText("Document-bound non-custodial checkout", { exact: true })).toBeNull();
    expect(screen.queryByText("Scoped settlement intent", { exact: true })).toBeNull();
    expect(screen.queryByText("Compile, review, then sign", { exact: true })).toBeNull();
    expect(screen.queryByText("No client arithmetic", { exact: true })).toBeNull();
  });

  it("localizes the document checkout decision surface while preserving protocol values", () => {
    render(<DocumentCheckoutPanel apiBase="/api" serviceOnline={false} accessToken="" locale="de-DE" />);

    expect(screen.getByRole("heading", { name: "Diese Zahlung prüfen und autorisieren" })).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Betrag und Empfänger bestätigen" })).toBeTruthy();
    expect(screen.getByText("Nachweis erforderlich", { exact: true })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Zahlung zur Prüfung vorbereiten" })).toBeTruthy();
    expect(screen.getByText("Derzeit ist kein kompatibles Solana-Wallet verfügbar.", { exact: true })).toBeTruthy();
    expect(screen.queryByText(/Tokenkeg|0199|1000/)).toBeNull();
  });

  it("offers an explicit retry when the document payment route cannot be loaded", async () => {
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
    let routeAttempts = 0;
    vi.stubGlobal("fetch", vi.fn(async (input: RequestInfo | URL) => {
      if (!String(input).endsWith("/v1/settlement/routes")) return new Response("[]", { status: 200 });
      routeAttempts += 1;
      return routeAttempts === 1
        ? new Response(JSON.stringify({ error: "temporary" }), { status: 503 })
        : new Response(JSON.stringify([route]), { status: 200 });
    }));

    render(<DocumentCheckoutPanel apiBase="/api" serviceOnline accessToken="token" />);

    await screen.findByText("Payment routes could not be loaded.");
    fireEvent.click(screen.getByRole("button", { name: "Retry payment route" }));
    await waitFor(() => expect(screen.getByRole("option", { name: /local-usdc/ })).toBeTruthy());
    expect(routeAttempts).toBe(2);
  });
});
