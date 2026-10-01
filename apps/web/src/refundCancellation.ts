import { Connection, PublicKey, Transaction, TransactionInstruction } from "@solana/web3.js";
import { Buffer } from "buffer";
import { ensureConnectedWallet, signWalletTransaction } from "./wallet";
import { parseCanonicalU64 } from "./settlementAdjustments";

const SETTLEMENT_PROGRAM_ID = "4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi";
const CANCEL_DISCRIMINATOR_BASE64 = "prTgptg+e5k=";
const UUID_V7 = /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

export type RefundCancellationStatus =
  | "prepared_unsubmitted" | "signed" | "submitting" | "submitted"
  | "processed" | "confirmed" | "finalized" | "rejected" | "unknown";

export type CancellationAccount = { pubkey: string; is_signer: boolean; is_writable: boolean };
export type CancellationInstruction = {
  status: "prepared_unsubmitted";
  program_id: string;
  instruction_data_base64: string;
  accounts: CancellationAccount[];
  registry: string;
  claim: string;
  claim_nonce: string;
};

export type RefundCancellationArtifact = {
  schema: string;
  instruction: CancellationInstruction;
  instruction_digest: number[];
  artifact_digest: number[];
  merchant_id: string;
  claim_intent_id: string;
  adjustment_id: string;
  merchant: string;
  mint: string;
  adjustment: string;
  governance_authority: string;
  governance_subject: string;
};

export type RefundCancellationResponse = {
  status: RefundCancellationStatus;
  intent_id: string;
  request_digest: number[];
  instruction_digest: number[];
  artifact_digest: number[];
  claim: string;
  artifact: RefundCancellationArtifact;
  signed_transaction_digest: number[] | null;
  signature: string | null;
  submitted_slot: string | null;
  commitment: string | null;
  last_error_code: string | null;
  finalized_evidence: RefundCancellationFinalizedEvidence | null;
};

export type RefundCancellationFinalizedEvidence = {
  event_type: "AdjustmentRefundClaimCancelled";
  event_version: "v1";
  event_discriminator: number[];
  commitment: "finalized";
  signature: string;
  slot: string;
  event_index: number;
  claim: number[];
  adjustment: number[];
  merchant: number[];
  mint: number[];
  nonce: string;
  governance_authority: number[];
  reference_digest: number[];
  evidence_digest: number[];
};

export type RefundCancellationSubmissionResponse = {
  intent_id: string;
  status: RefundCancellationStatus;
  signature: string | null;
  provider_index: number | null;
};

export function parseRefundCancellationSubmissionResponse(value: unknown): RefundCancellationSubmissionResponse {
  const response = record(value, "The API returned an invalid refund cancellation submission response.");
  const statuses: RefundCancellationStatus[] = ["prepared_unsubmitted", "signed", "submitting", "submitted", "processed", "confirmed", "finalized", "rejected", "unknown"];
  if (typeof response.status !== "string" || !statuses.includes(response.status as RefundCancellationStatus)) throw new Error("The API returned an invalid cancellation submission status.");
  if (typeof response.intent_id !== "string" || !UUID_V7.test(response.intent_id)) throw new Error("The API returned an invalid cancellation submission intent ID.");
  const signature = response.signature == null ? null : text(response.signature, "cancellation submission signature", 128);
  if (response.provider_index != null && (typeof response.provider_index !== "number" || !Number.isInteger(response.provider_index) || response.provider_index < 0 || response.provider_index > 2)) {
    throw new Error("The API returned an invalid cancellation provider index.");
  }
  return {
    intent_id: response.intent_id.toLowerCase(),
    status: response.status as RefundCancellationStatus,
    signature,
    provider_index: response.provider_index == null ? null : response.provider_index,
  };
}

function record(value: unknown, message: string): Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error(message);
  return value as Record<string, unknown>;
}

function key(value: unknown, field: string): string {
  if (typeof value !== "string") throw new Error(`The API returned an invalid ${field}.`);
  try { return new PublicKey(value).toBase58(); } catch { throw new Error(`The API returned an invalid ${field}.`); }
}

function uuid(value: unknown, field: string): string {
  if (typeof value !== "string" || !UUID_V7.test(value)) throw new Error(`The API returned an invalid UUIDv7 ${field}.`);
  return value.toLowerCase();
}

