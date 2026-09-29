import { readFile } from "node:fs/promises";
import {
  Connection,
  Keypair,
  PublicKey,
  Transaction,
  TransactionInstruction,
} from "@solana/web3.js";

const apiBase = required("API_BASE");
const rpcFixtureUrl = required("RPC_FIXTURE_URL");
const authorization = required("AUTHORIZATION");
const artifactFile = required("ARTIFACT_FILE");
const intentKind = required("INTENT_KIND");
const signerSeedHex = required("SIGNER_SEED_HEX");
const refundFunderSeedHex = required("REFUND_FUNDER_SEED_HEX");
const feePayerSeedHex = required("FEE_PAYER_SEED_HEX");
const initialSendCount = Number(required("INITIAL_SEND_COUNT"));
const response = JSON.parse(await readFile(artifactFile, "utf8"));
const artifact = response.artifact;

if (intentKind !== "claim" && intentKind !== "execute") {
  throw new Error("INTENT_KIND must be claim or execute");
}
if (!Number.isSafeInteger(initialSendCount) || initialSendCount < 0) {
  throw new Error("INITIAL_SEND_COUNT must be a non-negative safe integer");
}

const instruction = artifact.instruction;
const signer = Keypair.fromSeed(seedBytes(signerSeedHex));
const refundFunder = Keypair.fromSeed(seedBytes(refundFunderSeedHex));
const feePayer = Keypair.fromSeed(seedBytes(feePayerSeedHex));
if (instruction.accounts.length < 2) {
  throw new Error("refund instruction does not contain both authority accounts");
}
if (instruction.accounts[0].pubkey !== signer.publicKey.toBase58()) {
  throw new Error("server artifact signer does not match the synthetic wallet key");
}
if (instruction.accounts[1].pubkey !== refundFunder.publicKey.toBase58()) {
  throw new Error("server artifact funder does not match the synthetic wallet key");
}
if (feePayer.publicKey.equals(signer.publicKey) || feePayer.publicKey.equals(refundFunder.publicKey)) {
  throw new Error("synthetic fee payer must be distinct from movement signers");
}

const connection = new Connection(rpcFixtureUrl, "confirmed");
const { blockhash } = await connection.getLatestBlockhash("confirmed");
const transaction = new Transaction({
  feePayer: feePayer.publicKey,
  recentBlockhash: blockhash,
}).add(new TransactionInstruction({
  programId: new PublicKey(instruction.program_id),
  data: Buffer.from(instruction.instruction_data_base64, "base64"),
  keys: instruction.accounts.map((account) => ({
    pubkey: new PublicKey(account.pubkey),
    isSigner: account.is_signer,
    isWritable: account.is_writable,
  })),
}));
transaction.sign(feePayer, signer, refundFunder);
const signedTransactionBase64 = transaction.serialize({
  requireAllSignatures: true,
  verifySignatures: true,
}).toString("base64");

const submitPath = intentKind === "claim"
  ? "/v1/settlement/refund-claims/submit"
  : "/v1/settlement/refund-executions/submit";
const submitBody = JSON.stringify({
  intent_id: response.intent_id,
  signed_transaction_base64: signedTransactionBase64,
});
const [first, second] = await Promise.all([
  request(apiBase + submitPath, submitBody),
  request(apiBase + submitPath, submitBody),
]);
const responses = [first, second];
const statusCodes = responses.map((item) => item.status);
if (statusCodes.some((status) => status !== 200 && status !== 409)
    || !statusCodes.includes(200)) {
  throw new Error("unexpected concurrent refund submission responses: " + statusCodes.join(","));
}

const sendCountResponse = await fetch(rpcFixtureUrl + "/admin/send-count");
if (!sendCountResponse.ok) {
  throw new Error("RPC fixture send-count failed with HTTP " + sendCountResponse.status);
}
const sendCount = await sendCountResponse.json();
if (sendCount.send_transaction_calls !== initialSendCount + 1) {
  throw new Error("expected exactly one additional refund send, received "
    + (sendCount.send_transaction_calls - initialSendCount));
}

console.log(JSON.stringify({
  intent_kind: intentKind,
  intent_id: response.intent_id,
  status_codes: statusCodes,
  statuses: responses.map((item) => item.payload?.status ?? null),
  send_transaction_calls: sendCount.send_transaction_calls,
  signature: responses.find((item) => typeof item.payload?.signature === "string")?.payload.signature ?? null,
}));

function seedBytes(value) {
  if (!/^[0-9a-f]{64}$/i.test(value)) {
    throw new Error("wallet seed must be exactly 32 bytes of hexadecimal");
  }
  return Buffer.from(value, "hex");
}

async function request(url, body) {
  const response = await fetch(url, {
    method: "POST",
    headers: { "content-type": "application/json", authorization },
    body,
  });
  return {
    status: response.status,
    payload: await response.json().catch(() => null),
  };
}

function required(name) {
  const value = process.env[name];
  if (!value || !value.trim()) {
    throw new Error(name + " is required");
  }
  return value;
}
