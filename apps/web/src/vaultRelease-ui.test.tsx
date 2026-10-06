// @vitest-environment happy-dom

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import VaultReleasePanel from "./VaultReleasePanel";
import { SUPPORTED_LOCALES } from "./formatters";
import { vaultReleaseMessagesFor } from "./vaultReleaseMessages";

const claimId = "01992f1b-4a70-7e30-8ea2-b6e37a9d9d21";
const merchantId = "01992f1b-4a70-7e30-8ea2-b6e37a9d9d22";
const publicKey = "11111111111111111111111111111111";
const routeId = "01992f1b-4a70-7e30-8ea2-b6e37a9d9d23";
const tokenProgram = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

function routeResponse(): Record<string, unknown>[] {
  return [{
    id: routeId,
    route_key: "treasury-usdc",
    version: 1,
    merchant_account: publicKey,
    mint: publicKey,
    mint_decimals: 6,
    tax_vault: "Sysvar1nstructions1111111111111111111111111",
    net_destination: "SysvarRecentB1ockHashes11111111111111111111",
    token_program: tokenProgram,
  }];
}

function claim(status: string, overrides: Record<string, unknown> = {}): Record<string, unknown> {
  return {
    id: claimId,
    merchant_id: merchantId,
    idempotency_key: "release-01J8Q4J0M9M2G7YB8F5N3D6K9R",
    request_digest: Array(32).fill(1),
    mint: publicKey,
    token_program: publicKey,
    tax_vault: publicKey,
    destination: publicKey,
    claim_pda: null,
    nonce: "42",
    amount_atomic: "250000",
    reference_digest: Array(32).fill(2),
    status,
    proposer_subject: "proposer-subject",
    approver_subject: status === "approved" ? "approver-subject" : null,
    execute_after_unix: 1_800_000_000,
    expires_at_unix: 1_800_003_600,
    transaction_signature: null,
    slot: null,
    last_error_code: null,
    ...overrides,
  };
}

