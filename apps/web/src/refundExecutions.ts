import { Connection, PublicKey, Transaction, TransactionInstruction } from "@solana/web3.js";
import { Buffer } from "buffer";
import { ensureConnectedWallet, signWalletTransaction } from "./wallet";
import { parseCanonicalU64 } from "./settlementAdjustments";
import {
  REFUND_STATUSES,
  SETTLEMENT_PROGRAM_ID,
  SPL_TOKEN_PROGRAM_ID,
} from "./refundClaims";

export const REFUND_EXECUTE_ACCOUNT_COUNT = 14;
const REFUND_EXECUTE_DISCRIMINATOR_BASE64 = "nChphaQNsfM=";
const UUID_V7 = /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

export type RefundExecuteAccount = {
  pubkey: string;
  is_signer: boolean;
  is_writable: boolean;
};

export type RefundExecuteInstruction = {
  status: "prepared_unsubmitted";
  program_id: string;
  instruction_data_base64: string;
  accounts: RefundExecuteAccount[];
  registry: string;
  adjustment: string;
  original_receipt: string;
  claim: string;
  claim_nonce: string;
};

export type RefundExecuteArtifact = {
  schema: string;
  instruction: RefundExecuteInstruction;
  instruction_digest: number[];
  artifact_digest: number[];
  refund_destination: string;
  merchant_id: string;
  adjustment_id: string;
  funding_configuration_id: string;
  receipt_signature: string;
  receipt_slot: string | null;
  claim_signature: string;
  claim_slot: string;
  authority_subject: string;
  proposer_subject: string;
  refund_funder_subject: string;
  proposer: string;
  refund_funder: string;
  merchant: string;
  mint: string;
  token_program: string;
  tax_vault: string;
  net_source: string;
  gross_atomic: string | null;
  tax_atomic: string | null;
  net_atomic: string | null;
  execute_after_unix: number | null;
  expires_at_unix: number | null;
};

export type RefundExecuteResponse = {
  status: (typeof REFUND_STATUSES)[number];
  intent_id: string;
  request_digest: number[];
  instruction_digest: number[];
  artifact_digest: number[];
  claim: string;
  artifact: RefundExecuteArtifact;
  signed_transaction_digest: number[] | null;
  signature: string | null;
  submitted_slot: string | null;
  commitment: string | null;
  last_error_code: string | null;
};

export type RefundExecuteSubmissionResponse = {
  intent_id: string;
  status: (typeof REFUND_STATUSES)[number];
  signature: string | null;
  provider_index: number | null;
};

function objectValue(value: unknown, message: string): Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error(message);
  return value as Record<string, unknown>;
}

function publicKey(value: unknown, field: string): string {
  if (typeof value !== "string") throw new Error(`The API returned an invalid ${field}.`);
  try {
    return new PublicKey(value).toBase58();
  } catch {
    throw new Error(`The API returned an invalid ${field}.`);
  }
}

function uuidV7(value: unknown, field: string): string {
  if (typeof value !== "string" || !UUID_V7.test(value)) throw new Error(`The API returned an invalid UUIDv7 ${field}.`);
  return value.toLowerCase();
}

function digest(value: unknown, field: string): number[] {
  if (!Array.isArray(value) || value.length !== 32 || value.some((byte) => typeof byte !== "number" || !Number.isInteger(byte) || byte < 0 || byte > 255)) {
    throw new Error(`The API returned an invalid ${field} digest.`);
  }
  return [...value] as number[];
}

function boundedText(value: unknown, field: string, maxBytes: number): string {
  if (typeof value !== "string" || value.trim() === "" || new TextEncoder().encode(value).length > maxBytes) throw new Error(`The API returned an invalid ${field}.`);
  return value;
}

function optionalText(value: unknown, field: string, maxBytes: number): string | null {
  return value === null || value === undefined ? null : boundedText(value, field, maxBytes);
}

function optionalAtomic(value: unknown, field: string): string | null {
  return value === null || value === undefined ? null : parseCanonicalU64(value, field);
}

