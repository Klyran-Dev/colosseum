// @vitest-environment happy-dom

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import VaultReconciliationPanel from "./VaultReconciliationPanel";

afterEach(() => cleanup());

describe("vault reconciliation operator workspace", () => {
  it("explains the protected read-only boundary while signed out", () => {
    render(<VaultReconciliationPanel apiBase="/api" serviceOnline accessToken="" />);

    expect(screen.getByRole("note").textContent).toContain("Sign in to continue");
    expect(screen.getByRole("button", { name: "Load reconciliation targets" }).closest("fieldset")?.disabled).toBe(true);
  });

  it("localizes the read-only balance boundary and release-stop language", () => {
    render(<VaultReconciliationPanel apiBase="/api" serviceOnline accessToken="operator-token" locale="de-DE" />);

    expect(screen.getByRole("heading", { name: "Steuerguthaben prüfen" })).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Aktive Steuerkonten" })).toBeTruthy();
    expect(screen.queryByText("Nachweisdetails", { exact: true })).toBeNull();
    expect(screen.getByRole("button", { name: "Abgleichsziele laden" })).toBeTruthy();
    expect(screen.getByText(/Guthaben nicht ändern, Gelder nicht freigeben/i)).toBeTruthy();
    expect(screen.queryByText(/clear an alert/i)).toBeNull();
  });

  it("redacts provider and transport exceptions without inferring a reconciliation state", async () => {
    const providerDetail = "provider-internal reconciliation diagnostic";
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error(providerDetail)));
    render(<VaultReconciliationPanel apiBase="/api" serviceOnline accessToken="operator-token" />);

    fireEvent.click(screen.getByRole("button", { name: "Load reconciliation targets" }));

    await screen.findByText("Vault reconciliation lookup failed; retry or contact an operator.");
    expect(screen.queryByText(providerDetail)).toBeNull();
    expect(screen.queryByText("Matched")).toBeNull();
  });
});
