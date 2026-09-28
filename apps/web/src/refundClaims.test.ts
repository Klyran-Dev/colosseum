import { describe, expect, it } from "vitest";
import vector from "../../../test-vectors/settlement-refund-claim-v1.json";
import { parseRefundClaimInstruction, parseRefundClaimResponse } from "./refundClaims";

const artifact = {
  schema: "refund-movement-artifact/v1",
  instruction: vector.expected,
  instruction_digest: Array(32).fill(1),
  artifact_digest: Array(32).fill(2),
  refund_destination: vector.expected.accounts[12].pubkey,
  merchant_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d11",
  adjustment_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d12",
  funding_configuration_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d13",
  receipt_signature: "5VfY7WmYcR7h9o9E9nP7x5qC2oW6v8m1f3H4jK6pQ8rS",
  receipt_slot: "42",
  proposer_subject: "proposer",
  refund_funder_subject: "refund-funder",
  proposer: vector.expected.accounts[0].pubkey,
  refund_funder: vector.expected.accounts[1].pubkey,
  merchant: vector.expected.accounts[2].pubkey,
  mint: vector.expected.accounts[3].pubkey,
  token_program: vector.expected.accounts[4].pubkey,
  tax_vault: vector.expected.accounts[9].pubkey,
  net_source: vector.expected.accounts[11].pubkey,
  gross_atomic: "100",
  tax_atomic: "25",
  net_atomic: "75",
  execute_after_unix: 1_800_000_000,
  expires_at_unix: 1_800_000_600,
};

const response = {
  status: "prepared_unsubmitted",
  intent_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d14",
  request_digest: Array(32).fill(3),
  instruction_digest: Array(32).fill(1),
  artifact_digest: Array(32).fill(2),
  claim: vector.expected.claim,
  artifact,
  signed_transaction_digest: null,
  signature: null,
  submitted_slot: null,
  commitment: null,
  last_error_code: null,
};

describe("refund claim wallet contract", () => {
  it("parses the shared claim vector and binds signer/account roles", () => {
    const parsed = parseRefundClaimInstruction(vector.expected);
    expect(parsed.accounts).toHaveLength(15);
    expect(parsed.claim_nonce).toBe("3");
    expect(parsed.accounts[0].is_signer).toBe(true);
    expect(parsed.accounts[1].is_signer).toBe(true);
    expect(parsed.accounts[13].pubkey).toBe(parsed.claim);
  });

  it("parses a prepared artifact with explicit nullable transport evidence", () => {
    const parsed = parseRefundClaimResponse(response);
    expect(parsed.status).toBe("prepared_unsubmitted");
    expect(parsed.artifact.gross_atomic).toBe("100");
    expect(parsed.signature).toBeNull();
    expect(parsed.submitted_slot).toBeNull();
  });

  it("rejects account substitution before a wallet prompt", () => {
    const tampered = structuredClone(vector.expected);
    [tampered.accounts[0], tampered.accounts[1]] = [tampered.accounts[1], tampered.accounts[0]];
    expect(() => parseRefundClaimInstruction(tampered)).toThrow(/account flags|topology/);
  });
});
