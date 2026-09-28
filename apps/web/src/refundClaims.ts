import { Connection, PublicKey, Transaction, TransactionInstruction } from "@solana/web3.js";
import { Buffer } from "buffer";
import { injectedWallet } from "./checkout";
import { parseCanonicalU64 } from "./settlementAdjustments";

export const REFUND_CLAIM_ACCOUNT_COUNT = 15;
export const SETTLEMENT_PROGRAM_ID = "4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi";
export const SYSTEM_PROGRAM_ID = "11111111111111111111111111111111";
export const SPL_TOKEN_PROGRAM_ID = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
const REFUND_CLAIM_DISCRIMINATOR_BASE64 = "eYjG9eQLZXM=";
const UUID_V7 = /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
const REFUND_STATUSES = [
  "prepared_unsubmitted",
  "signed",
  "submitting",
  "submitted",
  "processed",
  "confirmed",
  "finalized",
  "rejected",
  "unknown",
  "cancelled",
] as const;

export type RefundClaimStatus = typeof REFUND_STATUSES[number];

export type RefundClaimAccount = {
  pubkey: string;
  is_signer: boolean;
  is_writable: boolean;
};

export type RefundClaimInstruction = {
  status: "prepared_unsubmitted";
  program_id: string;
  instruction_data_base64: string;
  accounts: RefundClaimAccount[];
  registry: string;
  adjustment: string;
  original_receipt: string;
  claim: string;
  claim_nonce: string;
};