function requiredAtomic(value: unknown, field: string): string {
  const result = parseCanonicalU64(value, field);
  if (result === "0") throw new Error(`The API returned an invalid ${field}.`);
  return result;
}

function optionalUnix(value: unknown, field: string): number | null {
  if (value === null || value === undefined) return null;
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value <= 0) throw new Error(`The API returned an invalid ${field}.`);
  return value;
}

function base64Bytes(value: unknown, field: string): Uint8Array {
  if (typeof value !== "string" || value.length === 0) throw new Error(`The API returned empty ${field}.`);
  const bytes = Buffer.from(value, "base64");
  if (bytes.length === 0 || bytes.toString("base64") !== value) throw new Error(`The API returned non-canonical ${field} base64.`);
  return Uint8Array.from(bytes);
}

function parseAccounts(value: unknown): RefundExecuteAccount[] {
  if (!Array.isArray(value) || value.length !== REFUND_EXECUTE_ACCOUNT_COUNT) throw new Error("The API returned an invalid refund execute account list.");
  const keys = new Set<string>();
  return value.map((candidate, index) => {
    const account = objectValue(candidate, `The API returned an invalid refund execute account at position ${index}.`);
    if (typeof account.is_signer !== "boolean" || typeof account.is_writable !== "boolean") throw new Error(`The API returned invalid refund execute account flags at position ${index}.`);
    const parsed = { pubkey: publicKey(account.pubkey, `refund execute account ${index}`), is_signer: account.is_signer, is_writable: account.is_writable };
    if (keys.has(parsed.pubkey)) throw new Error("The API returned duplicate refund execute accounts.");
    keys.add(parsed.pubkey);
    return parsed;
  });
}

export function parseRefundExecuteInstruction(value: unknown): RefundExecuteInstruction {
  const record = objectValue(value, "The API returned an invalid refund execute instruction.");
  if (record.status !== "prepared_unsubmitted") throw new Error("The refund execute instruction is not prepared for wallet review.");
  const instruction: RefundExecuteInstruction = {
    status: "prepared_unsubmitted",
    program_id: publicKey(record.program_id, "refund execute program"),
    instruction_data_base64: typeof record.instruction_data_base64 === "string" ? record.instruction_data_base64 : "",
    accounts: parseAccounts(record.accounts),
    registry: publicKey(record.registry, "refund adjustment registry"),
    adjustment: publicKey(record.adjustment, "refund adjustment PDA"),
    original_receipt: publicKey(record.original_receipt, "original receipt PDA"),
    claim: publicKey(record.claim, "refund claim PDA"),
    claim_nonce: requiredAtomic(record.claim_nonce, "refund claim nonce"),
  };
  if (instruction.program_id !== SETTLEMENT_PROGRAM_ID) throw new Error("The API returned an unsupported refund execute program.");
  const data = base64Bytes(instruction.instruction_data_base64, "refund execute instruction data");
  if (data.length !== 16 || Buffer.from(data.subarray(0, 8)).toString("base64") !== REFUND_EXECUTE_DISCRIMINATOR_BASE64) throw new Error("The API returned non-canonical refund execute instruction data.");
  const expectedFlags: Array<[boolean, boolean]> = [
    [true, false], [true, false], [false, false], [false, false], [false, false],
    [false, false], [false, false], [false, false], [false, false], [false, true],
    [false, false], [false, true], [false, true], [false, true],
  ];
  instruction.accounts.forEach((account, index) => {
    const [signer, writable] = expectedFlags[index];
    if (account.is_signer !== signer || account.is_writable !== writable) throw new Error(`The refund execute account flags differ at position ${index}.`);
  });
  if (instruction.accounts[4].pubkey !== SPL_TOKEN_PROGRAM_ID || instruction.accounts[6].pubkey !== instruction.registry || instruction.accounts[7].pubkey !== instruction.adjustment || instruction.accounts[8].pubkey !== instruction.original_receipt || instruction.accounts[13].pubkey !== instruction.claim) {
    throw new Error("The refund execute account topology is not bound to the compiled artifact.");
  }
  return instruction;
}

