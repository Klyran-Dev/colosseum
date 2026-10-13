// @vitest-environment happy-dom

import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import RefundClaimPanel from "./RefundClaimPanel";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe("refund claim localized operator surface", () => {
  it("makes refund preparation unavailable before sign-in", () => {
    render(<RefundClaimPanel apiBase="/api" serviceOnline accessToken="" locale="en-US" />);

    expect(screen.getByRole("note").textContent).toContain("Sign in to continue");
    expect(screen.getByRole("textbox", { name: /Approved correction reference/ }).closest("fieldset")).toHaveProperty("disabled", true);
  });

  it("localizes German copy while keeping the wallet action fail-closed", () => {
    vi.stubGlobal("fetch", vi.fn());
    render(<RefundClaimPanel apiBase="/api" serviceOnline={false} accessToken="" locale="de-DE" />);

    expect(screen.getByRole("heading", { name: "Rückerstattungsantrag vorbereiten" })).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Wallet-Autorisierung" })).toBeTruthy();
    expect(screen.getByLabelText("Genehmigte Korrekturreferenz")).toBeTruthy();
    expect((screen.getByRole("button", { name: "Rückerstattungsantrag vorbereiten" }) as HTMLButtonElement).disabled).toBe(true);
    expect(screen.getByText("Der Dienst ist nicht verfügbar.", { exact: false })).toBeTruthy();
  });
});
