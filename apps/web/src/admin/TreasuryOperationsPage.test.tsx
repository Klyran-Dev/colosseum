// @vitest-environment happy-dom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import TreasuryOperationsPage from "./TreasuryOperationsPage";

const session = { accessToken: "token", email: "operator@example.com", expiresAtUnix: 2_000_000_000 };
const capability = { id: "vault.release", label: "Vault release", decision: "allow" as const, reason: "scope_and_role_satisfied", control: "server" as const };
const routeId = "0199d6f4-2b2a-7c31-8c6d-8b0b1b5c9013";
const destination = "11111111111111111111111111111111";

afterEach(() => { cleanup(); vi.unstubAllGlobals(); });

describe("TreasuryOperationsPage", () => {
  it("creates a route-bound claim with exact decimal and digest fields", async () => {
    const fetchMock = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
      const path = String(input);
      if (path === "/api/v1/vault/release-routes") return new Response(JSON.stringify([{ id: routeId, route_key: "usdc-vault", version: 1, merchant_account: "merchant", mint: "mint", mint_decimals: 6, tax_vault: "vault", net_destination: destination, token_program: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA" }]), { status: 200 });
      expect(path).toBe("/api/v1/vault/release-claims");
      const body = JSON.parse(String(init?.body)) as Record<string, unknown>;
      expect(body).toMatchObject({ route_id: routeId, destination, nonce: "18446744073709551615", amount_atomic: "250000", reference_digest: Array(32).fill(7), execute_after_unix: 1790000000, expires_at_unix: 1790003600 });
      expect(new Headers(init?.headers).get("idempotency-key")).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-7/iu);
      return new Response(JSON.stringify({ id: "0199d6f4-2b2a-7c31-8c6d-8b0b1b5c9014", mint: "mint", token_program: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA", tax_vault: "vault", destination, nonce: "18446744073709551615", amount_atomic: "250000", reference_digest: Array(32).fill(7), status: "proposed", proposer_subject: "operator", execute_after_unix: 1790000000, expires_at_unix: 1790003600 }), { status: 200 });
    });
    vi.stubGlobal("fetch", fetchMock);
    render(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}><TreasuryOperationsPage session={session} capability={capability} /></QueryClientProvider>);
    expect(await screen.findByText("usdc-vault · v1")).toBeTruthy();
    fireEvent.change(screen.getByLabelText("Route UUIDv7"), { target: { value: routeId } });
    fireEvent.change(screen.getByLabelText("Destination token account"), { target: { value: destination } });
    fireEvent.change(screen.getByLabelText("Nonce · canonical decimal string"), { target: { value: "18446744073709551615" } });
    fireEvent.change(screen.getByLabelText("Amount · atomic units"), { target: { value: "250000" } });
    fireEvent.change(screen.getByLabelText("Execute after · Unix seconds"), { target: { value: "1790000000" } });
    fireEvent.change(screen.getByLabelText("Expires at · Unix seconds"), { target: { value: "1790003600" } });
    fireEvent.change(screen.getByLabelText("Reference digest · JSON array of 32 bytes"), { target: { value: JSON.stringify(Array(32).fill(7)) } });
    fireEvent.click(screen.getByRole("button", { name: "Propose release claim" }));
    expect(await screen.findByText("proposed")).toBeTruthy();
    expect(screen.getByDisplayValue("0199d6f4-2b2a-7c31-8c6d-8b0b1b5c9014")).toBeTruthy();
  });

  it("does not load or enable treasury actions without the server decision", () => {
    const fetchMock = vi.fn();
    vi.stubGlobal("fetch", fetchMock);
    render(<QueryClientProvider client={new QueryClient()}><TreasuryOperationsPage session={session} /></QueryClientProvider>);
    expect((screen.getByRole("button", { name: "Propose release claim" }) as HTMLButtonElement).disabled).toBe(true);
    expect(fetchMock).not.toHaveBeenCalled();
  });
});
