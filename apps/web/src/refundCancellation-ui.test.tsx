// @vitest-environment happy-dom

import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import RefundCancellationPanel from "./RefundCancellationPanel";

afterEach(cleanup);

describe("refund cancellation operator language", () => {
  it("makes cancellation preparation unavailable before sign-in", () => {
    render(<RefundCancellationPanel apiBase="/api" serviceOnline accessToken="" locale="en-US" />);

    expect(screen.getByRole("note").textContent).toContain("Sign in to continue");
    expect(screen.getByRole("textbox", { name: /Finalized refund request reference/ }).closest("fieldset")).toHaveProperty("disabled", true);
  });

  it("leads with the cancellation decision and keeps recovery data advanced", () => {
    render(<RefundCancellationPanel apiBase="/api" serviceOnline accessToken="operator-token" locale="de-DE" />);

    expect(screen.getByRole("heading", { name: "Rückerstattungsantrag stornieren" })).toBeTruthy();
    expect(screen.getByLabelText("Referenz der bestätigten Rückerstattungsanfrage")).toBeTruthy();
    expect(screen.getByLabelText(/Antragsreferenz/)).toBeTruthy();
    expect(screen.getByRole("button", { name: "Stornierung vorbereiten" })).toBeTruthy();
    expect(screen.getByLabelText("Referenz der Stornierung")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Gespeicherte Stornierung laden" })).toBeTruthy();
    expect(screen.queryByText(/UUIDv7|PDA|atomic units|server-owned/i)).toBeNull();
  });
});
