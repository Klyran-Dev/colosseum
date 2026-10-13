// @vitest-environment happy-dom

import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import RefundExecutePanel from "./RefundExecutePanel";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe("refund execution localized operator surface", () => {
  it("makes refund execution unavailable before sign-in", () => {
    render(<RefundExecutePanel apiBase="/api" serviceOnline accessToken="" locale="en-US" />);

    expect(screen.getByRole("note").textContent).toContain("Sign in to continue");
    expect(screen.getByRole("textbox", { name: /Finalized refund request reference/ }).closest("fieldset")).toHaveProperty("disabled", true);
  });

  it("localizes German copy while keeping final release disabled without identity", () => {
    vi.stubGlobal("fetch", vi.fn());
    render(<RefundExecutePanel apiBase="/api" serviceOnline={false} accessToken="" locale="de-DE" />);

    expect(screen.getByRole("heading", { name: "Rückerstattung freigeben" })).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Ausführungsautorisierung" })).toBeTruthy();
    expect(screen.getByLabelText("Referenz der bestätigten Rückerstattungsanfrage")).toBeTruthy();
    expect((screen.getByRole("button", { name: "Rückerstattung freigeben" }) as HTMLButtonElement).disabled).toBe(true);
    expect(screen.getByText("Der Dienst ist nicht verfügbar.", { exact: false })).toBeTruthy();
  });
});
