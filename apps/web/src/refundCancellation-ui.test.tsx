// @vitest-environment happy-dom

import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import RefundCancellationPanel from "./RefundCancellationPanel";

afterEach(cleanup);

describe("refund cancellation operator language", () => {
  it("leads with the cancellation decision and keeps recovery data advanced", () => {
    render(<RefundCancellationPanel apiBase="/api" serviceOnline accessToken="operator-token" />);

    expect(screen.getByRole("heading", { name: "Cancel a refund request" })).toBeTruthy();
    expect(screen.getByLabelText("Finalized refund request reference")).toBeTruthy();
    expect(screen.getByLabelText(/Request reference/)).toBeTruthy();
    expect(screen.getByRole("button", { name: "Prepare cancellation" })).toBeTruthy();
    expect(screen.getByLabelText("Cancellation reference")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Load saved cancellation" })).toBeTruthy();
    expect(screen.queryByText(/UUIDv7|PDA|atomic units|server-owned/i)).toBeNull();
  });
});
