import { PublicKey, Transaction } from "@solana/web3.js";
import { describe, expect, it } from "vitest";
import {
  parseRefundCancellationResponse,
  parseRefundCancellationSubmissionResponse,
  refundCancellationHasSignature,
} from "./refundCancellation";

const program = new PublicKey("4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi");
const governance = new PublicKey(new Uint8Array(32).fill(11));
const merchant = new PublicKey(new Uint8Array(32).fill(12));
const mint = new PublicKey(new Uint8Array(32).fill(13));
const adjustment = new PublicKey(new Uint8Array(32).fill(14));
const [registry] = PublicKey.findProgramAddressSync([Buffer.from("adjustment-registry"), merchant.toBytes(), mint.toBytes()], program);
const [claim] = PublicKey.findProgramAddressSync([Buffer.from("refund-claim"), adjustment.toBytes(), Buffer.from(new Uint8Array([42, 0, 0, 0, 0, 0, 0, 0]))], program);

function response(): Record<string, unknown> {
  const data = Buffer.alloc(16);
  Buffer.from([166, 180, 224, 166, 216, 62, 123, 153]).copy(data);
  data.writeBigUInt64LE(42n, 8);
  const accounts = [
    { pubkey: governance.toBase58(), is_signer: true, is_writable: true },
    { pubkey: merchant.toBase58(), is_signer: false, is_writable: false },
    { pubkey: mint.toBase58(), is_signer: false, is_writable: false },
    { pubkey: registry.toBase58(), is_signer: false, is_writable: false },
    { pubkey: claim.toBase58(), is_signer: false, is_writable: true },
  ];
  const artifact = {
    schema: "refund-claim-cancellation-artifact/v1",
    instruction: {
      status: "prepared_unsubmitted",
      program_id: program.toBase58(),
      instruction_data_base64: data.toString("base64"),
      accounts,
      registry: registry.toBase58(),
      claim: claim.toBase58(),
      claim_nonce: "42",
    },
    instruction_digest: Array(32).fill(1),
    artifact_digest: Array(32).fill(2),
    merchant_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d11",
    claim_intent_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d12",
    adjustment_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d13",
    merchant: merchant.toBase58(),
    mint: mint.toBase58(),
    adjustment: adjustment.toBase58(),
    governance_authority: governance.toBase58(),
    governance_subject: "governance-user",
  };
  return {
    status: "prepared_unsubmitted",
    intent_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d14",
    request_digest: Array(32).fill(3),
    instruction_digest: Array(32).fill(1),
    artifact_digest: Array(32).fill(2),
    claim: claim.toBase58(),
    artifact,
    signed_transaction_digest: null,
    signature: null,
    submitted_slot: null,
    commitment: null,
    last_error_code: null,
  };
}

describe("refund cancellation wallet boundary", () => {
  it("accepts only the canonical governance-only artifact topology", () => {
    const parsed = parseRefundCancellationResponse(response());
    expect(parsed.artifact.instruction.claim_nonce).toBe("42");
    expect(parsed.artifact.instruction.accounts).toHaveLength(5);
    expect(parsed.artifact.instruction.accounts[0].pubkey).toBe(governance.toBase58());
    expect(parsed.artifact.instruction.accounts[4].pubkey).toBe(claim.toBase58());
  });

  it("rejects account substitution before wallet signing", () => {
    const candidate = response();
    const artifact = candidate.artifact as { instruction: { accounts: Array<{ pubkey: string }> } };
    artifact.instruction.accounts[3].pubkey = new PublicKey(new Uint8Array(32).fill(99)).toBase58();
    expect(() => parseRefundCancellationResponse(candidate)).toThrow(/topology|registry/i);
  });

  it("does not accept an unsigned transaction as submission evidence", () => {
    const transaction = new Transaction({ feePayer: governance, recentBlockhash: new PublicKey(new Uint8Array(32).fill(15)).toBase58() });
    const bytes = transaction.serialize({ requireAllSignatures: false, verifySignatures: false });
    expect(refundCancellationHasSignature(Buffer.from(bytes).toString("base64"))).toBe(false);
  });

  it("rejects an unbounded provider index at the browser boundary", () => {
    expect(() => parseRefundCancellationSubmissionResponse({
      intent_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d14",
      status: "submitted",
      signature: "cancellation-signature",
      provider_index: 99,
    })).toThrow(/provider index/i);
  });
});