export function parseRefundExecuteResponse(value: unknown): RefundExecuteResponse {
  const record = objectValue(value, "The API returned an invalid refund execute response.");
  if (typeof record.status !== "string" || !REFUND_STATUSES.includes(record.status as RefundExecuteResponse["status"])) throw new Error("The API returned an invalid refund execute status.");
  const artifactRecord = objectValue(record.artifact, "The API returned an invalid refund execute artifact.");
  const artifact: RefundExecuteArtifact = {
    schema: boundedText(artifactRecord.schema, "refund execute artifact schema", 128),
    instruction: parseRefundExecuteInstruction(artifactRecord.instruction),
    instruction_digest: digest(artifactRecord.instruction_digest, "refund execute instruction"),
    artifact_digest: digest(artifactRecord.artifact_digest, "refund execute artifact"),
    refund_destination: publicKey(artifactRecord.refund_destination, "refund destination"),
    merchant_id: uuidV7(artifactRecord.merchant_id, "merchant ID"),
    adjustment_id: uuidV7(artifactRecord.adjustment_id, "adjustment ID"),
    funding_configuration_id: uuidV7(artifactRecord.funding_configuration_id, "funding configuration ID"),
    receipt_signature: boundedText(artifactRecord.receipt_signature, "receipt signature", 128),
    receipt_slot: optionalAtomic(artifactRecord.receipt_slot, "receipt slot"),
    claim_signature: boundedText(artifactRecord.claim_signature, "claim signature", 128),
    claim_slot: requiredAtomic(artifactRecord.claim_slot, "claim slot"),
    authority_subject: boundedText(artifactRecord.authority_subject, "approver subject", 256),
    proposer_subject: artifactRecord.proposer_subject === "" ? "" : boundedText(artifactRecord.proposer_subject, "proposer subject", 256),
    refund_funder_subject: boundedText(artifactRecord.refund_funder_subject, "refund funder subject", 256),
    proposer: artifactRecord.proposer === "" ? "" : publicKey(artifactRecord.proposer, "refund proposer"),
    refund_funder: publicKey(artifactRecord.refund_funder, "refund funder"),
    merchant: publicKey(artifactRecord.merchant, "refund merchant"),
    mint: publicKey(artifactRecord.mint, "refund mint"),
    token_program: publicKey(artifactRecord.token_program, "refund token program"),
    tax_vault: publicKey(artifactRecord.tax_vault, "tax vault"),
    net_source: publicKey(artifactRecord.net_source, "net source"),
    gross_atomic: optionalAtomic(artifactRecord.gross_atomic, "gross amount"),
    tax_atomic: optionalAtomic(artifactRecord.tax_atomic, "tax amount"),
    net_atomic: optionalAtomic(artifactRecord.net_atomic, "net amount"),
    execute_after_unix: optionalUnix(artifactRecord.execute_after_unix, "execute-after time"),
    expires_at_unix: optionalUnix(artifactRecord.expires_at_unix, "expiry time"),
  };
  if (artifact.schema !== "refund-movement-artifact/v1"
      || artifact.proposer !== ""
      || artifact.proposer_subject !== ""
      || artifact.authority_subject !== artifact.instruction.accounts[0].pubkey
      || artifact.refund_funder !== artifact.instruction.accounts[1].pubkey) {
    throw new Error("The API returned an unsupported or topology-mismatched refund execute artifact.");
  }
  const claim = publicKey(record.claim, "refund claim");
  if (claim !== artifact.instruction.claim) throw new Error("The API returned a refund claim identity mismatch.");
  return {
    status: record.status as RefundExecuteResponse["status"],
    intent_id: uuidV7(record.intent_id, "refund execute intent ID"),
    request_digest: digest(record.request_digest, "refund execute request"),
    instruction_digest: digest(record.instruction_digest, "refund execute instruction"),
    artifact_digest: digest(record.artifact_digest, "refund execute artifact"),
    claim,
    artifact,
    signed_transaction_digest: record.signed_transaction_digest === null || record.signed_transaction_digest === undefined ? null : digest(record.signed_transaction_digest, "signed transaction"),
    signature: optionalText(record.signature, "refund execute signature", 128),
    submitted_slot: optionalAtomic(record.submitted_slot, "submitted slot"),
    commitment: optionalText(record.commitment, "commitment", 32),
    last_error_code: optionalText(record.last_error_code, "last error code", 128),
  };
}

