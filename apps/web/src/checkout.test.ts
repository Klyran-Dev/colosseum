import { describe, expect, it, vi } from "vitest";
import { Connection, PublicKey } from "@solana/web3.js";
import { Buffer } from "buffer";
import {
  buildSettlementTransaction,
  buildSettlementAdjustmentTransaction,
  parseCompiledSettlementArtifact,
  parseCompiledSettlementAdjustmentArtifact,
  parseCanonicalU64,
  parsePolicyDigest,
  parseSettlementRoutes,
  parseSettlementStatusResponse,
  parseSettlementAdjustmentChainIntentResponse,
  parseSettlementSplitPlans,
  signAndEncodeSettlement,
  settlementIntentAllowsRetry,
  validateCompiledSettlementInstruction,
  validateCompiledSettlementAdjustmentInstruction,
} from "./checkout";
import settlementVector from "../../../test-vectors/settlement-instruction-v1.json";
import adjustmentVector from "../../../test-vectors/settlement-adjustment-evidence-v1.json";

const payer = new PublicKey("11111111111111111111111111111111");
const vectorInstruction = {
  ...settlementVector.expected,
  instruction_data_base64: settlementVector.expected.instruction_data_base64_chunks.join(""),
  compute_budget_policy: {
    policy_version: 1,
    compute_unit_limit: 200_000,
    micro_lamports_per_compute_unit: 1_000,
  },
};
const instruction = {
  status: "prepared_unsubmitted",
  program_id: "11111111111111111111111111111111",
  instruction_data_base64: "AQID",
  accounts: [{ pubkey: payer.toBase58(), is_signer: true, is_writable: true }],
  protocol_config: payer.toBase58(),
  vault_authority: payer.toBase58(),
  policy_attestation: payer.toBase58(),
  receipt: payer.toBase58(),
  compute_budget_policy: {
    policy_version: 1,
    compute_unit_limit: 200_000,
    micro_lamports_per_compute_unit: 1_000,
  },
};

