// @vitest-environment happy-dom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import RuleGovernancePage from "./RuleGovernancePage";

const session = { accessToken: "token", email: "operator@example.com", expiresAtUnix: 2_000_000_000 };
const capability = { id: "tax.rule.govern", label: "Manage tax rules", decision: "allow" as const, reason: "scope_and_role_satisfied", control: "server" as const };

afterEach(() => { cleanup(); vi.unstubAllGlobals(); });

describe("RuleGovernancePage", () => {
  it("creates an immutable draft through the governed API and renders the returned reference", async () => {
    const fetchMock = vi.fn(async (_input: RequestInfo | URL, init?: RequestInit) => {
      const body = JSON.parse(String(init?.body)) as Record<string, unknown>;
      expect(body).toMatchObject({ external_id: "us-ca-sales-tax", version: 7, jurisdiction: "US-CA", regime: "sales-tax" });
      expect(body.idempotency_key).toEqual(expect.any(String));
      expect(body.rule_set).toMatchObject({ id: "us-ca-sales-tax", version: 7 });
      return new Response(JSON.stringify({ rule_set_id: "0199d6f4-2b2a-7c31-8c6d-8b0b1b5c9013", status: "draft" }), { status: 200, headers: { "content-type": "application/json" } });
    });
    vi.stubGlobal("fetch", fetchMock);
    render(<QueryClientProvider client={new QueryClient()}><RuleGovernancePage session={session} capability={capability} /></QueryClientProvider>);

    fireEvent.change(document.getElementById("rule-rule-set-external-id")!, { target: { value: "us-ca-sales-tax" } });
    fireEvent.change(document.getElementById("rule-version")!, { target: { value: "7" } });
    fireEvent.change(document.getElementById("rule-jurisdiction")!, { target: { value: "US-CA" } });
    fireEvent.change(document.getElementById("rule-regime")!, { target: { value: "sales-tax" } });
    fireEvent.change(document.getElementById("rule-effective-from-unix-seconds")!, { target: { value: "1800000000" } });
    fireEvent.change(document.getElementById("rule-rule-document-json")!, { target: { value: JSON.stringify({ id: "us-ca-sales-tax", version: 7, components: [] }) } });
    fireEvent.change(document.getElementById("rule-provenance-json")!, { target: { value: JSON.stringify({ source: "ticket-123" }) } });
    fireEvent.submit(screen.getByRole("button", { name: "Create immutable draft" }).closest("form")!);

    expect(await screen.findByText("Draft created")).toBeTruthy();
    expect(screen.getByText("0199d6f4-2b2a-7c31-8c6d-8b0b1b5c9013")).toBeTruthy();
  });

  it("keeps governance mutation controls disabled without an allow decision", () => {
    render(<QueryClientProvider client={new QueryClient()}><RuleGovernancePage session={session} capability={{ ...capability, decision: "deny" }} /></QueryClientProvider>);
    expect(screen.getByText("Server governance permission required")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Create immutable draft" })).toHaveProperty("disabled", true);
  });
});
