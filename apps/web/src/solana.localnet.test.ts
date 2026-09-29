import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  ExtensionType,
  TOKEN_2022_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
  createInitializeMintInstruction,
  createInitializeTransferFeeConfigInstruction,
  createMint,
  getAccount,
  getMintLen,
  getOrCreateAssociatedTokenAccount,
  mintTo,
} from "@solana/spl-token";
import {
  Connection,
  AddressLookupTableAccount,
  AddressLookupTableProgram,
  ComputeBudgetProgram,
  Keypair,
  PublicKey,
  sendAndConfirmTransaction,
  SystemProgram,
  Transaction,
  TransactionInstruction,
  TransactionMessage,
  VersionedTransaction,
} from "@solana/web3.js";
import { Buffer } from "buffer";

const enabled = process.env.SOLANA_LOCALNET_TESTS === "true";
const endpoint = process.env.SOLANA_RPC_URL ?? "http://127.0.0.1:8899";
const programId = new PublicKey("4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi");
const systemProgram = SystemProgram.programId;

const payer = Keypair.generate();
const merchant = Keypair.generate();
const issuer = Keypair.generate();
const releaseGovernance = Keypair.generate();
const releaseProposer = Keypair.generate();
const releaseApprover = Keypair.generate();
const adjustmentProposer = Keypair.generate();
const adjustmentApprover = Keypair.generate();
const refundFunder = Keypair.generate();
const splitRecipientOwnerA = Keypair.generate();
const splitRecipientOwnerB = Keypair.generate();
const connection = new Connection(endpoint, "confirmed");

const digest = Buffer.alloc(32, 7);
const gross = 100n;
const tax = 25n;
const net = 75n;

function discriminator(name: string): Buffer {
  return createHash("sha256").update(`global:${name}`).digest().subarray(0, 8);
}

function u64(value: bigint): Buffer {
  const result = Buffer.alloc(8);
  result.writeBigUInt64LE(value, 0);
  return result;
}

function u16(value: number): Buffer {
  const result = Buffer.alloc(2);
  result.writeUInt16LE(value, 0);
  return result;
}

function u32(value: number): Buffer {
  const result = Buffer.alloc(4);
  result.writeUInt32LE(value, 0);
  return result;
}

function i64(value: bigint): Buffer {
  const result = Buffer.alloc(8);
  result.writeBigInt64LE(value, 0);
  return result;
}

function pda(...seeds: (Buffer | Uint8Array)[]): PublicKey {
  return PublicKey.findProgramAddressSync(seeds, programId)[0];
}

function protocolConfig(): PublicKey {
  return pda(Buffer.from("protocol-config"));
}

function vaultAuthority(mint: PublicKey): PublicKey {
  return pda(Buffer.from("vault-authority"), merchant.publicKey.toBuffer(), mint.toBuffer());
}

function issuerRegistry(mint: PublicKey): PublicKey {
  return pda(
    Buffer.from("policy-issuer-registry"),
    merchant.publicKey.toBuffer(),
    mint.toBuffer(),
  );
}

function releaseRegistry(mint: PublicKey): PublicKey {
  return pda(
    Buffer.from("vault-release-registry"),
    merchant.publicKey.toBuffer(),
    mint.toBuffer(),
  );
}

function adjustmentRegistry(mint: PublicKey): PublicKey {
  return pda(
    Buffer.from("adjustment-registry"),
    merchant.publicKey.toBuffer(),
    mint.toBuffer(),
  );
}

function adjustmentEvidence(mint: PublicKey, settlementNonce: bigint, adjustmentNonce: bigint): PublicKey {
  return pda(Buffer.from("adjustment"), receipt(mint, settlementNonce).toBuffer(), u64(adjustmentNonce));
}

function refundClaim(mint: PublicKey, settlementNonce: bigint, adjustmentNonce: bigint, claimNonce: bigint): PublicKey {
  return pda(
    Buffer.from("refund-claim"),
    adjustmentEvidence(mint, settlementNonce, adjustmentNonce).toBuffer(),
    u64(claimNonce),
  );
}

function releaseClaim(mint: PublicKey, nonce: bigint): PublicKey {
  return pda(
    Buffer.from("release-claim"),
    merchant.publicKey.toBuffer(),
    mint.toBuffer(),
    u64(nonce),
  );
}

function policyAttestation(mint: PublicKey): PublicKey {
  return pda(
    Buffer.from("policy-attestation"),
    merchant.publicKey.toBuffer(),
    mint.toBuffer(),
    u64(1n),
  );
}

function splitPlan(mint: PublicKey, version = 1n): PublicKey {
  return pda(
    Buffer.from("split-plan"),
    merchant.publicKey.toBuffer(),
    mint.toBuffer(),
    u64(version),
  );
}

function receipt(mint: PublicKey, nonce: bigint): PublicKey {
  return pda(
    Buffer.from("receipt"),
    payer.publicKey.toBuffer(),
    merchant.publicKey.toBuffer(),
    u64(nonce),
  );
}

function instruction(
  name: string,
  keys: { pubkey: PublicKey; isSigner: boolean; isWritable: boolean }[],
  data: Buffer,
): TransactionInstruction {
  return new TransactionInstruction({ programId, keys, data: Buffer.concat([discriminator(name), data]) });
}

function initializeRegistryInstruction(mint: PublicKey, tokenProgram: PublicKey): TransactionInstruction {
  return instruction(
    "initialize_policy_issuer_registry",
    [
      { pubkey: payer.publicKey, isSigner: true, isWritable: true },
      { pubkey: merchant.publicKey, isSigner: false, isWritable: false },
      { pubkey: mint, isSigner: false, isWritable: false },
      { pubkey: tokenProgram, isSigner: false, isWritable: false },
      { pubkey: issuerRegistry(mint), isSigner: false, isWritable: true },
      { pubkey: systemProgram, isSigner: false, isWritable: false },
    ],
    issuer.publicKey.toBuffer(),
  );
}