function digest(value: unknown, field: string): number[] {
  if (!Array.isArray(value) || value.length !== 32 || value.some((byte) => typeof byte !== "number" || !Number.isInteger(byte) || byte < 0 || byte > 255)) {
    throw new Error(`The API returned an invalid ${field} digest.`);
  }
  return [...value] as number[];
}

function text(value: unknown, field: string, max: number): string {
  if (typeof value !== "string" || value.trim() === "" || new TextEncoder().encode(value).length > max) throw new Error(`The API returned an invalid ${field}.`);
  return value;
}

function base64(value: unknown, field: string): Uint8Array {
  if (typeof value !== "string" || value.length === 0) throw new Error(`The API returned empty ${field}.`);
  const bytes = Buffer.from(value, "base64");
  if (bytes.length === 0 || bytes.toString("base64") !== value) throw new Error(`The API returned non-canonical ${field}.`);
  return Uint8Array.from(bytes);
}

function finalizedEvidence(value: unknown): RefundCancellationFinalizedEvidence | null {
  if (value == null) return null;
  const candidate = record(value, "The API returned invalid finalized cancellation evidence.");
  if (candidate.event_type !== "AdjustmentRefundClaimCancelled" || candidate.event_version !== "v1" || candidate.commitment !== "finalized") {
    throw new Error("The API returned unsupported finalized cancellation evidence.");
  }
  const fixedBytes = (field: string, length: number): number[] => {
    const value = candidate[field];
    if (!Array.isArray(value) || value.length !== length || value.some((byte) => typeof byte !== "number" || !Number.isInteger(byte) || byte < 0 || byte > 255)) {
      throw new Error(`The API returned an invalid ${field} byte array.`);
    }
    return [...value] as number[];
  };
  if (typeof candidate.signature !== "string" || typeof candidate.slot !== "string" || !/^\d+$/.test(candidate.slot) || typeof candidate.event_index !== "number" || !Number.isInteger(candidate.event_index) || candidate.event_index < 0 || candidate.event_index > 65535) {
    throw new Error("The API returned invalid finalized cancellation evidence identity.");
  }
  return {
    event_type: "AdjustmentRefundClaimCancelled",
    event_version: "v1",
    event_discriminator: fixedBytes("event_discriminator", 8),
    commitment: "finalized",
    signature: text(candidate.signature, "finalized cancellation signature", 128),
    slot: parseCanonicalU64(candidate.slot, "finalized cancellation slot"),
    event_index: candidate.event_index,
    claim: fixedBytes("claim", 32),
    adjustment: fixedBytes("adjustment", 32),
    merchant: fixedBytes("merchant", 32),
    mint: fixedBytes("mint", 32),
    nonce: parseCanonicalU64(candidate.nonce, "finalized cancellation nonce"),
    governance_authority: fixedBytes("governance_authority", 32),
    reference_digest: fixedBytes("reference_digest", 32),
    evidence_digest: fixedBytes("evidence_digest", 32),
  };
}

