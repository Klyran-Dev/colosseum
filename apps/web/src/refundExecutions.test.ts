import { describe, expect, it } from "vitest";
import vector from "../../../test-vectors/settlement-refund-claim-v1.json";
import { parseRefundExecuteInstruction, parseRefundExecuteResponse } from "./refundExecutions";

const expectedFlags: Array<[boolean, boolean]> = [
  [true, false], [true, false], [false, false], [false, false], [false, false],
  [false, false], [false, false], [false, false], [false, false], [false, true],
  [false, false], [false, true], [false, true], [false, true],
];
const executionInstruction = {
  ...vector.expected,
  instruction_data_base64: "nChphaQNsfMDAAAAAAAAAA==",
  accounts: vector.expected.accounts.slice(0, 14).map((account, index) => ({
    ...account,
    is_signer: expectedFlags[index][0],
    is_writable: expectedFlags[index][1],
  })),
};

const response = {
  status: "prepared_unsubmitted",
  intent_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d14",
  request_digest: Array(32).fill(3),
  instruction_digest: Array(32).fill(1),
  artifact_digest: Array(32).fill(2),
  claim: executionInstruction.claim,
  artifact: {
    schema: "refund-movement-artifact/v1",
    instruction: executionInstruction,
    instruction_digest: Array(32).fill(1),
    artifact_digest: Array(32).fill(2),
    refund_destination: executionInstruction.accounts[12].pubkey,
    merchant_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d11",
    adjustment_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d12",
    funding_configuration_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d13",
    receipt_signature: "5VfY7WmYcR7h9o9E9nP7x5qC2oW6v8m1f3H4jK6pQ8rS",
    receipt_slot: "42",
    claim_signature: "5VfY7WmYcR7h9o9E9nP7x5qC2oW6v8m1f3H4jK6pQ8rT",
    claim_slot: "43",
    authority_subject: executionInstruction.accounts[0].pubkey,
    proposer_subject: "",
    refund_funder_subject: "refund-funder",
    proposer: "",
    refund_funder: executionInstruction.accounts[1].pubkey,
    merchant: executionInstruction.accounts[2].pubkey,
    mint: executionInstruction.accounts[3].pubkey,
    token_program: executionInstruction.accounts[4].pubkey,
    tax_vault: executionInstruction.accounts[9].pubkey,
    net_source: executionInstruction.accounts[11].pubkey,
    gross_atomic: null,
    tax_atomic: null,
    net_atomic: null,
    execute_after_unix: null,
    expires_at_unix: null,
  },
  signed_transaction_digest: null,
  signature: null,
  submitted_slot: null,
  commitment: null,
  last_error_code: null,
};

describe("refund execute wallet contract", () => {
  it("parses the fixed fourteen-account execute topology", () => {
    const parsed = parseRefundExecuteInstruction(executionInstruction);
    expect(parsed.accounts).toHaveLength(14);
    expect(parsed.accounts[0].is_signer).toBe(true);
    expect(parsed.accounts[1].is_signer).toBe(true);
    expect(parsed.accounts[13].pubkey).toBe(parsed.claim);
  });

  it("parses finalized claim evidence and nullable transport state", () => {
    const parsed = parseRefundExecuteResponse(response);
    expect(parsed.artifact.claim_signature).toBeTruthy();
    expect(parsed.artifact.claim_slot).toBe("43");
    expect(parsed.signature).toBeNull();
  });

  it("rejects a writable or signer privilege substitution before wallet access", () => {
    const tampered = structuredClone(executionInstruction);
    tampered.accounts[9].is_writable = false;
    expect(() => parseRefundExecuteInstruction(tampered)).toThrow(/flags/);
  });
});
