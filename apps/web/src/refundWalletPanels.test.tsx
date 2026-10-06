// @vitest-environment happy-dom

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { Connection, Keypair, PublicKey, Transaction } from "@solana/web3.js";
import { afterEach, describe, expect, it, vi } from "vitest";
import vector from "../../../test-vectors/settlement-refund-claim-v1.json";
import RefundClaimPanel from "./RefundClaimPanel";
import RefundExecutePanel from "./RefundExecutePanel";

const claimIntentId = "01992f1b-4a70-7e30-8ea2-b6e37a9d9d14";
const executeIntentId = "01992f1b-4a70-7e30-8ea2-b6e37a9d9d15";
const rpcUrl = "http://rpc.test";
const rpcBlockhash = Keypair.fromSeed(new Uint8Array(32).fill(90)).publicKey.toBase58();

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.unstubAllEnvs();
  delete window.solana;
});

describe("refund wallet workspaces", () => {
  it("prepares, collects both claim signatures, and submits unchanged bytes", async () => {
    const proposer = Keypair.fromSeed(new Uint8Array(32).fill(91));
    const refundFunder = Keypair.fromSeed(new Uint8Array(32).fill(92));
    const calls = installWalletAndFetchMock(proposer, refundFunder, claimResponse(proposer, refundFunder), "claim");

    render(<RefundClaimPanel apiBase="/api" serviceOnline accessToken="test-token" locale="en-US" />);
    fireEvent.change(screen.getByLabelText("Approved correction reference"), { target: { value: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d12" } });
    fireEvent.click(screen.getByRole("button", { name: "Prepare refund request" }));
    await screen.findByText(/Your refund request is ready for review\./);

    fireEvent.click(screen.getByRole("button", { name: "Connect wallet" }));
    const claimWalletButton = await screen.findByRole("button", { name: "Reconnect wallet" });
    expect(claimWalletButton.parentElement?.textContent).toContain(proposer.publicKey.toBase58());

    fireEvent.click(screen.getByRole("button", { name: "Sign as proposer" }));
    await screen.findByText(/Proposer signature collected\./);
    (window.solana as TestWallet).current = refundFunder;
    fireEvent.click(screen.getByRole("button", { name: "Sign as refund funder" }));
    await screen.findByText(/Both wallet signatures are present in the local transaction artifact\./);

    fireEvent.click(screen.getByRole("button", { name: "Send approved refund" }));
    await screen.findByText(/The provider accepted the unchanged transaction bytes\./);
    expect(screen.getByText(/transport evidence only/)).toBeTruthy();
    expect(calls.some(({ url }) => url === "/api/v1/settlement/refund-claims/submit")).toBe(true);
    expect((window.solana as TestWallet).signTransaction).toHaveBeenCalledTimes(2);
  });

  it("prepares, collects both execute signatures, and remains proof-pending after submit", async () => {
    const approver = Keypair.fromSeed(new Uint8Array(32).fill(93));
    const refundFunder = Keypair.fromSeed(new Uint8Array(32).fill(94));
    const calls = installWalletAndFetchMock(approver, refundFunder, executeResponse(approver, refundFunder), "execute");

    render(<RefundExecutePanel apiBase="/api" serviceOnline accessToken="test-token" locale="en-US" />);
    fireEvent.change(screen.getByLabelText("Finalized refund request reference"), { target: { value: claimIntentId } });
    fireEvent.click(screen.getByRole("button", { name: "Prepare refund release" }));
    await screen.findByText(/The refund release is ready for review/);

    fireEvent.click(screen.getByRole("button", { name: "Connect wallet" }));
    const executeWalletButton = await screen.findByRole("button", { name: "Reconnect wallet" });
    expect(executeWalletButton.parentElement?.textContent).toContain(approver.publicKey.toBase58());

    fireEvent.click(screen.getByRole("button", { name: "Sign as approver" }));
    await screen.findByText(/Approver signature collected\./);
    (window.solana as TestWallet).current = refundFunder;
    fireEvent.click(screen.getByRole("button", { name: "Sign as refund funder" }));
    await screen.findByText(/Both execute wallet signatures are present in local memory\./);

    fireEvent.click(screen.getByRole("button", { name: "Send approved refund" }));
    await screen.findByText(/The provider accepted the unchanged execute transaction\./);
    expect(screen.getByText(/Proof pending: provider acknowledgement is not a completed refund/)).toBeTruthy();
    expect(calls.some(({ url }) => url === "/api/v1/settlement/refund-executions/submit")).toBe(true);
    expect((window.solana as TestWallet).signTransaction).toHaveBeenCalledTimes(2);
  });

  it("renders typed finalized claim-event proof instead of implying completion from a signature", async () => {
    const proposer = Keypair.fromSeed(new Uint8Array(32).fill(95));
    const refundFunder = Keypair.fromSeed(new Uint8Array(32).fill(96));
    installWalletAndFetchMock(proposer, refundFunder, finalizedClaimResponse(proposer, refundFunder), "claim");

    render(<RefundClaimPanel apiBase="/api" serviceOnline accessToken="test-token" locale="en-US" />);
    fireEvent.change(screen.getByLabelText("Approved correction reference"), { target: { value: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d12" } });
    fireEvent.click(screen.getByRole("button", { name: "Prepare refund request" }));

    await screen.findByText(/Finalized claim event evidence matched/);
    expect(screen.getByText("AdjustmentRefundClaimCreated · v1")).toBeTruthy();
    expect(screen.getByText("77")).toBeTruthy();
    expect(screen.getByText("Event index").parentElement?.textContent).toContain("2");
    expect(screen.getAllByText("100").length).toBeGreaterThan(0);
    expect(screen.getByText("Evidence digest").parentElement?.textContent).toContain("0101010101010101");
  });

  it("redacts an injected-wallet exception from the refund claim surface", async () => {
    const proposer = Keypair.fromSeed(new Uint8Array(32).fill(97));
    const refundFunder = Keypair.fromSeed(new Uint8Array(32).fill(98));
    installWalletAndFetchMock(proposer, refundFunder, claimResponse(proposer, refundFunder), "claim");

    render(<RefundClaimPanel apiBase="/api" serviceOnline accessToken="test-token" locale="en-US" />);
    fireEvent.change(screen.getByLabelText("Approved correction reference"), { target: { value: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d12" } });
    fireEvent.click(screen.getByRole("button", { name: "Prepare refund request" }));
    await screen.findByText(/Your refund request is ready for review\./);

    (window.solana as TestWallet).signTransaction = vi.fn(async () => {
      throw new Error("provider-internal refund signing detail");
    });
    fireEvent.click(screen.getByRole("button", { name: "Connect wallet" }));
    await screen.findByRole("button", { name: "Reconnect wallet" });
    fireEvent.click(screen.getByRole("button", { name: "Sign as proposer" }));

    const alert = await screen.findByRole("alert");
    expect(alert.textContent).toContain("Refund request wallet signing failed.");
    expect(alert.textContent).not.toContain("provider-internal refund signing detail");
  });
});

type TestWallet = {
  current: Keypair;
  connect: () => Promise<{ publicKey: Keypair["publicKey"] }>;
  signTransaction: (transaction: Transaction) => Promise<Transaction>;
};

type RequestCall = { url: string; body: unknown };

function installWalletAndFetchMock(
  initialWallet: Keypair,
  secondWallet: Keypair,
  response: Record<string, unknown>,
  kind: "claim" | "execute",
): RequestCall[] {
  vi.stubEnv("VITE_SOLANA_RPC_URL", rpcUrl);
  vi.spyOn(Connection.prototype, "getLatestBlockhash").mockResolvedValue({ blockhash: rpcBlockhash, lastValidBlockHeight: 1000 });
  const wallet: TestWallet = {
    current: initialWallet,
    connect: vi.fn(async () => ({ publicKey: wallet.current.publicKey })),
    signTransaction: vi.fn(async (transaction: Transaction) => {
      transaction.partialSign(wallet.current);
      return transaction;
    }),
  };
  void secondWallet;
  window.solana = wallet;
  const calls: RequestCall[] = [];
  vi.stubGlobal("fetch", vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
    const url = String(input);
    const body = init?.body ? JSON.parse(String(init.body)) : null;
    calls.push({ url, body });
    if (url === rpcUrl) {
      return jsonResponse({ jsonrpc: "2.0", id: 1, result: { context: { slot: 1 }, value: { blockhash: rpcBlockhash, lastValidBlockHeight: 1000 } } });
    }
    const collectionPath = kind === "claim" ? "/v1/settlement/refund-claims" : "/v1/settlement/refund-executions";
    if (url.endsWith(collectionPath)) return jsonResponse(response);
    if (url.endsWith(`${collectionPath}/submit`)) {
      return jsonResponse({ intent_id: kind === "claim" ? claimIntentId : executeIntentId, status: "submitted", signature: "5VfY7WmYcR7h9o9E9nP7pQ8rS", provider_index: 0 });
    }
    throw new Error(`Unexpected test request: ${url}`);
  }));
  return calls;
}

function claimResponse(proposer: Keypair, refundFunder: Keypair): Record<string, unknown> {
  const instruction = {
    ...vector.expected,
    accounts: vector.expected.accounts.map((account, index) => index === 0
      ? { ...account, pubkey: proposer.publicKey.toBase58() }
      : index === 1
        ? { ...account, pubkey: refundFunder.publicKey.toBase58() }
        : account),
  };
  return {
    status: "prepared_unsubmitted",
    intent_id: claimIntentId,
    request_digest: Array(32).fill(3),
    instruction_digest: Array(32).fill(1),
    artifact_digest: Array(32).fill(2),
    claim: instruction.claim,
    artifact: {
      schema: "refund-movement-artifact/v1",
      instruction,
      instruction_digest: Array(32).fill(1),
      artifact_digest: Array(32).fill(2),
      refund_destination: instruction.accounts[13].pubkey,
      merchant_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d11",
      adjustment_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d12",
      funding_configuration_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d13",
      receipt_signature: "5VfY7WmYcR7h9o9E9nP7pQ8rS",
      receipt_slot: "42",
      proposer_subject: "proposer",
      refund_funder_subject: "refund-funder",
      proposer: proposer.publicKey.toBase58(),
      refund_funder: refundFunder.publicKey.toBase58(),
      merchant: instruction.accounts[2].pubkey,
      mint: instruction.accounts[3].pubkey,
      token_program: instruction.accounts[4].pubkey,
      tax_vault: instruction.accounts[9].pubkey,
      net_source: instruction.accounts[11].pubkey,
      gross_atomic: "100",
      tax_atomic: "25",
      net_atomic: "75",
      execute_after_unix: 1800000000,
      expires_at_unix: 1800000600,
    },
    signed_transaction_digest: null,
    signature: null,
    submitted_slot: null,
    commitment: null,
    last_error_code: null,
  };
}

function finalizedClaimResponse(proposer: Keypair, refundFunder: Keypair): Record<string, unknown> {
  const response = claimResponse(proposer, refundFunder);
  return {
    ...response,
    status: "finalized",
    signature: "5VfY7WmYcR7h9o9E9nP7pQ8rS",
    submitted_slot: "77",
    commitment: "finalized",
    finalized_evidence: {
      event_type: "AdjustmentRefundClaimCreated",
      event_version: "v1",
      event_discriminator: Array(8).fill(4),
      commitment: "finalized",
      signature: "5VfY7WmYcR7h9o9E9nP7pQ8rS",
      slot: "77",
      event_index: 2,
      claim: Array.from(new PublicKey(vector.expected.claim).toBytes()),
      adjustment: Array(32).fill(1),
      original_receipt: Array(32).fill(2),
      merchant: Array(32).fill(3),
      mint: Array(32).fill(4),
      nonce: "3",
      gross_magnitude: "100",
      tax_magnitude: "25",
      net_magnitude: "75",
      refund_funder: Array(32).fill(5),
      refund_destination: Array(32).fill(6),
      proposer: Array(32).fill(7),
      reference_digest: Array(32).fill(8),
      execute_after_unix: 1800000000,
      expires_at_unix: 1800000600,
      evidence_digest: Array(32).fill(1),
    },
  };
}

function executeResponse(approver: Keypair, refundFunder: Keypair): Record<string, unknown> {
  const flags: Array<[boolean, boolean]> = [
    [true, false], [true, false], [false, false], [false, false], [false, false],
    [false, false], [false, false], [false, false], [false, false], [false, true],
    [false, false], [false, true], [false, true], [false, true],
  ];
  const instruction = {
    ...vector.expected,
    instruction_data_base64: "nChphaQNsfMDAAAAAAAAAA==",
    accounts: vector.expected.accounts.slice(0, 14).map((account, index) => ({
      ...account,
      pubkey: index === 0 ? approver.publicKey.toBase58() : index === 1 ? refundFunder.publicKey.toBase58() : account.pubkey,
      is_signer: flags[index][0],
      is_writable: flags[index][1],
    })),
  };
  return {
    status: "prepared_unsubmitted",
    intent_id: executeIntentId,
    request_digest: Array(32).fill(3),
    instruction_digest: Array(32).fill(1),
    artifact_digest: Array(32).fill(2),
    claim: instruction.claim,
    artifact: {
      schema: "refund-movement-artifact/v1",
      instruction,
      instruction_digest: Array(32).fill(1),
      artifact_digest: Array(32).fill(2),
      refund_destination: instruction.accounts[12].pubkey,
      merchant_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d11",
      adjustment_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d12",
      funding_configuration_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d13",
      receipt_signature: "5VfY7WmYcR7h9o9E9nP7pQ8rS",
      receipt_slot: "42",
      claim_signature: "5VfY7WmYcR7h9o9E9nP7pQ8rT",
      claim_slot: "43",
      authority_subject: approver.publicKey.toBase58(),
      proposer_subject: "",
      refund_funder_subject: "refund-funder",
      proposer: "",
      refund_funder: refundFunder.publicKey.toBase58(),
      merchant: instruction.accounts[2].pubkey,
      mint: instruction.accounts[3].pubkey,
      token_program: instruction.accounts[4].pubkey,
      tax_vault: instruction.accounts[9].pubkey,
      net_source: instruction.accounts[11].pubkey,
      gross_atomic: "100",
      tax_atomic: "25",
      net_atomic: "75",
      execute_after_unix: null,
      expires_at_unix: null,
    },
    signed_transaction_digest: null,
    signature: null,
    submitted_slot: null,
    commitment: null,
    last_error_code: null,
  };
}

function jsonResponse(value: unknown): Response {
  return new Response(JSON.stringify(value), { status: 200, headers: { "content-type": "application/json" } });
}
