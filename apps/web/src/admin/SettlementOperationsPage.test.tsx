// @vitest-environment happy-dom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import SettlementOperationsPage from "./SettlementOperationsPage";

const session = { accessToken: "token", email: "operator@example.com", expiresAtUnix: 2_000_000_000 };
const capability = { id: "settlement.wallet", label: "Review payment", decision: "allow" as const, reason: "scope_and_role_satisfied", control: "server" as const };

afterEach(() => { cleanup(); vi.unstubAllGlobals(); });

describe("SettlementOperationsPage", () => {
  it("does not show route loading when settlement permission is unavailable", () => {
    const fetchMock = vi.fn();
    vi.stubGlobal("fetch", fetchMock);
    render(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}><SettlementOperationsPage session={session} capability={{ id: "settlement.wallet", label: "Review payment", decision: "deny", reason: "scope_missing", control: "server" }} /></QueryClientProvider>);

    expect(screen.getByText(/Settlement access is not assigned to this account\./u)).toBeTruthy();
    expect(screen.queryByText("Loading finalized route evidence…")).toBeNull();
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it("loads finalized routes and sends only quote-bound compilation inputs", async () => {
    const fetchMock = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
      const path = String(input);
      if (path === "/api/v1/settlement/routes") return new Response(JSON.stringify([{ id: "0199d6f4-2b2a-7c31-8c6d-8b0b1b5c9013", route_key: "usdc-mainnet", version: 3, merchant_account: "merchant", mint: "mint", mint_decimals: 6, tax_vault: "vault", net_destination: "destination", token_program: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA" }]), { status: 200, headers: { "content-type": "application/json" } });
      expect(path).toBe("/api/v1/settlement/compile-from-quote");
      const body = JSON.parse(String(init?.body)) as Record<string, unknown>;
      expect(body).toEqual({ calculation_id: "0199d6f4-2b2a-7c31-8c6d-8b0b1b5c9014", route_id: "0199d6f4-2b2a-7c31-8c6d-8b0b1b5c9013", payer: "payer-wallet", nonce: "42" });
      return new Response(JSON.stringify({ status: "prepared_unsubmitted", intent_id: "0199d6f4-2b2a-7c31-8c6d-8b0b1b5c9015", program_id: "4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi", instruction_data_base64: "AQID", accounts: [{ pubkey: "payer-wallet", is_signer: true, is_writable: true }], request_digest: Array(32).fill(1), instruction_digest: Array(32).fill(2), compute_budget_policy: { policy_version: 1, compute_unit_limit: 250000, micro_lamports_per_compute_unit: 1000 } }), { status: 200, headers: { "content-type": "application/json" } });
    });
    vi.stubGlobal("fetch", fetchMock);
    render(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}><SettlementOperationsPage session={session} capability={capability} /></QueryClientProvider>);

    expect(await screen.findByText("usdc-mainnet · v3")).toBeTruthy();
    fireEvent.change(screen.getByLabelText("Calculation reference"), { target: { value: "0199d6f4-2b2a-7c31-8c6d-8b0b1b5c9014" } });
    fireEvent.change(screen.getByLabelText("Route reference"), { target: { value: "0199d6f4-2b2a-7c31-8c6d-8b0b1b5c9013" } });
    fireEvent.change(screen.getByLabelText("Payer wallet"), { target: { value: "payer-wallet" } });
    fireEvent.change(screen.getByLabelText("Nonce · canonical decimal string"), { target: { value: "42" } });
    fireEvent.click(screen.getByRole("button", { name: "Prepare wallet handoff" }));

    expect(await screen.findByText("Wallet handoff prepared")).toBeTruthy();
    expect(screen.getByText("prepared_unsubmitted")).toBeTruthy();
    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(2));
  });
});