function registerPolicyInstruction(mint: PublicKey, tokenProgram: PublicKey): TransactionInstruction {
  return instruction(
    "register_policy_attestation",
    [
      { pubkey: issuer.publicKey, isSigner: true, isWritable: true },
      { pubkey: merchant.publicKey, isSigner: false, isWritable: false },
      { pubkey: mint, isSigner: false, isWritable: false },
      { pubkey: tokenProgram, isSigner: false, isWritable: false },
      { pubkey: issuerRegistry(mint), isSigner: false, isWritable: false },
      { pubkey: policyAttestation(mint), isSigner: false, isWritable: true },
      { pubkey: systemProgram, isSigner: false, isWritable: false },
    ],
    Buffer.concat([u64(1n), u64(gross), u64(tax), u64(net), digest, i64(BigInt(Math.floor(Date.now() / 1000) + 3600))]),
  );
}

function revokePolicyInstruction(mint: PublicKey): TransactionInstruction {
  return instruction(
    "revoke_policy_attestation",
    [
      { pubkey: payer.publicKey, isSigner: true, isWritable: false },
      { pubkey: policyAttestation(mint), isSigner: false, isWritable: true },
      { pubkey: issuerRegistry(mint), isSigner: false, isWritable: false },
    ],
    Buffer.alloc(0),
  );
}

function settleInstruction(
  mint: PublicKey,
  source: PublicKey,
  taxVault: PublicKey,
  destination: PublicKey,
  tokenProgram: PublicKey,
  nonce: bigint,
): TransactionInstruction {
  return instruction(
    "settle",
    [
      { pubkey: payer.publicKey, isSigner: true, isWritable: true },
      { pubkey: protocolConfig(), isSigner: false, isWritable: false },
      { pubkey: merchant.publicKey, isSigner: false, isWritable: false },
      { pubkey: source, isSigner: false, isWritable: true },
      { pubkey: mint, isSigner: false, isWritable: false },
      { pubkey: taxVault, isSigner: false, isWritable: true },
      { pubkey: vaultAuthority(mint), isSigner: false, isWritable: false },
      { pubkey: policyAttestation(mint), isSigner: false, isWritable: false },
      { pubkey: destination, isSigner: false, isWritable: true },
      { pubkey: receipt(mint, nonce), isSigner: false, isWritable: true },
      { pubkey: tokenProgram, isSigner: false, isWritable: false },
      { pubkey: systemProgram, isSigner: false, isWritable: false },
    ],
    Buffer.concat([u64(nonce), u64(1n), u64(gross), u64(tax), u64(net), digest]),
  );
}

function settlementComputeBudgetInstructions(): TransactionInstruction[] {
  return [
    ComputeBudgetProgram.setComputeUnitLimit({ units: 200_000 }),
    ComputeBudgetProgram.setComputeUnitPrice({ microLamports: 1_000 }),
  ];
}

function initializeAdjustmentRegistryInstruction(
  mint: PublicKey,
  tokenProgram: PublicKey,
): TransactionInstruction {
  return instruction(
    "initialize_adjustment_registry",
    [
      { pubkey: payer.publicKey, isSigner: true, isWritable: true },
      { pubkey: merchant.publicKey, isSigner: false, isWritable: false },
      { pubkey: mint, isSigner: false, isWritable: false },
      { pubkey: tokenProgram, isSigner: false, isWritable: false },
      { pubkey: adjustmentRegistry(mint), isSigner: false, isWritable: true },
      { pubkey: systemProgram, isSigner: false, isWritable: false },
    ],
    Buffer.concat([adjustmentProposer.publicKey.toBuffer(), adjustmentApprover.publicKey.toBuffer()]),
  );
}

function recordAdjustmentInstruction(
  mint: PublicKey,
  settlementNonce: bigint,
  adjustmentNonce: bigint,
  tokenProgram: PublicKey,
  options: {
    receiptSettlementNonce?: bigint;
    grossMagnitude?: bigint;
    taxMagnitude?: bigint;
    netMagnitude?: bigint;
  } = {},
): TransactionInstruction {
  const receiptSettlementNonce = options.receiptSettlementNonce ?? settlementNonce;
  const grossMagnitude = options.grossMagnitude ?? 100n;
  const taxMagnitude = options.taxMagnitude ?? 25n;
  const netMagnitude = options.netMagnitude ?? 75n;
  return instruction(
    "record_adjustment",
    [
      { pubkey: adjustmentApprover.publicKey, isSigner: true, isWritable: true },
      { pubkey: adjustmentProposer.publicKey, isSigner: false, isWritable: false },
      { pubkey: merchant.publicKey, isSigner: false, isWritable: false },
      { pubkey: mint, isSigner: false, isWritable: false },
      { pubkey: tokenProgram, isSigner: false, isWritable: false },
      { pubkey: protocolConfig(), isSigner: false, isWritable: false },
      { pubkey: adjustmentRegistry(mint), isSigner: false, isWritable: false },
      { pubkey: receipt(mint, receiptSettlementNonce), isSigner: false, isWritable: false },
      { pubkey: adjustmentEvidence(mint, settlementNonce, adjustmentNonce), isSigner: false, isWritable: true },
      { pubkey: systemProgram, isSigner: false, isWritable: false },
    ],
    Buffer.concat([
      u64(adjustmentNonce),
      Buffer.from([1, 1]), u64(grossMagnitude),
      Buffer.from([1]), u64(taxMagnitude),
      Buffer.from([1]), u64(netMagnitude),
      Buffer.alloc(32, 3),
      Buffer.alloc(32, 4),
      Buffer.alloc(32, 5),
      Buffer.alloc(32, 6),
    ]),
  );
}