function fillClaimProposalForm(): void {
  fireEvent.change(screen.getByLabelText("Release reference number"), { target: { value: "42" } });
  fireEvent.change(screen.getByLabelText("Verified payment route"), { target: { value: routeId } });
  fireEvent.change(screen.getByLabelText("Recipient account"), { target: { value: publicKey } });
  fireEvent.change(screen.getByLabelText("Amount"), { target: { value: "250000" } });
  fireEvent.change(screen.getByLabelText("Evidence reference"), { target: { value: JSON.stringify(Array(32).fill(2)) } });
}

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe("vault-release response and authority boundary", () => {
  it("provides a complete bounded catalog with deterministic fallback", () => {
    for (const locale of SUPPORTED_LOCALES) {
      const messages = vaultReleaseMessagesFor(locale);
      expect(messages.proposal.title).toBeTruthy();
      expect(messages.lifecycle.title).toBeTruthy();
      expect(messages.lifecycle.nextStep).toBeTruthy();
      expect(messages.authority.title).toBeTruthy();
    }
    expect(vaultReleaseMessagesFor("xx-XX").proposal.title).toBe(vaultReleaseMessagesFor("en-US").proposal.title);
  });

  it("localizes operator copy without translating authority-bound values", () => {
    vi.stubGlobal("fetch", vi.fn());
    render(<VaultReleasePanel apiBase="/api" serviceOnline={false} accessToken="" locale="de-DE" />);

    expect(screen.getByRole("heading", { name: "Freigabeantrag erstellen" })).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Antrag prüfen oder weiterführen" })).toBeTruthy();
    expect((screen.getByRole("button", { name: "Freigabeantrag erstellen" }) as HTMLButtonElement).disabled).toBe(true);
    expect(screen.getByText("Diese Seite kann Folgendes tun:", { exact: false })).toBeTruthy();
    expect(screen.getByText("Die API ist nicht verfügbar.", { exact: false })).toBeTruthy();
  });

  it("does not invent a release evidence digest", () => {
    vi.stubGlobal("fetch", vi.fn());
    render(<VaultReleasePanel apiBase="/api" serviceOnline accessToken="proposer-token" />);

    expect((screen.getByLabelText("Evidence reference") as HTMLTextAreaElement).value).toBe("");
    expect((screen.getByLabelText("Release reference number") as HTMLInputElement).value).toBe("");
    expect((screen.getByLabelText("Amount") as HTMLInputElement).value).toBe("");
  });

  it("fails closed before rendering a malformed financial claim", async () => {
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(new Response(JSON.stringify(routeResponse()), { status: 200 }))
      .mockResolvedValueOnce(new Response(JSON.stringify(claim("proposed", { amount_atomic: 250000 })), { status: 200 }));
    vi.stubGlobal("fetch", fetchMock);
    render(<VaultReleasePanel apiBase="/api" serviceOnline accessToken="proposer-token" />);

    await waitFor(() => expect((screen.getByLabelText("Verified payment route") as HTMLSelectElement).value).toBe(routeId));
    fillClaimProposalForm();
    fireEvent.click(screen.getByRole("button", { name: "Create release request" }));

    await waitFor(() => expect(screen.getByRole("alert").textContent).toMatch(/invalid release request/));
    expect(screen.queryByText(/250,000 atomic units/)).toBeNull();
  });

  it("reloads the authoritative claim after a successful transition", async () => {
    const approved = claim("approved");
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(new Response(JSON.stringify(routeResponse()), { status: 200 }))
      .mockResolvedValueOnce(new Response(JSON.stringify(claim("proposed")), { status: 200 }))
      .mockResolvedValueOnce(new Response(JSON.stringify({
        status: "transitioned",
        claim_id: claimId,
        from_status: "proposed",
        to_status: "approved",
        approver_subject: "approver-subject",
      }), { status: 200 }))
      .mockResolvedValueOnce(new Response(JSON.stringify(approved), { status: 200 }));
    vi.stubGlobal("fetch", fetchMock);
    render(<VaultReleasePanel apiBase="/api" serviceOnline accessToken="approver-token" />);

    await waitFor(() => expect((screen.getByLabelText("Verified payment route") as HTMLSelectElement).value).toBe(routeId));
    fillClaimProposalForm();
    fireEvent.click(screen.getByRole("button", { name: "Create release request" }));
    await waitFor(() => expect(screen.getAllByText("proposed").length).toBeGreaterThan(0));
    fireEvent.click(screen.getByRole("button", { name: "Record next step" }));

    await waitFor(() => expect(screen.getByText(/page was refreshed from the service/)).toBeTruthy());
    expect(fetchMock).toHaveBeenCalledTimes(4);
    expect(fetchMock.mock.calls[2][0]).toBe(`/api/v1/vault/release-claims/${claimId}/transition`);
    expect(fetchMock.mock.calls[3][0]).toBe(`/api/v1/vault/release-claims/${claimId}`);
    expect(screen.getAllByText("approved").length).toBeGreaterThan(0);
  });

  it("submits only the governed route reference for merchant-side accounts", async () => {
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(new Response(JSON.stringify(routeResponse()), { status: 200 }))
      .mockResolvedValueOnce(new Response(JSON.stringify(claim("proposed")), { status: 200 }));
    vi.stubGlobal("fetch", fetchMock);
    render(<VaultReleasePanel apiBase="/api" serviceOnline accessToken="proposer-token" />);

    await waitFor(() => expect((screen.getByLabelText("Verified payment route") as HTMLSelectElement).value).toBe(routeId));
    fillClaimProposalForm();
    fireEvent.click(screen.getByRole("button", { name: "Create release request" }));

    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(2));
    const request = JSON.parse(fetchMock.mock.calls[1][1].body as string) as Record<string, unknown>;
    expect(request.route_id).toBe(routeId);
    expect(request).not.toHaveProperty("mint");
    expect(request).not.toHaveProperty("token_program");
    expect(request).not.toHaveProperty("tax_vault");
  });

  it("redacts provider and transport exceptions while loading release routes", async () => {
    const providerDetail = "provider-internal vault route diagnostic";
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error(providerDetail)));
    render(<VaultReleasePanel apiBase="/api" serviceOnline accessToken="proposer-token" />);

    await screen.findByText("Payment routes could not be loaded.", { exact: false });
    expect(screen.queryByText(providerDetail)).toBeNull();
    expect(screen.getByRole("button", { name: "Create release request" }).getAttribute("disabled")).not.toBeNull();
  });
});
