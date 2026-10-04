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
    expect(screen.getByRole("button", { name: "Verify invoice evidence" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Prepare payment for review" })).toBeTruthy();

    expect(screen.queryByText("Document-bound non-custodial checkout", { exact: true })).toBeNull();
    expect(screen.queryByText("Scoped settlement intent", { exact: true })).toBeNull();
    expect(screen.queryByText("Compile, review, then sign", { exact: true })).toBeNull();
    expect(screen.queryByText("No client arithmetic", { exact: true })).toBeNull();
  });
});
