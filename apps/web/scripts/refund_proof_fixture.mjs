import { Keypair, PublicKey } from "@solana/web3.js";

const programId = new PublicKey("4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi");
const tokenProgram = new PublicKey("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
const ids = {
  merchant: key(3),
  mint: key(4),
  payer: key(5),
  refund_funder: key(6),
  tax_vault: key(7),
  refund_source: key(8),
  payer_destination: key(9),
  approver: key(10),
  signature: key(12),
  adjustment: key(1),
  original_receipt: key(2),
};
const nonce = 11n;
const [claim] = PublicKey.findProgramAddressSync(
  [Buffer.from("refund-claim"), ids.adjustment.toBuffer(), u64(nonce)],
  programId,
);
const [vaultAuthority] = PublicKey.findProgramAddressSync(
  [Buffer.from("vault-authority"), ids.merchant.toBuffer(), ids.mint.toBuffer()],
  programId,
);
const referenceDigest = Buffer.alloc(32, 0x1f);
const artifact = {
  schema: "refund-movement-artifact/v1",
  instruction: {
    status: "prepared_unsubmitted",
    program_id: programId.toBase58(),
    instruction_data_base64: "",
    accounts: [
      account(ids.approver, true, true),
      account(ids.refund_funder, true, true),
      account(ids.merchant, false, false),
      account(ids.mint, false, false),
      account(tokenProgram, false, false),
      account(key(13), false, false),
      account(key(14), false, false),
      account(ids.adjustment, false, false),
      account(ids.original_receipt, false, false),
      account(claim, false, true),
      account(ids.tax_vault, false, true),
      account(ids.refund_source, false, true),
      account(ids.payer_destination, false, true),
    ],
    registry: key(14).toBase58(),
    adjustment: ids.adjustment.toBase58(),
    original_receipt: ids.original_receipt.toBase58(),
    claim: claim.toBase58(),
    claim_nonce: nonce.toString(),
  },
  instruction_digest: bytes(0x21),
  artifact_digest: bytes(0x22),
  refund_destination: ids.payer_destination.toBase58(),
  merchant_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d51",
  adjustment_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d52",
  funding_configuration_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d53",
  funding_configuration_digest: bytes(0x23),
  receipt_signature: "refund-proof-receipt",
  receipt_slot: "700",
  claim_signature: "refund-proof-claim",
  claim_slot: "800",
  authority_subject: "refund-proof-approver",
  proposer_subject: "",
  refund_funder_subject: "refund-proof-funder",
  proposer: "",
  refund_funder: ids.refund_funder.toBase58(),
  merchant: ids.merchant.toBase58(),
  mint: ids.mint.toBase58(),
  token_program: tokenProgram.toBase58(),
  tax_vault: ids.tax_vault.toBase58(),
  net_source: ids.refund_source.toBase58(),
  gross_atomic: null,
  tax_atomic: null,
  net_atomic: null,
  execute_after_unix: null,
  expires_at_unix: null,
};

const output = {
  claim: claim.toBase58(),
  adjustment: ids.adjustment.toBase58(),
  original_receipt: ids.original_receipt.toBase58(),
  merchant: ids.merchant.toBase58(),
  mint: ids.mint.toBase58(),
  payer: ids.payer.toBase58(),
  refund_funder: ids.refund_funder.toBase58(),
  tax_vault: ids.tax_vault.toBase58(),
  refund_source: ids.refund_source.toBase58(),
  payer_destination: ids.payer_destination.toBase58(),
  approver: ids.approver.toBase58(),
  vault_authority: vaultAuthority.toBase58(),
  signature: ids.signature.toBase58(),
  reference_digest_hex: referenceDigest.toString("hex"),
  nonce: "11",
  gross: "100",
  tax: "25",
  net: "75",
  slot: "900",
  artifact,
  hex: Object.fromEntries(Object.entries(ids).map(([name, value]) => [name, value.toBuffer().toString("hex")])),
  claim_hex: claim.toBuffer().toString("hex"),
};
console.log(JSON.stringify(output));

function key(byte) {
  return Keypair.fromSeed(Buffer.alloc(32, byte)).publicKey;
}

function bytes(byte) {
  return Array.from(Buffer.alloc(32, byte));
}

function u64(value) {
  const bytes = Buffer.alloc(8);
  bytes.writeBigUInt64LE(value);
  return bytes;
}

function account(pubkey, isSigner, isWritable) {
  return { pubkey: pubkey.toBase58(), is_signer: isSigner, is_writable: isWritable };
}
