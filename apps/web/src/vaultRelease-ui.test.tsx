// @vitest-environment happy-dom

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import VaultReleasePanel from "./VaultReleasePanel";

const claimId = "01992f1b-4a70-7e30-8ea2-b6e37a9d9d21";
const merchantId = "01992f1b-4a70-7e30-8ea2-b6e37a9d9d22";
const publicKey = "11111111111111111111111111111111";

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
  fireEvent.change(screen.getByLabelText("Asset mint address"), { target: { value: publicKey } });
  fireEvent.change(screen.getByLabelText("Asset program address"), { target: { value: publicKey } });
  fireEvent.change(screen.getByLabelText("Tax holding account"), { target: { value: publicKey } });
  fireEvent.change(screen.getByLabelText("Recipient account"), { target: { value: publicKey } });
}

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe("vault-release response and authority boundary", () => {
  it("fails closed before rendering a malformed financial claim", async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response(JSON.stringify(claim("proposed", { amount_atomic: 250000 })), { status: 200 }));
    vi.stubGlobal("fetch", fetchMock);
    render(<VaultReleasePanel apiBase="/api" serviceOnline accessToken="proposer-token" />);

    fillClaimProposalForm();
    fireEvent.click(screen.getByRole("button", { name: "Create release request" }));

    await waitFor(() => expect(screen.getByRole("alert").textContent).toMatch(/amount/));
    expect(screen.queryByText(/250,000 atomic units/)).toBeNull();
  });

  it("reloads the authoritative claim after a successful transition", async () => {
    const approved = claim("approved");
    const fetchMock = vi.fn()
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

    fillClaimProposalForm();
    fireEvent.click(screen.getByRole("button", { name: "Create release request" }));
    await waitFor(() => expect(screen.getAllByText("proposed").length).toBeGreaterThan(0));
    fireEvent.click(screen.getByRole("button", { name: "Record next step" }));

    await waitFor(() => expect(screen.getByText(/page was refreshed from the service/)).toBeTruthy());
    expect(fetchMock).toHaveBeenCalledTimes(3);
    expect(fetchMock.mock.calls[1][0]).toBe(`/api/v1/vault/release-claims/${claimId}/transition`);
    expect(fetchMock.mock.calls[2][0]).toBe(`/api/v1/vault/release-claims/${claimId}`);
    expect(screen.getAllByText("approved").length).toBeGreaterThan(0);
  });
});
