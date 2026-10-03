// @vitest-environment happy-dom

import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import App from "./App";
import DocumentQuotePanel from "./DocumentQuotePanel";
import GovernancePanel from "./GovernancePanel";

describe("quote workspace financial-input boundary", () => {
  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
  });

  it("does not present example financial facts as if they belonged to the account", () => {
    vi.stubGlobal("fetch", vi.fn(async () => new Response(null, { status: 200 })));
    render(<App />);

    expect((screen.getByLabelText("Total before tax") as HTMLInputElement).value).toBe("");
    expect((screen.getByLabelText("Amount subject to tax") as HTMLInputElement).value).toBe("");
    expect((screen.getByLabelText("Currency") as HTMLInputElement).value).toBe("");
    expect((screen.getByLabelText("Approved tax rule") as HTMLInputElement).value).toBe("");
    expect((screen.getByLabelText("Rule version") as HTMLInputElement).value).toBe("");
    expect((screen.getAllByLabelText("Jurisdiction")[0] as HTMLInputElement).value).toBe("");
    expect((screen.getByLabelText("Tax category") as HTMLInputElement).value).toBe("");
    expect(screen.queryByLabelText("Request reference")).toBeNull();
    expect(screen.getByText(/secure retry reference is assigned automatically/i)).toBeTruthy();
    expect(screen.queryByText("Acme Treasury")).toBeNull();
  });

  it("does not present an example invoice as the operator's document", () => {
    render(
      <DocumentQuotePanel
        apiBase="/api"
        serviceOnline
        accessToken=""
        formatAtomic={(value, currency) => `${currency} ${value}`}
        onOpenCheckout={() => undefined}
      />,
    );

    expect((screen.getByLabelText("Document ID") as HTMLInputElement).value).toBe("");
    expect((screen.getByLabelText("Currency") as HTMLInputElement).value).toBe("");
    expect((screen.getByLabelText("Jurisdiction") as HTMLInputElement).value).toBe("");
    expect((screen.getByLabelText("Tax category") as HTMLInputElement).value).toBe("");
    expect((screen.getByLabelText("Rule version") as HTMLInputElement).value).toBe("");
    expect((screen.getByLabelText("Amount before tax") as HTMLInputElement).value).toBe("");
    expect((screen.getByLabelText("Amount after tax") as HTMLInputElement).value).toBe("");
    expect(screen.queryByLabelText("Request reference")).toBeNull();
    expect(screen.getByText(/secure retry reference is assigned automatically/i)).toBeTruthy();
  });

  it("does not present example facts in persisted-rule simulation", () => {
    render(<GovernancePanel apiBase="/api" serviceOnline accessToken="signed-token" />);

    expect(screen.getAllByLabelText("Amount before tax").every((field) => (field as HTMLInputElement).value === "")).toBe(true);
    expect(screen.getAllByLabelText("Amount subject to tax").every((field) => (field as HTMLInputElement).value === "")).toBe(true);
    expect(screen.getAllByLabelText("Currency").every((field) => (field as HTMLInputElement).value === "")).toBe(true);
  });
});
