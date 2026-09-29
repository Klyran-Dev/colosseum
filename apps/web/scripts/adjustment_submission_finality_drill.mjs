import { readFile } from "node:fs/promises";
import crypto from "node:crypto";
import {
  Connection,
  Keypair,
  PublicKey,
  Transaction,
  TransactionInstruction,
} from "@solana/web3.js";

const apiBase = required("API_BASE");
const rpcUrl = required("RPC_FIXTURE_URL");
const authorization = required("AUTHORIZATION");
const artifactFile = required("ARTIFACT_FILE");
const approverSeedHex = required("APPROVER_SEED_HEX");
const adjustmentDigestHex = requiredHex("ADJUSTMENT_DIGEST_HEX");
const adjustmentEvidenceDigestHex = requiredHex("ADJUSTMENT_EVIDENCE_DIGEST_HEX");
const initialSendCount = Number(required("INITIAL_SEND_COUNT"));
const artifactResponse = JSON.parse(await readFile(artifactFile, "utf8"));
const artifact = artifactResponse.instruction;
if (!artifact || artifact.status !== "prepared_unsubmitted") {
  throw new Error("compiled adjustment artifact is not prepared for submission");
}
if (!Array.isArray(artifact.accounts) || artifact.accounts.length !== 10) {
  throw new Error("compiled adjustment artifact must contain exactly ten accounts");
}

const approver = Keypair.fromSeed(Buffer.from(approverSeedHex, "hex"));
if (artifact.accounts[0].pubkey !== approver.publicKey.toBase58()
    || !artifact.accounts[0].is_signer
    || !artifact.accounts[0].is_writable) {
  throw new Error("compiled adjustment signer is not the expected synthetic approver wallet");
}

const connection = new Connection(rpcUrl, "confirmed");
const { blockhash } = await connection.getLatestBlockhash("confirmed");
const transaction = new Transaction({
  feePayer: approver.publicKey,
  recentBlockhash: blockhash,
}).add(new TransactionInstruction({
  programId: new PublicKey(artifact.program_id),
  data: Buffer.from(artifact.instruction_data_base64, "base64"),
  keys: artifact.accounts.map((account) => ({
    pubkey: new PublicKey(account.pubkey),
    isSigner: account.is_signer,
    isWritable: account.is_writable,
  })),
}));
transaction.sign(approver);
const signedTransactionBase64 = transaction.serialize({
  requireAllSignatures: true,
  verifySignatures: true,
}).toString("base64");
const requestBody = JSON.stringify({
  chain_intent_id: artifactResponse.chain_intent_id,
  signed_transaction_base64: signedTransactionBase64,
  instruction: artifact,
});

const [first, second] = await Promise.all([
  request(`${apiBase}/v1/settlement/adjustment-chain-intents/submit`, requestBody),
  request(`${apiBase}/v1/settlement/adjustment-chain-intents/submit`, requestBody),
]);
const responses = [first, second];
const statuses = responses.map((response) => response.status).sort((a, b) => a - b);
if (statuses[0] !== 200 || statuses[1] !== 409) {
  throw new Error(`expected one successful and one fenced submission, received ${statuses.join(",")}`);
}
const submission = responses.find((response) => response.status === 200)?.payload;
if (!submission || submission.status !== "submitted" || typeof submission.signature !== "string") {
  throw new Error(`adjustment submission did not return a durable submitted state: ${JSON.stringify(submission)}`);
}

const slot = 42;
const eventIndex = 0;
const eventData = buildEventData({
  adjustment: artifact.adjustment,
  originalReceipt: artifact.original_receipt,
  merchant: artifact.accounts[2].pubkey,
  mint: artifact.accounts[3].pubkey,
  nonce: BigInt(artifact.adjustment_nonce),
  adjustmentDigest: Buffer.from(adjustmentDigestHex, "hex"),
  evidenceDigest: Buffer.from(adjustmentEvidenceDigestHex, "hex"),
  approver: artifact.accounts[0].pubkey,
});
const eventEvidenceDigest = crypto.createHash("sha256")
  .update("solana-tax-protocol/settlement-event/v1")
  .update(submission.signature, "utf8")
  .update(uint64le(BigInt(slot)))
  .update(uint16le(eventIndex))
  .update(eventData)
  .digest("hex");

const sendCount = await fetch(`${rpcUrl}/admin/send-count`).then(expectJson);
if (sendCount.send_transaction_calls !== initialSendCount + 1) {
  throw new Error(`expected one RPC submission, received ${sendCount.send_transaction_calls - initialSendCount}`);
}

console.log(JSON.stringify({
  chain_intent_id: artifactResponse.chain_intent_id,
  adjustment_id: artifactResponse.intent_id,
  signature: submission.signature,
  slot,
  event_index: eventIndex,
  adjustment_hex: publicKeyHex(artifact.adjustment),
  event_data_hex: eventData.toString("hex"),
  event_evidence_digest_hex: eventEvidenceDigest,
  submission_statuses: statuses,
  send_transaction_calls: sendCount.send_transaction_calls,
}));

async function request(url, body) {
  const response = await fetch(url, {
    method: "POST",
    headers: { "content-type": "application/json", authorization },
    body,
  });
  return { status: response.status, payload: await response.json().catch(() => null) };
}

async function expectJson(response) {
  if (!response.ok) throw new Error(`fixture request failed with HTTP ${response.status}`);
  return response.json();
}

function buildEventData({ adjustment, originalReceipt, merchant, mint, nonce, adjustmentDigest, evidenceDigest, approver }) {
  return Buffer.concat([
    Buffer.from([208, 86, 69, 183, 44, 203, 236, 75]),
    publicKeyBytes(adjustment),
    publicKeyBytes(originalReceipt),
    publicKeyBytes(merchant),
    publicKeyBytes(mint),
    uint64le(nonce),
    Buffer.from([1, 1]), uint64le(250),
    Buffer.from([1]), uint64le(25),
    Buffer.from([1]), uint64le(225),
    adjustmentDigest,
    evidenceDigest,
    publicKeyBytes(approver),
  ]);
}

function publicKeyBytes(value) {
  return new PublicKey(value).toBytes();
}

function publicKeyHex(value) {
  return Buffer.from(publicKeyBytes(value)).toString("hex");
}

function uint64le(value) {
  const output = Buffer.alloc(8);
  output.writeBigUInt64LE(BigInt(value));
  return output;
}

function uint16le(value) {
  const output = Buffer.alloc(2);
  output.writeUInt16LE(value);
  return output;
}

function required(name) {
  const value = process.env[name];
  if (!value || !value.trim()) throw new Error(`${name} is required`);
  return value;
}

function requiredHex(name) {
  const value = required(name);
  if (!/^[0-9a-f]{64}$/i.test(value)) throw new Error(`${name} must contain exactly 32 bytes of hex`);
  return value.toLowerCase();
}
