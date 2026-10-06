// @vitest-environment happy-dom

import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
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
});