function createRefundClaimInstruction(
  mint: PublicKey,
  taxVault: PublicKey,
  netSource: PublicKey,
  refundDestination: PublicKey,
  tokenProgram: PublicKey,
  settlementNonce: bigint,
  adjustmentNonce: bigint,
  claimNonce: bigint,
  executeAfterUnix: bigint,
  expiresAtUnix: bigint,
): TransactionInstruction {
  return instruction(
    "create_adjustment_refund_claim",
    [
      { pubkey: adjustmentProposer.publicKey, isSigner: true, isWritable: true },
      { pubkey: refundFunder.publicKey, isSigner: true, isWritable: false },
      { pubkey: merchant.publicKey, isSigner: false, isWritable: false },
      { pubkey: mint, isSigner: false, isWritable: false },
      { pubkey: tokenProgram, isSigner: false, isWritable: false },
      { pubkey: protocolConfig(), isSigner: false, isWritable: false },
      { pubkey: adjustmentRegistry(mint), isSigner: false, isWritable: false },
      { pubkey: adjustmentEvidence(mint, settlementNonce, adjustmentNonce), isSigner: false, isWritable: false },
      { pubkey: receipt(mint, settlementNonce), isSigner: false, isWritable: false },
      { pubkey: taxVault, isSigner: false, isWritable: false },
      { pubkey: vaultAuthority(mint), isSigner: false, isWritable: false },
      { pubkey: netSource, isSigner: false, isWritable: false },
      { pubkey: refundDestination, isSigner: false, isWritable: false },
      { pubkey: refundClaim(mint, settlementNonce, adjustmentNonce, claimNonce), isSigner: false, isWritable: true },
      { pubkey: systemProgram, isSigner: false, isWritable: false },
    ],
    Buffer.concat([u64(claimNonce), Buffer.alloc(32, 9), i64(executeAfterUnix), i64(expiresAtUnix)]),
  );
}

function executeRefundInstruction(
  mint: PublicKey,
  taxVault: PublicKey,
  netSource: PublicKey,
  refundDestination: PublicKey,
  tokenProgram: PublicKey,
  settlementNonce: bigint,
  adjustmentNonce: bigint,
  claimNonce: bigint,
): TransactionInstruction {
  return instruction(
    "execute_adjustment_refund",
    [
      { pubkey: adjustmentApprover.publicKey, isSigner: true, isWritable: false },
      { pubkey: refundFunder.publicKey, isSigner: true, isWritable: false },
      { pubkey: merchant.publicKey, isSigner: false, isWritable: false },
      { pubkey: mint, isSigner: false, isWritable: false },
      { pubkey: tokenProgram, isSigner: false, isWritable: false },
      { pubkey: protocolConfig(), isSigner: false, isWritable: false },
      { pubkey: adjustmentRegistry(mint), isSigner: false, isWritable: false },
      { pubkey: adjustmentEvidence(mint, settlementNonce, adjustmentNonce), isSigner: false, isWritable: false },
      { pubkey: receipt(mint, settlementNonce), isSigner: false, isWritable: false },
      { pubkey: taxVault, isSigner: false, isWritable: true },
      { pubkey: vaultAuthority(mint), isSigner: false, isWritable: false },
      { pubkey: netSource, isSigner: false, isWritable: true },
      { pubkey: refundDestination, isSigner: false, isWritable: true },
      { pubkey: refundClaim(mint, settlementNonce, adjustmentNonce, claimNonce), isSigner: false, isWritable: true },
    ],
    u64(claimNonce),
  );
}

function initializeSplitPlanInstruction(
  mint: PublicKey,
  tokenProgram: PublicKey,
  recipients: { tokenAccount: PublicKey; basisPoints: number }[],
  version = 1n,
): TransactionInstruction {
  const serializedRecipients = Buffer.concat([
    u32(recipients.length),
    ...recipients.map(({ tokenAccount, basisPoints }) =>
      Buffer.concat([tokenAccount.toBuffer(), u16(basisPoints)]),
    ),
  ]);
  return instruction(
    "initialize_split_plan",
    [
      { pubkey: merchant.publicKey, isSigner: true, isWritable: true },
      { pubkey: mint, isSigner: false, isWritable: false },
      { pubkey: tokenProgram, isSigner: false, isWritable: false },
      { pubkey: splitPlan(mint, version), isSigner: false, isWritable: true },
      { pubkey: systemProgram, isSigner: false, isWritable: false },
    ],
    Buffer.concat([u64(version), serializedRecipients]),
  );
}

function settleSplitInstruction(
  mint: PublicKey,
  source: PublicKey,
  taxVault: PublicKey,
  recipientAccounts: PublicKey[],
  tokenProgram: PublicKey,
  nonce: bigint,
  splitPlanVersion = 1n,
): TransactionInstruction {
  return instruction(
    "settle_split",
    [
      { pubkey: payer.publicKey, isSigner: true, isWritable: true },
      { pubkey: protocolConfig(), isSigner: false, isWritable: false },
      { pubkey: merchant.publicKey, isSigner: false, isWritable: false },
      { pubkey: source, isSigner: false, isWritable: true },
      { pubkey: mint, isSigner: false, isWritable: false },
      { pubkey: taxVault, isSigner: false, isWritable: true },
      { pubkey: vaultAuthority(mint), isSigner: false, isWritable: false },
      { pubkey: policyAttestation(mint), isSigner: false, isWritable: false },
      { pubkey: splitPlan(mint, splitPlanVersion), isSigner: false, isWritable: false },
      { pubkey: receipt(mint, nonce), isSigner: false, isWritable: true },
      { pubkey: tokenProgram, isSigner: false, isWritable: false },
      { pubkey: systemProgram, isSigner: false, isWritable: false },
      ...recipientAccounts.map((pubkey) => ({ pubkey, isSigner: false, isWritable: true })),
    ],
    Buffer.concat([
      u64(nonce),
      u64(1n),
      u64(splitPlanVersion),
      u64(gross),
      u64(tax),
      u64(net),
      digest,
    ]),
  );
}