export function parseRefundCancellationResponse(value: unknown): RefundCancellationResponse {
  const response = record(value, "The API returned an invalid refund cancellation response.");
  const statuses: RefundCancellationStatus[] = ["prepared_unsubmitted", "signed", "submitting", "submitted", "processed", "confirmed", "finalized", "rejected", "unknown"];
  if (typeof response.status !== "string" || !statuses.includes(response.status as RefundCancellationStatus)) throw new Error("The API returned an invalid cancellation status.");
  const artifactRecord = record(response.artifact, "The API returned an invalid refund cancellation artifact.");
  const instructionRecord = record(artifactRecord.instruction, "The API returned an invalid cancellation instruction.");
  if (instructionRecord.status !== "prepared_unsubmitted") throw new Error("The cancellation instruction is not prepared for wallet review.");
  const accountsValue = instructionRecord.accounts;
  if (!Array.isArray(accountsValue) || accountsValue.length !== 5) throw new Error("The API returned an invalid cancellation account list.");
  const accounts = accountsValue.map((candidate, index) => {
    const account = record(candidate, `The API returned an invalid cancellation account at ${index}.`);
    if (typeof account.is_signer !== "boolean" || typeof account.is_writable !== "boolean") throw new Error("The API returned invalid cancellation account flags.");
    return { pubkey: key(account.pubkey, `cancellation account ${index}`), is_signer: account.is_signer, is_writable: account.is_writable };
  });
  if (new Set(accounts.map((account) => account.pubkey)).size !== 5) throw new Error("The API returned duplicate cancellation accounts.");
  const instruction: CancellationInstruction = {
    status: "prepared_unsubmitted",
    program_id: key(instructionRecord.program_id, "cancellation program"),
    instruction_data_base64: typeof instructionRecord.instruction_data_base64 === "string" ? instructionRecord.instruction_data_base64 : "",
    accounts,
    registry: key(instructionRecord.registry, "cancellation registry"),
    claim: key(instructionRecord.claim, "cancellation claim"),
    claim_nonce: parseCanonicalU64(instructionRecord.claim_nonce, "cancellation claim nonce"),
  };
  if (instruction.program_id !== SETTLEMENT_PROGRAM_ID || instruction.claim_nonce === "0") throw new Error("The API returned an unsupported cancellation instruction.");
  const data = base64(instruction.instruction_data_base64, "cancellation instruction data");
  if (data.length !== 16 || Buffer.from(data.subarray(0, 8)).toString("base64") !== CANCEL_DISCRIMINATOR_BASE64) throw new Error("The API returned non-canonical cancellation instruction data.");
  const expectedFlags: Array<[boolean, boolean]> = [[true, true], [false, false], [false, false], [false, false], [false, true]];
  accounts.forEach((account, index) => {
    const [signer, writable] = expectedFlags[index];
    if (account.is_signer !== signer || account.is_writable !== writable) throw new Error(`The cancellation account flags differ at position ${index}.`);
  });
  if (instruction.claim !== accounts[4].pubkey || instruction.registry !== accounts[3].pubkey) throw new Error("The cancellation account topology is not bound to the compiled artifact.");
  const artifact: RefundCancellationArtifact = {
    schema: text(artifactRecord.schema, "cancellation artifact schema", 128),
    instruction,
    instruction_digest: digest(artifactRecord.instruction_digest, "cancellation instruction"),
    artifact_digest: digest(artifactRecord.artifact_digest, "cancellation artifact"),
    merchant_id: uuid(artifactRecord.merchant_id, "merchant ID"),
    claim_intent_id: uuid(artifactRecord.claim_intent_id, "claim intent ID"),
    adjustment_id: uuid(artifactRecord.adjustment_id, "adjustment ID"),
    merchant: key(artifactRecord.merchant, "cancellation merchant"),
    mint: key(artifactRecord.mint, "cancellation mint"),
    adjustment: key(artifactRecord.adjustment, "cancellation adjustment"),
    governance_authority: key(artifactRecord.governance_authority, "governance authority"),
    governance_subject: text(artifactRecord.governance_subject, "governance subject", 256),
  };
  if (artifact.schema !== "refund-claim-cancellation-artifact/v1") throw new Error("The API returned an unsupported cancellation artifact schema.");
  const [derivedRegistry] = PublicKey.findProgramAddressSync(
    [Buffer.from("adjustment-registry"), new PublicKey(artifact.merchant).toBytes(), new PublicKey(artifact.mint).toBytes()],
    new PublicKey(SETTLEMENT_PROGRAM_ID),
  );
  const nonceBytes = Buffer.alloc(8);
  nonceBytes.writeBigUInt64LE(BigInt(artifact.instruction.claim_nonce));
  const [derivedClaim] = PublicKey.findProgramAddressSync(
    [Buffer.from("refund-claim"), new PublicKey(artifact.adjustment).toBytes(), nonceBytes],
    new PublicKey(SETTLEMENT_PROGRAM_ID),
  );
  if (artifact.instruction.registry !== derivedRegistry.toBase58()
      || artifact.instruction.claim !== derivedClaim.toBase58()
      || artifact.instruction.accounts[0].pubkey !== artifact.governance_authority
      || artifact.instruction.accounts[1].pubkey !== artifact.merchant
      || artifact.instruction.accounts[2].pubkey !== artifact.mint
      || artifact.instruction.accounts[3].pubkey !== artifact.instruction.registry
      || artifact.instruction.accounts[4].pubkey !== artifact.instruction.claim) {
    throw new Error("The API returned cancellation identities that do not match their deterministic PDAs.");
  }
  const responseClaim = key(response.claim, "cancellation claim");
  const requestDigest = digest(response.request_digest, "cancellation request");
  const instructionDigest = digest(response.instruction_digest, "cancellation instruction");
  const artifactDigest = digest(response.artifact_digest, "cancellation artifact");
  if (responseClaim !== artifact.instruction.claim
      || instructionDigest.some((byte, index) => byte !== artifact.instruction_digest[index])
      || artifactDigest.some((byte, index) => byte !== artifact.artifact_digest[index])) {
    throw new Error("The API returned cancellation digests or claim identity inconsistent with its artifact.");
  }
  return {
    status: response.status as RefundCancellationStatus,
    intent_id: uuid(response.intent_id, "cancellation intent ID"),
    request_digest: requestDigest,
    instruction_digest: instructionDigest,
    artifact_digest: artifactDigest,
    claim: responseClaim,
    artifact,
    signed_transaction_digest: response.signed_transaction_digest == null ? null : digest(response.signed_transaction_digest, "signed transaction"),
    signature: response.signature == null ? null : text(response.signature, "cancellation signature", 128),
    submitted_slot: response.submitted_slot == null ? null : parseCanonicalU64(response.submitted_slot, "submitted slot"),
    commitment: response.commitment == null ? null : text(response.commitment, "commitment", 32),
    last_error_code: response.last_error_code == null ? null : text(response.last_error_code, "last error code", 128),
    finalized_evidence: finalizedEvidence(response.finalized_evidence),
  };
}

