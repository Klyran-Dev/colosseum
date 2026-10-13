// @vitest-environment happy-dom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import WorkflowPanel from "./WorkflowPanel";

afterEach(() => vi.unstubAllGlobals());

describe("admin workflow panel", () => {
  beforeEach(() => {
    vi.stubGlobal("fetch", vi.fn(async () => new Response(JSON.stringify({ calculation_id: "calc-1", status: "verified" }), {
      status: 200,
      headers: { "content-type": "application/json" },
    })));
  });

  it("sends a contract-bound quote with a durable idempotency key", async () => {
    const queryClient = new QueryClient({ defaultOptions: { mutations: { retry: false } } });
    render(<QueryClientProvider client={queryClient}><WorkflowPanel section="quotes" session={{ accessToken: "token", email: "operator@example.com", expiresAtUnix: 2_000_000_000 }} capability={{ id: "tax.quote.create", label: "Create tax quote", decision: "allow", reason: "scope_and_role_satisfied", control: "server" }} /></QueryClientProvider>);

    fireEvent.change(document.getElementById("gross_atomic")!, { target: { value: "1100" } });
    fireEvent.change(document.getElementById("net_atomic")!, { target: { value: "1000" } });
    fireEvent.change(document.getElementById("rule_set_id")!, { target: { value: "us-ca-sales-tax" } });
    fireEvent.change(document.getElementById("jurisdiction")!, { target: { value: "US-CA" } });
    fireEvent.change(document.getElementById("regime")!, { target: { value: "sales-tax" } });
    fireEvent.click(screen.getByRole("button", { name: "Request verified quote" }));

    await waitFor(() => expect(screen.getByRole("status")).toBeTruthy());
    const request = vi.mocked(fetch).mock.calls[0];
    const body = JSON.parse(String(request[1]?.body)) as Record<string, unknown>;
    expect(request[0]).toBe("/api/v1/tax/quote");
    const headers = request[1]?.headers as Headers;
    expect(headers.get("authorization")).toBe("Bearer token");
    expect(headers.get("content-type")).toBe("application/json");
    expect(body.idempotency_key).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-7/iu);
    expect(body.facts).toMatchObject({ gross_atomic: "1100", net_atomic: "1000", currency: "USDC" });
  });
});