function setProtocolPausedInstruction(paused: boolean): TransactionInstruction {
  return instruction(
    "set_protocol_paused",
    [
      { pubkey: payer.publicKey, isSigner: true, isWritable: false },
      { pubkey: protocolConfig(), isSigner: false, isWritable: true },
    ],
    Buffer.from([paused ? 1 : 0]),
  );
}

function initializeReleaseRegistryInstruction(
  mint: PublicKey,
  tokenProgram: PublicKey,
): TransactionInstruction {
  return instruction(
    "initialize_vault_release_registry",
    [
      { pubkey: releaseGovernance.publicKey, isSigner: true, isWritable: true },
      { pubkey: merchant.publicKey, isSigner: false, isWritable: false },
      { pubkey: mint, isSigner: false, isWritable: false },
      { pubkey: tokenProgram, isSigner: false, isWritable: false },
      { pubkey: releaseRegistry(mint), isSigner: false, isWritable: true },
      { pubkey: systemProgram, isSigner: false, isWritable: false },
    ],
    Buffer.concat([releaseProposer.publicKey.toBuffer(), releaseApprover.publicKey.toBuffer()]),
  );
}

function createReleaseClaimInstruction(
  mint: PublicKey,
  taxVault: PublicKey,
  destination: PublicKey,
  tokenProgram: PublicKey,
  nonce: bigint,
  amountAtomic: bigint,
  executeAfterUnix: bigint,
  expiresAtUnix: bigint,
): TransactionInstruction {
  return instruction(
    "create_vault_release_claim",
    [
      { pubkey: releaseProposer.publicKey, isSigner: true, isWritable: true },
      { pubkey: merchant.publicKey, isSigner: false, isWritable: false },
      { pubkey: mint, isSigner: false, isWritable: false },
      { pubkey: tokenProgram, isSigner: false, isWritable: false },
      { pubkey: releaseRegistry(mint), isSigner: false, isWritable: false },
      { pubkey: taxVault, isSigner: false, isWritable: true },
      { pubkey: vaultAuthority(mint), isSigner: false, isWritable: false },
      { pubkey: destination, isSigner: false, isWritable: false },
      { pubkey: releaseClaim(mint, nonce), isSigner: false, isWritable: true },
      { pubkey: systemProgram, isSigner: false, isWritable: false },
    ],
    Buffer.concat([
      u64(nonce),
      u64(amountAtomic),
      Buffer.alloc(32, 9),
      i64(executeAfterUnix),
      i64(expiresAtUnix),
    ]),
  );
}

function executeReleaseInstruction(
  mint: PublicKey,
  taxVault: PublicKey,
  destination: PublicKey,
  tokenProgram: PublicKey,
  nonce: bigint,
): TransactionInstruction {
  return instruction(
    "execute_vault_release",
    [
      { pubkey: releaseApprover.publicKey, isSigner: true, isWritable: false },
      { pubkey: merchant.publicKey, isSigner: false, isWritable: false },
      { pubkey: mint, isSigner: false, isWritable: false },
      { pubkey: tokenProgram, isSigner: false, isWritable: false },
      { pubkey: releaseRegistry(mint), isSigner: false, isWritable: false },
      { pubkey: taxVault, isSigner: false, isWritable: true },
      { pubkey: vaultAuthority(mint), isSigner: false, isWritable: false },
      { pubkey: destination, isSigner: false, isWritable: true },
      { pubkey: releaseClaim(mint, nonce), isSigner: false, isWritable: true },
    ],
    u64(nonce),
  );
}

function cancelReleaseInstruction(mint: PublicKey, nonce: bigint): TransactionInstruction {
  return instruction(
    "cancel_vault_release_claim",
    [
      { pubkey: releaseGovernance.publicKey, isSigner: true, isWritable: false },
      { pubkey: merchant.publicKey, isSigner: false, isWritable: false },
      { pubkey: mint, isSigner: false, isWritable: false },
      { pubkey: releaseRegistry(mint), isSigner: false, isWritable: false },
      { pubkey: releaseClaim(mint, nonce), isSigner: false, isWritable: true },
    ],
    u64(nonce),
  );
}

async function send(instructions: TransactionInstruction[], signers: Keypair[]): Promise<string> {
  const latest = await connection.getLatestBlockhash("confirmed");
  const transaction = new Transaction({
    feePayer: payer.publicKey,
    recentBlockhash: latest.blockhash,
  }).add(...instructions);
  const signature = await sendAndConfirmTransaction(connection, transaction, [payer, ...signers], {
    commitment: "confirmed",
    preflightCommitment: "confirmed",
  });
  await connection.confirmTransaction(signature, "finalized");
  return signature;
}

async function sendVersioned(
  instructions: TransactionInstruction[],
  signers: Keypair[],
  lookupTables: AddressLookupTableAccount[],
): Promise<{ signature: string; message: VersionedTransaction["message"] }> {
  const latest = await connection.getLatestBlockhash("confirmed");
  const message = new TransactionMessage({
    payerKey: payer.publicKey,
    recentBlockhash: latest.blockhash,
    instructions,
  }).compileToV0Message(lookupTables);
  if (message.addressTableLookups.length === 0) {
    throw new Error("v0 conformance transaction did not use an address lookup table");
  }
  const transaction = new VersionedTransaction(message);
  transaction.sign([payer, ...signers]);
  const signature = await connection.sendTransaction(transaction, {
    skipPreflight: false,
    preflightCommitment: "confirmed",
    maxRetries: 0,
  });
  await connection.confirmTransaction({
    signature,
    blockhash: latest.blockhash,
    lastValidBlockHeight: latest.lastValidBlockHeight,
  }, "confirmed");
  await connection.confirmTransaction(signature, "finalized");
  return { signature, message: transaction.message };
}