export type RefundClaimArtifact = {
  schema: string;
  instruction: RefundClaimInstruction;
  instruction_digest: number[];
  artifact_digest: number[];
  refund_destination: string;
  merchant_id: string;
  adjustment_id: string;
  funding_configuration_id: string;
  receipt_signature: string;
  receipt_slot: string | null;
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

export type RefundClaimResponse = {
  status: RefundClaimStatus;
  intent_id: string;
  request_digest: number[];
  instruction_digest: number[];
  artifact_digest: number[];
  claim: string;
  artifact: RefundClaimArtifact;
  signed_transaction_digest: number[] | null;
  signature: string | null;
  submitted_slot: string | null;
  commitment: string | null;
  last_error_code: string | null;
};

export type RefundClaimSubmissionResponse = {
  intent_id: string;
  status: RefundClaimStatus;
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

function optionalDigest(value: unknown, field: string): number[] | null {
  return value === null || value === undefined ? null : digest(value, field);
}

function boundedText(value: unknown, field: string, maxBytes: number): string {
  if (typeof value !== "string" || value.trim() === "" || new TextEncoder().encode(value).length > maxBytes) {
    throw new Error(`The API returned an invalid ${field}.`);
  }
  return value;
}

function optionalText(value: unknown, field: string, maxBytes: number): string | null {
  return value === null || value === undefined ? null : boundedText(value, field, maxBytes);
}

function optionalAtomic(value: unknown, field: string): string | null {
  return value === null || value === undefined ? null : parseCanonicalU64(value, field);
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

function parseAccounts(value: unknown): RefundClaimAccount[] {
  if (!Array.isArray(value) || value.length !== REFUND_CLAIM_ACCOUNT_COUNT) throw new Error("The API returned an invalid refund claim account list.");
  const keys = new Set<string>();
  return value.map((candidate, index) => {
    const account = objectValue(candidate, `The API returned an invalid refund claim account at position ${index}.`);
    if (typeof account.is_signer !== "boolean" || typeof account.is_writable !== "boolean") throw new Error(`The API returned invalid refund claim account flags at position ${index}.`);
    const parsed: RefundClaimAccount = {
      pubkey: publicKey(account.pubkey, `refund claim account ${index}`),
      is_signer: account.is_signer,
      is_writable: account.is_writable,
    };
    if (keys.has(parsed.pubkey)) throw new Error("The API returned duplicate refund claim accounts.");
    keys.add(parsed.pubkey);
    return parsed;
  });
}

export function parseRefundClaimInstruction(value: unknown): RefundClaimInstruction {
  const record = objectValue(value, "The API returned an invalid refund claim instruction.");
  if (record.status !== "prepared_unsubmitted") throw new Error("The refund claim instruction is not prepared for wallet review.");
  const instruction: RefundClaimInstruction = {
    status: "prepared_unsubmitted",
    program_id: publicKey(record.program_id, "refund claim program"),
    instruction_data_base64: typeof record.instruction_data_base64 === "string" ? record.instruction_data_base64 : "",
    accounts: parseAccounts(record.accounts),
    registry: publicKey(record.registry, "refund adjustment registry"),
    adjustment: publicKey(record.adjustment, "refund adjustment PDA"),
    original_receipt: publicKey(record.original_receipt, "original receipt PDA"),
    claim: publicKey(record.claim, "refund claim PDA"),
    claim_nonce: parseCanonicalU64(record.claim_nonce, "refund claim nonce"),
  };
  if (instruction.program_id !== SETTLEMENT_PROGRAM_ID || instruction.claim_nonce === "0") throw new Error("The API returned an unsupported refund claim instruction.");
  const data = base64Bytes(instruction.instruction_data_base64, "refund claim instruction data");
  if (data.length !== 64 || Buffer.from(data.subarray(0, 8)).toString("base64") !== REFUND_CLAIM_DISCRIMINATOR_BASE64) throw new Error("The API returned non-canonical refund claim instruction data.");
  const expectedFlags: Array<[boolean, boolean]> = [
    [true, true], [true, false], [false, false], [false, false], [false, false],
    [false, false], [false, false], [false, false], [false, false], [false, false],
    [false, false], [false, false], [false, false], [false, true], [false, false],
  ];
  instruction.accounts.forEach((account, index) => {
    const [signer, writable] = expectedFlags[index];
    if (account.is_signer !== signer || account.is_writable !== writable) throw new Error(`The refund claim account flags differ at position ${index}.`);
  });
  if (instruction.accounts[4].pubkey !== SPL_TOKEN_PROGRAM_ID || instruction.accounts[14].pubkey !== SYSTEM_PROGRAM_ID || instruction.accounts[6].pubkey !== instruction.registry || instruction.accounts[7].pubkey !== instruction.adjustment || instruction.accounts[8].pubkey !== instruction.original_receipt || instruction.accounts[13].pubkey !== instruction.claim) {
    throw new Error("The refund claim account topology is not bound to the compiled artifact.");
  }
  return instruction;
}

export function parseRefundClaimResponse(value: unknown): RefundClaimResponse {
  const record = objectValue(value, "The API returned an invalid refund claim response.");
  if (typeof record.status !== "string" || !REFUND_STATUSES.includes(record.status as RefundClaimStatus)) throw new Error("The API returned an invalid refund claim status.");
  const artifactRecord = objectValue(record.artifact, "The API returned an invalid refund claim artifact.");
  const artifact: RefundClaimArtifact = {
    schema: boundedText(artifactRecord.schema, "refund claim artifact schema", 128),
    instruction: parseRefundClaimInstruction(artifactRecord.instruction),
    instruction_digest: digest(artifactRecord.instruction_digest, "refund claim instruction"),
    artifact_digest: digest(artifactRecord.artifact_digest, "refund claim artifact"),
    refund_destination: publicKey(artifactRecord.refund_destination, "refund destination"),
    merchant_id: uuidV7(artifactRecord.merchant_id, "merchant ID"),
    adjustment_id: uuidV7(artifactRecord.adjustment_id, "adjustment ID"),
    funding_configuration_id: uuidV7(artifactRecord.funding_configuration_id, "funding configuration ID"),
    receipt_signature: boundedText(artifactRecord.receipt_signature, "receipt signature", 128),
    receipt_slot: optionalAtomic(artifactRecord.receipt_slot, "receipt slot"),
    proposer_subject: boundedText(artifactRecord.proposer_subject, "proposer subject", 256),
    refund_funder_subject: boundedText(artifactRecord.refund_funder_subject, "refund funder subject", 256),
    proposer: publicKey(artifactRecord.proposer, "refund proposer"),
    refund_funder: publicKey(artifactRecord.refund_funder, "refund funder"),
    merchant: publicKey(artifactRecord.merchant, "refund merchant"),
    mint: publicKey(artifactRecord.mint, "refund mint"),
    token_program: publicKey(artifactRecord.token_program, "refund token program"),
    tax_vault: publicKey(artifactRecord.tax_vault, "refund tax vault"),
    net_source: publicKey(artifactRecord.net_source, "refund net source"),
    gross_atomic: optionalAtomic(artifactRecord.gross_atomic, "gross amount"),
    tax_atomic: optionalAtomic(artifactRecord.tax_atomic, "tax amount"),
    net_atomic: optionalAtomic(artifactRecord.net_atomic, "net amount"),
    execute_after_unix: optionalUnix(artifactRecord.execute_after_unix, "execute-after time"),
    expires_at_unix: optionalUnix(artifactRecord.expires_at_unix, "expiry time"),
  };
  if (typeof artifact.schema !== "string" || artifact.schema !== "refund-movement-artifact/v1") throw new Error("The API returned an unsupported refund claim artifact schema.");
  return {
    status: record.status as RefundClaimStatus,
    intent_id: uuidV7(record.intent_id, "refund claim intent ID"),
    request_digest: digest(record.request_digest, "refund claim request"),
    instruction_digest: digest(record.instruction_digest, "refund claim instruction"),
    artifact_digest: digest(record.artifact_digest, "refund claim artifact"),
    claim: publicKey(record.claim, "refund claim"),
    artifact,
    signed_transaction_digest: optionalDigest(record.signed_transaction_digest, "signed transaction"),
    signature: optionalText(record.signature, "refund claim signature", 128),
    submitted_slot: optionalAtomic(record.submitted_slot, "submitted slot"),
    commitment: optionalText(record.commitment, "commitment", 32),
    last_error_code: optionalText(record.last_error_code, "last error code", 128),
  };
}

export function parseRefundClaimSubmissionResponse(value: unknown): RefundClaimSubmissionResponse {
  const record = objectValue(value, "The API returned an invalid refund claim submission response.");
  if (typeof record.status !== "string" || !REFUND_STATUSES.includes(record.status as RefundClaimStatus)) throw new Error("The API returned an invalid refund claim submission status.");
  const signature = record.signature;
  if (signature !== null && (typeof signature !== "string" || signature.length === 0 || signature.length > 128)) throw new Error("The API returned an invalid refund claim signature.");
  const providerIndex = record.provider_index;
  if (providerIndex !== null && (typeof providerIndex !== "number" || !Number.isSafeInteger(providerIndex) || providerIndex < 0)) throw new Error("The API returned an invalid refund provider index.");
  return { intent_id: uuidV7(record.intent_id, "refund claim intent ID"), status: record.status as RefundClaimStatus, signature: signature as string | null, provider_index: providerIndex as number | null };
}

function validateInstruction(instruction: RefundClaimInstruction, proposer: PublicKey, refundFunder: PublicKey): void {
  if (!new PublicKey(instruction.accounts[0].pubkey).equals(proposer) || !new PublicKey(instruction.accounts[1].pubkey).equals(refundFunder)) throw new Error("Connected wallets do not match the compiled refund claim signer roles.");
}

function encodeTransaction(transaction: Transaction, requireAllSignatures: boolean): string {
  const bytes = transaction.serialize({ requireAllSignatures, verifySignatures: requireAllSignatures });
  return btoa(String.fromCharCode(...bytes));
}

export function buildRefundClaimTransaction(instruction: RefundClaimInstruction, proposer: PublicKey, refundFunder: PublicKey, connection: Connection): Promise<Transaction> {
  validateInstruction(instruction, proposer, refundFunder);
  return connection.getLatestBlockhash("confirmed").then(({ blockhash }) => new Transaction({ feePayer: proposer, recentBlockhash: blockhash }).add(new TransactionInstruction({
    programId: new PublicKey(instruction.program_id),
    data: Buffer.from(base64Bytes(instruction.instruction_data_base64, "refund claim instruction data")),
    keys: instruction.accounts.map((account) => ({ pubkey: new PublicKey(account.pubkey), isSigner: account.is_signer, isWritable: account.is_writable })),
  })));
}

export async function signRefundClaimRole(instruction: RefundClaimInstruction, role: "proposer" | "refund_funder", expectedSigner: PublicKey, rpcUrl: string, serialized: string | null): Promise<string> {
  const wallet = injectedWallet();
  if (!wallet) throw new Error("No compatible injected Solana wallet was detected.");
  if (!rpcUrl.trim()) throw new Error("A VITE_SOLANA_RPC_URL is required before refund signing is enabled.");
  const connected = await wallet.connect();
  if (!connected.publicKey.equals(expectedSigner)) throw new Error(`Connected wallet must match the compiled ${role.replace("_", " ")} signer.`);
  const transaction = serialized
    ? Transaction.from(Buffer.from(serialized, "base64"))
    : await buildRefundClaimTransaction(instruction, new PublicKey(instruction.accounts[0].pubkey), new PublicKey(instruction.accounts[1].pubkey), new Connection(rpcUrl, "confirmed"));
  const signed = await wallet.signTransaction(transaction);
  return encodeTransaction(signed, false);
}

export function refundClaimHasAllSignatures(serialized: string): boolean {
  const transaction = Transaction.from(Buffer.from(serialized, "base64"));
  return transaction.signatures.length >= 2 && transaction.signatures.slice(0, 2).every((entry) => entry.signature !== null);
}
