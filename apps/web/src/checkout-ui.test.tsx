// @vitest-environment happy-dom

import { cleanup, render, screen } from "@testing-library/react";
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
});