export function parseRefundExecuteSubmissionResponse(value: unknown): RefundExecuteSubmissionResponse {
  const record = objectValue(value, "The API returned an invalid refund execute submission response.");
  if (typeof record.status !== "string" || !REFUND_STATUSES.includes(record.status as RefundExecuteSubmissionResponse["status"])) throw new Error("The API returned an invalid refund execute submission status.");
  const signature = record.signature;
  if (signature !== null && (typeof signature !== "string" || signature.length === 0 || signature.length > 128)) throw new Error("The API returned an invalid refund execute signature.");
  const providerIndex = record.provider_index;
  if (providerIndex !== null && (typeof providerIndex !== "number" || !Number.isSafeInteger(providerIndex) || providerIndex < 0)) throw new Error("The API returned an invalid refund provider index.");
  return { intent_id: uuidV7(record.intent_id, "refund execute intent ID"), status: record.status as RefundExecuteSubmissionResponse["status"], signature: signature as string | null, provider_index: providerIndex as number | null };
}

function encodeTransaction(transaction: Transaction): string {
  const bytes = transaction.serialize({ requireAllSignatures: false, verifySignatures: false });
  return btoa(String.fromCharCode(...bytes));
}

export function buildRefundExecuteTransaction(instruction: RefundExecuteInstruction, approver: PublicKey, refundFunder: PublicKey, connection: Connection): Promise<Transaction> {
  if (!new PublicKey(instruction.accounts[0].pubkey).equals(approver) || !new PublicKey(instruction.accounts[1].pubkey).equals(refundFunder)) throw new Error("Connected wallets do not match the compiled refund execute signer roles.");
  return connection.getLatestBlockhash("confirmed").then(({ blockhash }) => new Transaction({ feePayer: approver, recentBlockhash: blockhash }).add(new TransactionInstruction({
    programId: new PublicKey(instruction.program_id),
    data: Buffer.from(base64Bytes(instruction.instruction_data_base64, "refund execute instruction data")),
    keys: instruction.accounts.map((account) => ({ pubkey: new PublicKey(account.pubkey), isSigner: account.is_signer, isWritable: account.is_writable })),
  })));
}

export async function signRefundExecuteRole(instruction: RefundExecuteInstruction, role: "approver" | "refund_funder", expectedSigner: PublicKey, rpcUrl: string, serialized: string | null): Promise<string> {
  const wallet = await ensureConnectedWallet();
  if (!rpcUrl.trim()) throw new Error("A VITE_SOLANA_RPC_URL is required before refund execution signing is enabled.");
  if (!wallet.publicKey.equals(expectedSigner)) throw new Error(`Connected wallet must match the compiled ${role.replace("_", " ")} signer.`);
  const transaction = serialized ? Transaction.from(Buffer.from(serialized, "base64")) : await buildRefundExecuteTransaction(instruction, new PublicKey(instruction.accounts[0].pubkey), new PublicKey(instruction.accounts[1].pubkey), new Connection(rpcUrl, "confirmed"));
  return encodeTransaction(await signWalletTransaction(wallet, transaction, rpcUrl));
}

export function refundExecuteHasAllSignatures(serialized: string): boolean {
  const transaction = Transaction.from(Buffer.from(serialized, "base64"));
  return transaction.signatures.length >= 2 && transaction.signatures.slice(0, 2).every((entry) => entry.signature !== null);
}