describe("checkout settlement boundary", () => {
  it("parses exactly one 32-byte policy digest", () => {
    expect(parsePolicyDigest("00".repeat(31) + "ff")).toEqual([...new Uint8Array(31), 255]);
    expect(() => parsePolicyDigest("00")).toThrow(/32 bytes/);
    expect(() => parsePolicyDigest("zz".repeat(32))).toThrow(/32 bytes/);
  });

  it("preserves canonical Solana u64 values as decimal strings", () => {
    expect(parseCanonicalU64("9007199254740992", "slot")).toBe("9007199254740992");
    expect(parseCanonicalU64("18446744073709551615", "slot")).toBe("18446744073709551615");
    expect(() => parseCanonicalU64(9007199254740992, "slot")).toThrow(/canonical unsigned/);
    expect(() => parseCanonicalU64("01", "slot")).toThrow(/canonical unsigned/);
    expect(() => parseCanonicalU64("18446744073709551616", "slot")).toThrow(/canonical unsigned/);
  });

  it("accepts only bounded UUIDv7 settlement intent status responses", () => {
    const parsed = parseSettlementStatusResponse({
      intent_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d13",
      status: "unknown",
      signature: null,
      provider_index: null,
    });
    expect(parsed.status).toBe("unknown");
    expect(parsed.intent_id).toMatch(/-7/);

    expect(() => parseSettlementStatusResponse({
      intent_id: "01992f1b-4a70-6e30-8ea2-b6e37a9d9d13",
      status: "unknown",
      signature: null,
      provider_index: null,
    })).toThrow(/UUIDv7/);
    expect(() => parseSettlementStatusResponse({
      intent_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d13",
      status: "finalized",
      signature: "",
      provider_index: 0,
    })).toThrow(/signature/);
    expect(() => parseSettlementStatusResponse({
      intent_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d13",
      status: "unknown",
      signature: null,
      provider_index: -1,
    })).toThrow(/provider index/);
  });

  it("allows retry only for pre-submission intent states", () => {
    expect(settlementIntentAllowsRetry("prepared_unsubmitted")).toBe(true);
    expect(settlementIntentAllowsRetry("signed")).toBe(true);
    expect(settlementIntentAllowsRetry("submitting")).toBe(false);
    for (const status of ["submitted", "processed", "confirmed", "finalized", "rejected", "unknown"] as const) {
      expect(settlementIntentAllowsRetry(status)).toBe(false);
    }
  });

  it("accepts a durable submitting state with its bound signature", () => {
    const parsed = parseSettlementStatusResponse({
      intent_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d13",
      status: "submitting",
      signature: "5".repeat(88),
      provider_index: null,
    });
    expect(parsed.status).toBe("submitting");
    expect(parsed.signature).toHaveLength(88);
    expect(() => parseSettlementStatusResponse({
      intent_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d13",
      status: "submitting",
      signature: null,
      provider_index: null,
    })).toThrow(/bound signature/);
  });

  it("requires a bound signature for adjustment reconciliation states", () => {
    const parsed = parseSettlementAdjustmentChainIntentResponse({
      chain_intent_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d13",
      status: "unknown",
      signature: "5".repeat(88),
      provider_index: 1,
    });
    expect(parsed.status).toBe("unknown");
    expect(() => parseSettlementAdjustmentChainIntentResponse({
      chain_intent_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d13",
      status: "finalized",
      signature: null,
      provider_index: null,
    })).toThrow(/bound signature/i);
  });

  it("validates bounded routes and compiled artifacts before review", () => {
    const routeId = "01992f1b-4a70-7e30-8ea2-b6e37a9d9d13";
    const [route] = parseSettlementRoutes([{
      id: routeId,
      route_key: "local-usdc",
      version: 1,
      merchant_account: vectorInstruction.accounts[2].pubkey,
      mint: vectorInstruction.accounts[4].pubkey,
      tax_vault: vectorInstruction.accounts[5].pubkey,
      net_destination: vectorInstruction.accounts[8].pubkey,
      token_program: vectorInstruction.accounts[10].pubkey,
    }]);
    expect(route.id).toBe(routeId);
    expect(route.token_program).toBe(vectorInstruction.accounts[10].pubkey);

    const artifact = parseCompiledSettlementArtifact({
      ...vectorInstruction,
      intent_id: routeId,
      request_digest: new Array(32).fill(1),
      instruction_digest: new Array(32).fill(2),
      route_id: routeId,
    });
    expect(artifact.intent_id).toBe(routeId);
    expect(artifact.route_id).toBe(routeId);

    const splitInstruction = {
      ...vectorInstruction,
      instruction_data_base64: Buffer.concat([
        Buffer.from("A6qY2V4JCdU=", "base64"),
        Buffer.alloc(80),
      ]).toString("base64"),
      accounts: [
        ...vectorInstruction.accounts.slice(0, 8),
        { ...vectorInstruction.accounts[7], is_writable: false },
        vectorInstruction.accounts[9],
        vectorInstruction.accounts[10],
        vectorInstruction.accounts[11],
        { ...vectorInstruction.accounts[8], is_writable: true },
        { ...vectorInstruction.accounts[2], is_writable: true },
      ],
      split_plan: vectorInstruction.accounts[7].pubkey,
      split_plan_digest: [25, ...new Array(31).fill(0)],
    };
    const splitArtifact = parseCompiledSettlementArtifact({
      ...splitInstruction,
      intent_id: routeId,
      request_digest: new Array(32).fill(1),
      instruction_digest: new Array(32).fill(2),
      route_id: routeId,
      split_plan_id: routeId,
    });
    expect("split_plan" in splitArtifact).toBe(true);

    expect(() => parseSettlementRoutes([{
      id: routeId,
      route_key: "bad-program",
      version: 1,
      merchant_account: vectorInstruction.accounts[2].pubkey,
      mint: vectorInstruction.accounts[4].pubkey,
      tax_vault: vectorInstruction.accounts[5].pubkey,
      net_destination: vectorInstruction.accounts[8].pubkey,
      token_program: "11111111111111111111111111111111",
    }])).toThrow(/token program/);
    expect(() => parseCompiledSettlementArtifact({
      ...vectorInstruction,
      intent_id: routeId,
      request_digest: new Array(31).fill(1),
      instruction_digest: new Array(32).fill(2),
      route_id: routeId,
    })).toThrow(/digest/);
    expect(() => parseCompiledSettlementArtifact({
      ...vectorInstruction,
      intent_id: routeId,
      request_digest: new Array(32).fill(1),
      instruction_digest: new Array(32).fill(2),
      route_id: routeId,
      accounts: vectorInstruction.accounts.map((account, index) => index === 5
        ? { ...account, pubkey: vectorInstruction.accounts[8].pubkey }
        : account),
    })).toThrow(/distinct/);
  });

  it("builds the governed compute-budget pair before settlement", async () => {
    const fakeConnection = {
      getLatestBlockhash: async () => ({ blockhash: payer.toBase58(), lastValidBlockHeight: 10 }),
    } as unknown as Connection;
    const vectorPayer = new PublicKey(settlementVector.request.payer);
    const transaction = await buildSettlementTransaction(vectorInstruction, vectorPayer, fakeConnection);
    expect(transaction.instructions).toHaveLength(3);
    expect(transaction.instructions[0].programId.toBase58()).toBe("ComputeBudget111111111111111111111111111111");
    expect(transaction.instructions[0].data).toEqual(Buffer.from([2, 64, 13, 3, 0]));
    expect(transaction.instructions[1].data).toEqual(Buffer.from([3, 232, 3, 0, 0, 0, 0, 0, 0]));
    expect(transaction.instructions[2].data.toString("base64")).toBe(vectorInstruction.instruction_data_base64);
    expect(transaction.feePayer?.equals(vectorPayer)).toBe(true);
  });

  it("matches the shared Rust settlement artifact vector exactly", async () => {
    const vectorPayer = new PublicKey(settlementVector.request.payer);
    const expectedInstruction = vectorInstruction;
    const transaction = await buildSettlementTransaction(
      expectedInstruction,
      vectorPayer,
      {
        getLatestBlockhash: async () => ({
          blockhash: payer.toBase58(),
          lastValidBlockHeight: 10,
        }),
      } as unknown as Connection,
    );
    const compiled = transaction.instructions[2];

    expect(compiled.programId.toBase58()).toBe(expectedInstruction.program_id);
    expect(compiled.data.toString("base64")).toBe(
      expectedInstruction.instruction_data_base64,
    );
    expect(compiled.keys).toEqual(
      expectedInstruction.accounts.map((account) => ({
        pubkey: new PublicKey(account.pubkey),
        isSigner: account.is_signer,
        isWritable: account.is_writable,
      })),
    );
  });

  it("rejects account substitution before a wallet signing prompt", () => {
    const substituted = {
      ...vectorInstruction,
      accounts: vectorInstruction.accounts.map((account, index) =>
        index === 5 ? { ...account, pubkey: vectorInstruction.accounts[8].pubkey } : account,
      ),
    };
    expect(() => validateCompiledSettlementInstruction(
      substituted,
      new PublicKey(settlementVector.request.payer),
    )).toThrow(/distinct/);
  });

  it("validates and builds the bounded Split Plan account envelope", async () => {
    const splitInstruction = {
      status: "prepared_unsubmitted",
      program_id: vectorInstruction.program_id,
      instruction_data_base64: Buffer.concat([
        Buffer.from("A6qY2V4JCdU=", "base64"),
        Buffer.alloc(80),
      ]).toString("base64"),
      accounts: [
        ...vectorInstruction.accounts.slice(0, 8),
        { ...vectorInstruction.accounts[7], is_writable: false },
        vectorInstruction.accounts[9],
        vectorInstruction.accounts[10],
        vectorInstruction.accounts[11],
        { ...vectorInstruction.accounts[8], is_writable: true },
        { ...vectorInstruction.accounts[2], is_writable: true },
      ],
      protocol_config: vectorInstruction.accounts[1].pubkey,
      vault_authority: vectorInstruction.accounts[6].pubkey,
      policy_attestation: vectorInstruction.accounts[7].pubkey,
      split_plan: vectorInstruction.accounts[7].pubkey,
      split_plan_digest: [25, ...new Array(31).fill(0)],
      receipt: vectorInstruction.accounts[9].pubkey,
      compute_budget_policy: vectorInstruction.compute_budget_policy,
    };
    const vectorPayer = new PublicKey(settlementVector.request.payer);
    validateCompiledSettlementInstruction(splitInstruction, vectorPayer);
    const transaction = await buildSettlementTransaction(
      splitInstruction,
      vectorPayer,
      {
        getLatestBlockhash: async () => ({
          blockhash: payer.toBase58(),
          lastValidBlockHeight: 10,
        }),
      } as unknown as Connection,
    );
    expect(transaction.instructions[2].keys).toHaveLength(14);
    expect(transaction.instructions[2].data).toHaveLength(88);
  });

  it("rejects a Split Plan with a recipient substituted for the source account", () => {
    const splitInstruction = {
      status: "prepared_unsubmitted",
      program_id: vectorInstruction.program_id,
      instruction_data_base64: Buffer.concat([
        Buffer.from("A6qY2V4JCdU=", "base64"),
        Buffer.alloc(80),
      ]).toString("base64"),
      accounts: [
        ...vectorInstruction.accounts.slice(0, 8),
        { ...vectorInstruction.accounts[7], is_writable: false },
        vectorInstruction.accounts[9],
        vectorInstruction.accounts[10],
        vectorInstruction.accounts[11],
        { ...vectorInstruction.accounts[3], is_writable: true },
      ],
      protocol_config: vectorInstruction.accounts[1].pubkey,
      vault_authority: vectorInstruction.accounts[6].pubkey,
      policy_attestation: vectorInstruction.accounts[7].pubkey,
      split_plan: vectorInstruction.accounts[7].pubkey,
      split_plan_digest: [25, ...new Array(31).fill(0)],
      receipt: vectorInstruction.accounts[9].pubkey,
      compute_budget_policy: vectorInstruction.compute_budget_policy,
    };
    expect(() => validateCompiledSettlementInstruction(
      splitInstruction,
      new PublicKey(settlementVector.request.payer),
    )).toThrow(/distinct/);
  });

  it("rejects an API Split Plan response that does not conserve recipients", () => {
    const payload = [{
      id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d13",
      plan_key: "local-synthetic-split",
      version: 1,
      merchant_account: vectorInstruction.accounts[2].pubkey,
      mint: vectorInstruction.accounts[4].pubkey,
      plan_digest: new Array(32).fill(0),
      recipients: [{
        token_account: vectorInstruction.accounts[8].pubkey,
        basis_points: 9_999,
      }],
    }];
    expect(() => parseSettlementSplitPlans(payload)).toThrow(/conserving/);
  });

  it("parses the bounded adjustment compile artifact and rejects signer substitution", () => {
    const id = "01992f1b-4a70-7e30-8ea2-b6e37a9d9d13";
    const artifact = parseCompiledSettlementAdjustmentArtifact({
      status: "prepared_unsubmitted",
      intent_id: id,
      chain_intent_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d15",
      request_digest: new Array(32).fill(1),
      instruction_digest: new Array(32).fill(2),
      registry: adjustmentVector.expected.adjustment_registry,
      registry_slot: "12",
      registry_commitment: "finalized",
      registry_account_digest: new Array(32).fill(3),
      instruction: adjustmentVector.expected,
    });
    expect(artifact.chain_intent_id).toBe("01992f1b-4a70-7e30-8ea2-b6e37a9d9d15");
    expect(artifact.instruction.accounts).toHaveLength(10);
    expect(() => validateCompiledSettlementAdjustmentInstruction(
      artifact.instruction,
      new PublicKey(adjustmentVector.expected.accounts[1].pubkey),
    )).toThrow(/approver|signer/i);
    expect(() => parseCompiledSettlementAdjustmentArtifact({
      status: "prepared_unsubmitted",
      intent_id: id,
      chain_intent_id: id,
      request_digest: new Array(32).fill(1),
      instruction_digest: new Array(32).fill(2),
      registry: adjustmentVector.expected.adjustment_registry,
      registry_slot: "12",
      registry_commitment: "confirmed",
      registry_account_digest: new Array(32).fill(3),
      instruction: adjustmentVector.expected,
    })).toThrow(/finalized/i);
  });

  it("builds a record-only adjustment transaction without compute-budget injection", async () => {
    const approver = new PublicKey(adjustmentVector.expected.accounts[0].pubkey);
    const typedInstruction = parseCompiledSettlementAdjustmentArtifact({
      status: "prepared_unsubmitted",
      intent_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d13",
      chain_intent_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d15",
      request_digest: new Array(32).fill(1),
      instruction_digest: new Array(32).fill(2),
      registry: adjustmentVector.expected.adjustment_registry,
      registry_slot: "12",
      registry_commitment: "finalized",
      registry_account_digest: new Array(32).fill(3),
      instruction: adjustmentVector.expected,
    }).instruction;
    const connection = {
      getLatestBlockhash: vi.fn().mockResolvedValue({ blockhash: approver.toBase58() }),
    } as unknown as Connection;
    const transaction = await buildSettlementAdjustmentTransaction(typedInstruction, approver, connection);
    expect(transaction.instructions).toHaveLength(1);
    expect(transaction.instructions[0].programId.toBase58()).toBe(adjustmentVector.expected.program_id);
    expect(transaction.feePayer?.toBase58()).toBe(approver.toBase58());
  });

  it("fails closed when no injected wallet is available", async () => {
    await expect(signAndEncodeSettlement(instruction, payer, "http://127.0.0.1:8899"))
      .rejects.toThrow(/wallet/);
  });
});