function encode(transaction: Transaction): string {
  const bytes = transaction.serialize({ requireAllSignatures: false, verifySignatures: false });
  return btoa(String.fromCharCode(...bytes));
}

function validateCancellationTransaction(transaction: Transaction, instruction: CancellationInstruction, expectedAuthority: PublicKey): void {
  if (!transaction.feePayer?.equals(expectedAuthority) || transaction.instructions.length !== 1) {
    throw new Error("The wallet transaction does not preserve the server-bound cancellation fee payer or instruction count.");
  }
  const candidate = transaction.instructions[0];
  const expectedData = base64(instruction.instruction_data_base64, "cancellation instruction data");
  if (!candidate.programId.equals(new PublicKey(instruction.program_id)) || Buffer.from(candidate.data).compare(Buffer.from(expectedData)) !== 0 || candidate.keys.length !== instruction.accounts.length) {
    throw new Error("The wallet transaction does not preserve the server-compiled cancellation instruction.");
  }
  candidate.keys.forEach((account, index) => {
    const expected = instruction.accounts[index];
    if (account.pubkey.toBase58() !== expected.pubkey || account.isSigner !== expected.is_signer || account.isWritable !== expected.is_writable) {
      throw new Error(`The wallet transaction changed the cancellation account at position ${index}.`);
    }
  });
}

export async function signRefundCancellation(instruction: CancellationInstruction, expectedAuthority: PublicKey, rpcUrl: string, serialized: string | null): Promise<string> {
  const wallet = await ensureConnectedWallet();
  if (!rpcUrl.trim()) throw new Error("A VITE_SOLANA_RPC_URL is required before cancellation signing is enabled.");
  if (!wallet.publicKey.equals(expectedAuthority)) throw new Error("Connected wallet must match the server-bound governance authority.");
  const transaction = serialized
    ? Transaction.from(Buffer.from(serialized, "base64"))
    : await new Connection(rpcUrl, "confirmed").getLatestBlockhash("confirmed").then(({ blockhash }) => new Transaction({ feePayer: expectedAuthority, recentBlockhash: blockhash }).add(new TransactionInstruction({
      programId: new PublicKey(instruction.program_id),
      data: Buffer.from(base64(instruction.instruction_data_base64, "cancellation instruction data")),
      keys: instruction.accounts.map((account) => ({ pubkey: new PublicKey(account.pubkey), isSigner: account.is_signer, isWritable: account.is_writable })),
    })));
  validateCancellationTransaction(transaction, instruction, expectedAuthority);
  return encode(await signWalletTransaction(wallet, transaction, rpcUrl));
}

export function refundCancellationHasSignature(serialized: string): boolean {
  const transaction = Transaction.from(Buffer.from(serialized, "base64"));
  return transaction.signatures.length > 0 && transaction.signatures[0].signature !== null;
}