async function airdrop(address: PublicKey, lamports = 5_000_000_000): Promise<void> {
  const signature = await connection.requestAirdrop(address, lamports);
  await connection.confirmTransaction(signature, "confirmed");
}

async function createExtendedMint(): Promise<PublicKey> {
  const mint = Keypair.generate();
  const space = getMintLen([ExtensionType.TransferFeeConfig]);
  const lamports = await connection.getMinimumBalanceForRentExemption(space);
  await send([
    SystemProgram.createAccount({
      fromPubkey: payer.publicKey,
      newAccountPubkey: mint.publicKey,
      lamports,
      space,
      programId: TOKEN_2022_PROGRAM_ID,
    }),
    createInitializeTransferFeeConfigInstruction(
      mint.publicKey,
      payer.publicKey,
      payer.publicKey,
      25,
      10_000n,
      TOKEN_2022_PROGRAM_ID,
    ),
    createInitializeMintInstruction(
      mint.publicKey,
      0,
      payer.publicKey,
      null,
      TOKEN_2022_PROGRAM_ID,
    ),
  ], [mint]);
  return mint.publicKey;
}

describe("validator-backed settlement conformance", () => {
  const test = enabled ? it : it.skip;

  test("settles legacy SPL and rejects a duplicate nonce without balance drift", async () => {
    await airdrop(payer.publicKey, 10_000_000_000);
    await airdrop(merchant.publicKey);
    await airdrop(issuer.publicKey);
    await airdrop(releaseGovernance.publicKey);
    await airdrop(releaseProposer.publicKey);
    await airdrop(releaseApprover.publicKey);
    await airdrop(adjustmentProposer.publicKey);
    await airdrop(adjustmentApprover.publicKey);
    await airdrop(refundFunder.publicKey);
    await send([
      instruction(
        "initialize_protocol_config",
        [
          { pubkey: payer.publicKey, isSigner: true, isWritable: true },
          { pubkey: protocolConfig(), isSigner: false, isWritable: true },
          { pubkey: systemProgram, isSigner: false, isWritable: false },
        ],
        Buffer.alloc(0),
      ),
    ], []);

    const mint = await createMint(connection, payer, payer.publicKey, null, 0, undefined, undefined, TOKEN_PROGRAM_ID);
    const source = await getOrCreateAssociatedTokenAccount(connection, payer, mint, payer.publicKey);
    const vault = await getOrCreateAssociatedTokenAccount(
      connection,
      payer,
      mint,
      vaultAuthority(mint),
      true,
    );
    const destination = await getOrCreateAssociatedTokenAccount(connection, payer, mint, merchant.publicKey);
    await mintTo(connection, payer, mint, source.address, payer, 100n);
    await send([initializeRegistryInstruction(mint, TOKEN_PROGRAM_ID), registerPolicyInstruction(mint, TOKEN_PROGRAM_ID)], [issuer]);

    await send([setProtocolPausedInstruction(true)], []);
    const pausedBalances = await Promise.all([
      getAccount(connection, source.address, "finalized"),
      getAccount(connection, vault.address, "finalized"),
      getAccount(connection, destination.address, "finalized"),
    ]);
    await expect(send([
      ...settlementComputeBudgetInstructions(),
      settleInstruction(mint, source.address, vault.address, destination.address, TOKEN_PROGRAM_ID, 99n),
    ], [])).rejects.toThrow();
    const unchangedWhilePaused = await Promise.all([
      getAccount(connection, source.address, "finalized"),
      getAccount(connection, vault.address, "finalized"),
      getAccount(connection, destination.address, "finalized"),
    ]);
    expect(unchangedWhilePaused.map((account) => account.amount)).toEqual(
      pausedBalances.map((account) => account.amount),
    );
    await send([setProtocolPausedInstruction(false)], []);

    await send([
      ...settlementComputeBudgetInstructions(),
      settleInstruction(mint, source.address, vault.address, destination.address, TOKEN_PROGRAM_ID, 1n),
    ], []);
    const after = await Promise.all([
      getAccount(connection, source.address, "finalized"),
      getAccount(connection, vault.address, "finalized"),
      getAccount(connection, destination.address, "finalized"),
    ]);
    expect(after.map((account) => account.amount)).toEqual([0n, 25n, 75n]);

    await send([initializeAdjustmentRegistryInstruction(mint, TOKEN_PROGRAM_ID)], []);
    const balancesBeforeAdjustmentEvidence = await Promise.all([
      getAccount(connection, source.address, "finalized"),
      getAccount(connection, vault.address, "finalized"),
      getAccount(connection, destination.address, "finalized"),
    ]);
    await send([
      recordAdjustmentInstruction(mint, 1n, 1n, TOKEN_PROGRAM_ID),
    ], [adjustmentApprover]);
    const adjustmentAccount = await connection.getAccountInfo(
      adjustmentEvidence(mint, 1n, 1n),
      "finalized",
    );
    expect(adjustmentAccount?.owner.toBase58()).toBe(programId.toBase58());
    const balancesAfterAdjustmentEvidence = await Promise.all([
      getAccount(connection, source.address, "finalized"),
      getAccount(connection, vault.address, "finalized"),
      getAccount(connection, destination.address, "finalized"),
    ]);
    expect(balancesAfterAdjustmentEvidence.map((account) => account.amount)).toEqual(
      balancesBeforeAdjustmentEvidence.map((account) => account.amount),
    );
    await expect(send([
      recordAdjustmentInstruction(mint, 1n, 1n, TOKEN_PROGRAM_ID),
    ], [adjustmentApprover])).rejects.toThrow();

    await expect(send([
      recordAdjustmentInstruction(mint, 1n, 2n, TOKEN_PROGRAM_ID, {
        receiptSettlementNonce: 2n,
      }),
    ], [adjustmentApprover])).rejects.toThrow();
    expect(await connection.getAccountInfo(
      adjustmentEvidence(mint, 1n, 2n),
      "finalized",
    )).toBeNull();

    await expect(send([
      recordAdjustmentInstruction(mint, 1n, 3n, TOKEN_PROGRAM_ID, {
        netMagnitude: 74n,
      }),
    ], [adjustmentApprover])).rejects.toThrow();
    expect(await connection.getAccountInfo(
      adjustmentEvidence(mint, 1n, 3n),
      "finalized",
    )).toBeNull();

    const refundFunderSource = await getOrCreateAssociatedTokenAccount(
      connection,
      payer,
      mint,
      refundFunder.publicKey,
    );
    await mintTo(connection, payer, mint, refundFunderSource.address, payer, 75n);
    const refundNow = BigInt(Math.floor(Date.now() / 1000));
    await send([
      createRefundClaimInstruction(
        mint,
        vault.address,
        refundFunderSource.address,
        source.address,
        TOKEN_PROGRAM_ID,
        1n,
        1n,
        1n,
        refundNow,
        refundNow + 3_600n,
      ),
    ], [adjustmentProposer, refundFunder]);
    const beforeRefund = await Promise.all([
      getAccount(connection, vault.address, "finalized"),
      getAccount(connection, refundFunderSource.address, "finalized"),
      getAccount(connection, source.address, "finalized"),
    ]);
    expect(beforeRefund.map((account) => account.amount)).toEqual([25n, 75n, 0n]);
    await send([
      executeRefundInstruction(
        mint,
        vault.address,
        refundFunderSource.address,
        source.address,
        TOKEN_PROGRAM_ID,
        1n,
        1n,
        1n,
      ),
    ], [adjustmentApprover, refundFunder]);
    const afterRefund = await Promise.all([
      getAccount(connection, vault.address, "finalized"),
      getAccount(connection, refundFunderSource.address, "finalized"),
      getAccount(connection, source.address, "finalized"),
    ]);
    expect(afterRefund.map((account) => account.amount)).toEqual([0n, 0n, 100n]);
    expect(await connection.getAccountInfo(
      refundClaim(mint, 1n, 1n, 1n),
      "finalized",
    )).not.toBeNull();
    await expect(send([
      executeRefundInstruction(
        mint,
        vault.address,
        refundFunderSource.address,
        source.address,
        TOKEN_PROGRAM_ID,
        1n,
        1n,
        1n,
      ),
    ], [adjustmentApprover, refundFunder])).rejects.toThrow();
    const unchangedAfterRefundRetry = await Promise.all([
      getAccount(connection, vault.address, "finalized"),
      getAccount(connection, refundFunderSource.address, "finalized"),
      getAccount(connection, source.address, "finalized"),
    ]);
    expect(unchangedAfterRefundRetry.map((account) => account.amount)).toEqual([0n, 0n, 100n]);

    await expect(send([
      ...settlementComputeBudgetInstructions(),
      settleInstruction(mint, source.address, vault.address, destination.address, TOKEN_PROGRAM_ID, 1n),
    ], [])).rejects.toThrow();
    const unchanged = await Promise.all([
      getAccount(connection, source.address, "finalized"),
      getAccount(connection, vault.address, "finalized"),
      getAccount(connection, destination.address, "finalized"),
    ]);
    expect(unchanged.map((account) => account.amount)).toEqual([0n, 25n, 75n]);

    const releaseDestination = await getOrCreateAssociatedTokenAccount(
      connection,
      payer,
      mint,
      releaseApprover.publicKey,
    );
    await send([initializeReleaseRegistryInstruction(mint, TOKEN_PROGRAM_ID)], [releaseGovernance]);
    const now = BigInt(Math.floor(Date.now() / 1000));
    await send([
      createReleaseClaimInstruction(
        mint,
        vault.address,
        releaseDestination.address,
        TOKEN_PROGRAM_ID,
        9n,
        25n,
        now,
        now + 3_600n,
      ),
    ], [releaseProposer]);
    await send([
      executeReleaseInstruction(
        mint,
        vault.address,
        releaseDestination.address,
        TOKEN_PROGRAM_ID,
        9n,
      ),
    ], [releaseApprover]);
    const released = await Promise.all([
      getAccount(connection, vault.address, "finalized"),
      getAccount(connection, releaseDestination.address, "finalized"),
    ]);
    expect(released.map((account) => account.amount)).toEqual([0n, 25n]);
    await expect(send([
      executeReleaseInstruction(
        mint,
        vault.address,
        releaseDestination.address,
        TOKEN_PROGRAM_ID,
        9n,
      ),
    ], [releaseApprover])).rejects.toThrow();
    const unchangedAfterDuplicate = await Promise.all([
      getAccount(connection, vault.address, "finalized"),
      getAccount(connection, releaseDestination.address, "finalized"),
    ]);
    expect(unchangedAfterDuplicate.map((account) => account.amount)).toEqual([0n, 25n]);

    await send([
      createReleaseClaimInstruction(
        mint,
        vault.address,
        releaseDestination.address,
        TOKEN_PROGRAM_ID,
        10n,
        1n,
        now,
        now + 3_600n,
      ),
    ], [releaseProposer]);
    await send([cancelReleaseInstruction(mint, 10n)], [releaseGovernance]);
    await expect(send([
      executeReleaseInstruction(
        mint,
        vault.address,
        releaseDestination.address,
        TOKEN_PROGRAM_ID,
        10n,
      ),
    ], [releaseApprover])).rejects.toThrow();
  }, 30_000);

  test("executes settlement through a real v0 address lookup table", async () => {
    await airdrop(payer.publicKey, 10_000_000_000);
    await airdrop(merchant.publicKey);
    await airdrop(issuer.publicKey);
    if ((await connection.getAccountInfo(protocolConfig(), "finalized")) === null) {
      await send([
        instruction(
          "initialize_protocol_config",
          [
            { pubkey: payer.publicKey, isSigner: true, isWritable: true },
            { pubkey: protocolConfig(), isSigner: false, isWritable: true },
            { pubkey: systemProgram, isSigner: false, isWritable: false },
          ],
          Buffer.alloc(0),
        ),
      ], []);
    }

    const mint = await createMint(
      connection,
      payer,
      payer.publicKey,
      null,
      0,
      undefined,
      undefined,
      TOKEN_PROGRAM_ID,
    );
    const source = await getOrCreateAssociatedTokenAccount(connection, payer, mint, payer.publicKey);
    const vault = await getOrCreateAssociatedTokenAccount(
      connection,
      payer,
      mint,
      vaultAuthority(mint),
      true,
    );
    const destination = await getOrCreateAssociatedTokenAccount(connection, payer, mint, merchant.publicKey);
    await mintTo(connection, payer, mint, source.address, payer, 100n);
    await send([
      initializeRegistryInstruction(mint, TOKEN_PROGRAM_ID),
      registerPolicyInstruction(mint, TOKEN_PROGRAM_ID),
    ], [issuer]);

    const nonce = 77n;
    const settlement = settleInstruction(
      mint,
      source.address,
      vault.address,
      destination.address,
      TOKEN_PROGRAM_ID,
      nonce,
    );
    const recentSlot = await connection.getSlot("finalized");
    const [createLookupTable, lookupTableAddress] = AddressLookupTableProgram.createLookupTable({
      authority: payer.publicKey,
      payer: payer.publicKey,
      recentSlot,
    });
    await send([createLookupTable], []);
    const lookupAddresses = settlement.keys
      .map(({ pubkey }) => pubkey)
      .filter((pubkey) => !pubkey.equals(payer.publicKey));
    await send([
      AddressLookupTableProgram.extendLookupTable({
        authority: payer.publicKey,
        payer: payer.publicKey,
        lookupTable: lookupTableAddress,
        addresses: lookupAddresses,
      }),
    ], []);

    const lookupResponse = await connection.getAddressLookupTable(lookupTableAddress, { commitment: "finalized" });
    expect(lookupResponse.value).not.toBeNull();
    const lookupTable = lookupResponse.value;
    if (lookupTable === null) throw new Error("finalized address lookup table was not returned");
    expect(lookupTable.state.addresses).toEqual(expect.arrayContaining(lookupAddresses));

    const { signature, message } = await sendVersioned(
      [...settlementComputeBudgetInstructions(), settlement],
      [],
      [lookupTable],
    );
    expect(message.addressTableLookups).toHaveLength(1);
    const finalized = await connection.getTransaction(signature, {
      commitment: "finalized",
      maxSupportedTransactionVersion: 0,
    });
    expect(finalized?.transaction.message.addressTableLookups).toHaveLength(1);
    const after = await Promise.all([
      getAccount(connection, source.address, "finalized"),
      getAccount(connection, vault.address, "finalized"),
      getAccount(connection, destination.address, "finalized"),
    ]);
    expect(after.map((account) => account.amount)).toEqual([0n, 25n, 75n]);
  }, 45_000);

  test("settles extension-free Token-2022 with the same bounded instruction", async () => {
    const mint = await createMint(
      connection,
      payer,
      payer.publicKey,
      null,
      0,
      undefined,
      undefined,
      TOKEN_2022_PROGRAM_ID,
    );
    const source = await getOrCreateAssociatedTokenAccount(connection, payer, mint, payer.publicKey, false, undefined, undefined, TOKEN_2022_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID);
    const vault = await getOrCreateAssociatedTokenAccount(connection, payer, mint, vaultAuthority(mint), true, undefined, undefined, TOKEN_2022_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID);
    const destination = await getOrCreateAssociatedTokenAccount(connection, payer, mint, merchant.publicKey, false, undefined, undefined, TOKEN_2022_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID);
    await mintTo(connection, payer, mint, source.address, payer, 100n, [], undefined, TOKEN_2022_PROGRAM_ID);
    await send([initializeRegistryInstruction(mint, TOKEN_2022_PROGRAM_ID), registerPolicyInstruction(mint, TOKEN_2022_PROGRAM_ID)], [issuer]);
    await send([
      ...settlementComputeBudgetInstructions(),
      settleInstruction(mint, source.address, vault.address, destination.address, TOKEN_2022_PROGRAM_ID, 2n),
    ], []);
    const after = await Promise.all([
      getAccount(connection, source.address, "finalized", TOKEN_2022_PROGRAM_ID),
      getAccount(connection, vault.address, "finalized", TOKEN_2022_PROGRAM_ID),
      getAccount(connection, destination.address, "finalized", TOKEN_2022_PROGRAM_ID),
    ]);
    expect(after.map((account) => account.amount)).toEqual([0n, 25n, 75n]);
  }, 30_000);

  test("settles an immutable two-recipient Split Plan and rejects unsafe retries", async () => {
    await airdrop(payer.publicKey, 10_000_000_000);
    await airdrop(merchant.publicKey);
    await airdrop(issuer.publicKey);
    if ((await connection.getAccountInfo(protocolConfig(), "finalized")) === null) {
      await send([
        instruction(
          "initialize_protocol_config",
          [
            { pubkey: payer.publicKey, isSigner: true, isWritable: true },
            { pubkey: protocolConfig(), isSigner: false, isWritable: true },
            { pubkey: systemProgram, isSigner: false, isWritable: false },
          ],
          Buffer.alloc(0),
        ),
      ], []);
    }
    const mint = await createMint(
      connection,
      payer,
      payer.publicKey,
      null,
      0,
      undefined,
      undefined,
      TOKEN_PROGRAM_ID,
    );
    const source = await getOrCreateAssociatedTokenAccount(connection, payer, mint, payer.publicKey);
    const vault = await getOrCreateAssociatedTokenAccount(
      connection,
      payer,
      mint,
      vaultAuthority(mint),
      true,
    );
    const recipientA = await getOrCreateAssociatedTokenAccount(
      connection,
      payer,
      mint,
      splitRecipientOwnerA.publicKey,
    );
    const recipientB = await getOrCreateAssociatedTokenAccount(
      connection,
      payer,
      mint,
      splitRecipientOwnerB.publicKey,
    );
    await mintTo(connection, payer, mint, source.address, payer, 100n);

    await send([
      initializeRegistryInstruction(mint, TOKEN_PROGRAM_ID),
      registerPolicyInstruction(mint, TOKEN_PROGRAM_ID),
      initializeSplitPlanInstruction(mint, TOKEN_PROGRAM_ID, [
        { tokenAccount: recipientA.address, basisPoints: 6_000 },
        { tokenAccount: recipientB.address, basisPoints: 4_000 },
      ]),
    ], [issuer, merchant]);

    await send([
      settleSplitInstruction(
        mint,
        source.address,
        vault.address,
        [recipientA.address, recipientB.address],
        TOKEN_PROGRAM_ID,
        20n,
      ),
    ], []);
    const settled = await Promise.all([
      getAccount(connection, source.address, "finalized"),
      getAccount(connection, vault.address, "finalized"),
      getAccount(connection, recipientA.address, "finalized"),
      getAccount(connection, recipientB.address, "finalized"),
    ]);
    expect(settled.map((account) => account.amount)).toEqual([0n, 25n, 45n, 30n]);

    await expect(send([
      settleSplitInstruction(
        mint,
        source.address,
        vault.address,
        [recipientA.address, recipientB.address],
        TOKEN_PROGRAM_ID,
        20n,
      ),
    ], [])).rejects.toThrow();
    const afterDuplicate = await Promise.all([
      getAccount(connection, source.address, "finalized"),
      getAccount(connection, vault.address, "finalized"),
      getAccount(connection, recipientA.address, "finalized"),
      getAccount(connection, recipientB.address, "finalized"),
    ]);
    expect(afterDuplicate.map((account) => account.amount)).toEqual([0n, 25n, 45n, 30n]);

    await expect(send([
      settleSplitInstruction(
        mint,
        source.address,
        vault.address,
        [recipientB.address, recipientA.address],
        TOKEN_PROGRAM_ID,
        21n,
      ),
    ], [])).rejects.toThrow();
    const afterWrongOrder = await Promise.all([
      getAccount(connection, source.address, "finalized"),
      getAccount(connection, vault.address, "finalized"),
      getAccount(connection, recipientA.address, "finalized"),
      getAccount(connection, recipientB.address, "finalized"),
    ]);
    expect(afterWrongOrder.map((account) => account.amount)).toEqual([0n, 25n, 45n, 30n]);
  }, 30_000);

  test("rejects a real Token-2022 TransferFeeConfig mint before registry activation", async () => {
    const mint = await createExtendedMint();
    await expect(send([initializeRegistryInstruction(mint, TOKEN_2022_PROGRAM_ID)], [])).rejects.toThrow();
    expect(await connection.getAccountInfo(issuerRegistry(mint), "finalized")).toBeNull();
  }, 30_000);

  test("revoked policy attestations cannot move tokens", async () => {
    await airdrop(payer.publicKey, 10_000_000_000);
    await airdrop(merchant.publicKey);
    await airdrop(issuer.publicKey);
    if ((await connection.getAccountInfo(protocolConfig(), "finalized")) === null) {
      await send([
        instruction(
          "initialize_protocol_config",
          [
            { pubkey: payer.publicKey, isSigner: true, isWritable: true },
            { pubkey: protocolConfig(), isSigner: false, isWritable: true },
            { pubkey: systemProgram, isSigner: false, isWritable: false },
          ],
          Buffer.alloc(0),
        ),
      ], []);
    }
    const mint = await createMint(
      connection,
      payer,
      payer.publicKey,
      null,
      0,
      undefined,
      undefined,
      TOKEN_PROGRAM_ID,
    );
    const source = await getOrCreateAssociatedTokenAccount(connection, payer, mint, payer.publicKey);
    const vault = await getOrCreateAssociatedTokenAccount(
      connection,
      payer,
      mint,
      vaultAuthority(mint),
      true,
    );
    const destination = await getOrCreateAssociatedTokenAccount(
      connection,
      payer,
      mint,
      merchant.publicKey,
    );
    await mintTo(connection, payer, mint, source.address, payer, 100n);
    await send([
      initializeRegistryInstruction(mint, TOKEN_PROGRAM_ID),
      registerPolicyInstruction(mint, TOKEN_PROGRAM_ID),
    ], [issuer]);
    await send([revokePolicyInstruction(mint)], []);

    const before = await Promise.all([
      getAccount(connection, source.address, "finalized"),
      getAccount(connection, vault.address, "finalized"),
      getAccount(connection, destination.address, "finalized"),
    ]);
    await expect(send([
      ...settlementComputeBudgetInstructions(),
      settleInstruction(
        mint,
        source.address,
        vault.address,
        destination.address,
        TOKEN_PROGRAM_ID,
        30n,
      ),
    ], [])).rejects.toThrow();
    const after = await Promise.all([
      getAccount(connection, source.address, "finalized"),
      getAccount(connection, vault.address, "finalized"),
      getAccount(connection, destination.address, "finalized"),
    ]);
    expect(after.map((account) => account.amount)).toEqual(
      before.map((account) => account.amount),
    );
  }, 30_000);
});
