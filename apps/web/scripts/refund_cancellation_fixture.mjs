import { Keypair, PublicKey } from "@solana/web3.js";
import { createHash } from "node:crypto";

const programId = new PublicKey("4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi");
const tokenProgram = new PublicKey("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
const merchant = key(3);
const mint = key(4);
const adjustment = key(1);
const originalReceipt = key(2);
const payer = key(5);
const refundFunder = key(6);
const taxVault = key(7);
const netSource = key(8);
const refundDestination = key(9);
const proposer = key(10);
const governance = key(15);
const nonce = 11n;
const executeAfter = 1n;
const expiresAt = 9_999_999_999n;
const referenceDigest = Buffer.alloc(32, 0x1f);
const [claim] = PublicKey.findProgramAddressSync(
  [Buffer.from("refund-claim"), adjustment.toBuffer(), u64(nonce)],
  programId,
);
const [registry] = PublicKey.findProgramAddressSync(
  [Buffer.from("adjustment-registry"), merchant.toBuffer(), mint.toBuffer()],
  programId,
);
const [protocolConfig] = PublicKey.findProgramAddressSync(
  [Buffer.from("protocol-config")],
  programId,
);
const [vaultAuthority] = PublicKey.findProgramAddressSync(
  [Buffer.from("vault-authority"), merchant.toBuffer(), mint.toBuffer()],
  programId,
);
const cancellationSignature = base58(Buffer.concat([key(16).toBytes(), key(17).toBytes()]));
const instructionData = Buffer.concat([
  anchorDiscriminator("global:create_adjustment_refund_claim"),
  u64(nonce),
  referenceDigest,
  i64(executeAfter),
  i64(expiresAt),
]);
const instruction = {
  status: "prepared_unsubmitted",
  program_id: programId.toBase58(),
  instruction_data_base64: instructionData.toString("base64"),
  accounts: [
    account(proposer, true, true),
    account(refundFunder, true, false),
    account(merchant, false, false),
    account(mint, false, false),
    account(tokenProgram, false, false),
    account(protocolConfig, false, false),
    account(registry, false, false),
    account(adjustment, false, false),
    account(originalReceipt, false, false),
    account(taxVault, false, false),
    account(vaultAuthority, false, false),
    account(netSource, false, false),
    account(refundDestination, false, false),
    account(claim, false, true),
    account(new PublicKey("11111111111111111111111111111111"), false, false),
  ],
  registry: registry.toBase58(),
  adjustment: adjustment.toBase58(),
  original_receipt: originalReceipt.toBase58(),
  claim: claim.toBase58(),
  claim_nonce: nonce.toString(),
};
const instructionDigest = sha256(
  Buffer.concat([Buffer.from("refund-instruction/v1"), Buffer.from(JSON.stringify(instruction))]),
);
const claimArtifact = {
  schema: "refund-movement-artifact/v1",
  instruction,
  instruction_digest: [...instructionDigest],
  artifact_digest: [...Buffer.alloc(32, 0x39)],
  refund_destination: refundDestination.toBase58(),
  merchant_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d51",
  adjustment_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d52",
  funding_configuration_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d53",
  funding_configuration_digest: [...Buffer.alloc(32, 0x23)],
  receipt_signature: "refund-cancellation-receipt",
  receipt_slot: "700",
  claim_signature: "refund-cancellation-claim",
  claim_slot: "800",
  authority_subject: proposer.toBase58(),
  proposer_subject: "refund-cancellation-proposer",
  refund_funder_subject: "refund-cancellation-funder",
  proposer: proposer.toBase58(),
  refund_funder: refundFunder.toBase58(),
  merchant: merchant.toBase58(),
  mint: mint.toBase58(),
  token_program: tokenProgram.toBase58(),
  tax_vault: taxVault.toBase58(),
  net_source: netSource.toBase58(),
  gross_atomic: "100",
  tax_atomic: "25",
  net_atomic: "75",
  execute_after_unix: Number(executeAfter),
  expires_at_unix: Number(expiresAt),
};
const cancellationEvent = Buffer.concat([
  Buffer.from([201, 89, 79, 76, 75, 107, 22, 3]),
  claim.toBuffer(),
  adjustment.toBuffer(),
  merchant.toBuffer(),
  mint.toBuffer(),
  u64(nonce),
  governance.toBuffer(),
  referenceDigest,
]);
const evidenceDigest = settlementEvidenceDigest(cancellationSignature, 901, 0, cancellationEvent);

console.log(JSON.stringify({
  claim: claim.toBase58(),
  claim_hex: claim.toBuffer().toString("hex"),
  adjustment: adjustment.toBase58(),
  adjustment_hex: adjustment.toBuffer().toString("hex"),
  merchant: merchant.toBase58(),
  merchant_hex: merchant.toBuffer().toString("hex"),
  mint: mint.toBase58(),
  mint_hex: mint.toBuffer().toString("hex"),
  original_receipt: originalReceipt.toBase58(),
  payer: payer.toBase58(),
  refund_funder: refundFunder.toBase58(),
  tax_vault: taxVault.toBase58(),
  net_source: netSource.toBase58(),
  refund_destination: refundDestination.toBase58(),
  proposer: proposer.toBase58(),
  vault_authority: vaultAuthority.toBase58(),
  token_program: tokenProgram.toBase58(),
  protocol_config: protocolConfig.toBase58(),
  registry: registry.toBase58(),
  original_receipt_hex: originalReceipt.toBuffer().toString("hex"),
  payer_hex: payer.toBuffer().toString("hex"),
  refund_funder_hex: refundFunder.toBuffer().toString("hex"),
  tax_vault_hex: taxVault.toBuffer().toString("hex"),
  net_source_hex: netSource.toBuffer().toString("hex"),
  refund_destination_hex: refundDestination.toBuffer().toString("hex"),
  proposer_hex: proposer.toBuffer().toString("hex"),
  governance: governance.toBase58(),
  governance_hex: governance.toBuffer().toString("hex"),
  reference_digest_hex: referenceDigest.toString("hex"),
  instruction_digest_hex: instructionDigest.toString("hex"),
  cancellation_signature: cancellationSignature,
  cancellation_slot: "901",
  cancellation_event_hex: cancellationEvent.toString("hex"),
  cancellation_evidence_digest_hex: evidenceDigest.toString("hex"),
  artifact: claimArtifact,
}));

function key(seed) {
  return Keypair.fromSeed(Buffer.alloc(32, seed)).publicKey;
}

function account(pubkey, isSigner, isWritable) {
  return { pubkey: pubkey.toBase58(), is_signer: isSigner, is_writable: isWritable };
}

function u64(value) {
  const bytes = Buffer.alloc(8);
  bytes.writeBigUInt64LE(value);
  return bytes;
}

function i64(value) {
  const bytes = Buffer.alloc(8);
  bytes.writeBigInt64LE(value);
  return bytes;
}

function anchorDiscriminator(name) {
  return sha256(Buffer.from(name)).subarray(0, 8);
}

function sha256(value) {
  return createHash("sha256").update(value).digest();
}

function settlementEvidenceDigest(signature, slot, eventIndex, eventData) {
  return sha256(Buffer.concat([
    Buffer.from("solana-tax-protocol/settlement-event/v1"),
    Buffer.from(signature),
    u64(BigInt(slot)),
    Buffer.from([eventIndex & 0xff, (eventIndex >> 8) & 0xff]),
    eventData,
  ]));
}

function base58(bytes) {
  const alphabet = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
  let value = 0n;
  for (const byte of bytes) value = (value << 8n) | BigInt(byte);
  let encoded = "";
  while (value > 0n) {
    encoded = alphabet[Number(value % 58n)] + encoded;
    value /= 58n;
  }
  for (const byte of bytes) {
    if (byte !== 0) break;
    encoded = `1${encoded}`;
  }
  return encoded || "1";
}
