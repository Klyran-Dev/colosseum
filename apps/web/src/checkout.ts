import {
  Connection,
  ComputeBudgetProgram,
  PublicKey,
  Transaction,
  TransactionInstruction,
} from "@solana/web3.js";
import { Buffer } from "buffer";
import type {
  AdjustmentChainIntentResponse,
  AdjustmentChainIntentStatus,
  CompiledSettlementAdjustmentArtifact,
  CompiledSettlementAdjustmentInstruction,
} from "./settlementAdjustments";
import { parseCanonicalU64, validateByteArray } from "./settlementAdjustments";
export { parseCanonicalU64 } from "./settlementAdjustments";

export const SETTLEMENT_PROGRAM_ID = "4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi";
export const SYSTEM_PROGRAM_ID = "11111111111111111111111111111111";
export const SPL_TOKEN_PROGRAM_ID = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
export const TOKEN_2022_PROGRAM_ID = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";
export const COMPUTE_BUDGET_PROGRAM_ID = "ComputeBudget111111111111111111111111111111";
export const SETTLEMENT_INSTRUCTION_ACCOUNT_COUNT = 12;
const SETTLE_DISCRIMINATOR_BASE64 = "ryq5V5CDZtQ=";
const SPLIT_SETTLE_DISCRIMINATOR_BASE64 = "A6qY2V4JCdU=";
const ADJUSTMENT_RECORDED_EVENT_DISCRIMINATOR = [208, 86, 69, 183, 44, 203, 236, 75] as const;

function proofNumber(value: unknown, label: string, minimum: number, maximum: number): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < minimum || value > maximum) {
    throw new Error(`The API returned an invalid ${label}.`);
  }
  return value;
}

export type SettlementRoute = {
  id: string;
  route_key: string;
  version: number;
  merchant_account: string;
  mint: string;
  tax_vault: string;
  net_destination: string;
  token_program: string;
};

const UUIDV7_PATTERN = /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;

function recordValue(value: unknown, message: string): Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error(message);
  return value as Record<string, unknown>;
}

function parseUuidV7(value: unknown, field: string): string {
  if (typeof value !== "string") throw new Error(`The API returned an invalid ${field}.`);
  const normalized = value.toLowerCase();
  if (!UUIDV7_PATTERN.test(normalized)) throw new Error(`The API returned an invalid UUIDv7 ${field}.`);
  return normalized;
}

function parseDigestBytes(value: unknown, field: string): number[] {
  if (!Array.isArray(value) || value.length !== 32
      || value.some((byte) => typeof byte !== "number"
        || !Number.isInteger(byte) || byte < 0 || byte > 255)) {
    throw new Error(`The API returned an invalid ${field} digest.`);
  }
  return [...value] as number[];
}

function boundedText(value: unknown, field: string, maxBytes: number): string {
  if (typeof value !== "string" || value.trim() === "" || new TextEncoder().encode(value).length > maxBytes) {
    throw new Error(`The API returned an invalid ${field}.`);
  }
  return value;
}

function parseVersion(value: unknown, field: string): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value <= 0) {
    throw new Error(`The API returned an invalid ${field} version.`);
  }
  return value;
}

export function parseSettlementRoutes(value: unknown): SettlementRoute[] {
  if (!Array.isArray(value) || value.length > 64) {
    throw new Error("The API returned an invalid or unbounded settlement route list.");
  }
  const ids = new Set<string>();
  return value.map((candidate) => {
    const route = recordValue(candidate, "The API returned an invalid settlement route.");
    const id = parseUuidV7(route.id, "settlement route ID");
    if (ids.has(id)) throw new Error("The API returned duplicate settlement route IDs.");
    ids.add(id);
    const tokenProgram = route.token_program;
    if (tokenProgram !== SPL_TOKEN_PROGRAM_ID && tokenProgram !== TOKEN_2022_PROGRAM_ID) {
      throw new Error("The API returned an unsupported settlement token program.");
    }
    return {
      id,
      route_key: boundedText(route.route_key, "settlement route key", 128),
      version: parseVersion(route.version, "settlement route"),
      merchant_account: requirePublicKey(route.merchant_account as string, "route merchant account").toBase58(),
      mint: requirePublicKey(route.mint as string, "route mint").toBase58(),
      tax_vault: requirePublicKey(route.tax_vault as string, "route tax vault").toBase58(),
      net_destination: requirePublicKey(route.net_destination as string, "route net destination").toBase58(),
      token_program: tokenProgram,
    };
  });
}

export type SettlementSplitPlan = {
  id: string;
  plan_key: string;
  version: number;
  merchant_account: string;
  mint: string;
  plan_digest: number[];
  recipients: Array<{ token_account: string; basis_points: number }>;
};

export function parseSettlementSplitPlans(value: unknown): SettlementSplitPlan[] {
  if (!Array.isArray(value) || value.length > 64) {
    throw new Error("The API returned an invalid or unbounded Split Plan list.");
  }
  return value.map((candidate) => {
    if (!candidate || typeof candidate !== "object") {
      throw new Error("The API returned an invalid Split Plan record.");
    }
    const record = candidate as Record<string, unknown>;
    const id = record.id;
    const planKey = record.plan_key;
    const version = record.version;
    const merchantAccount = record.merchant_account;
    const mint = record.mint;
    const digest = record.plan_digest;
    const recipients = record.recipients;
    if (typeof id !== "string" || !id
        || typeof planKey !== "string" || !planKey
        || typeof version !== "number" || !Number.isSafeInteger(version) || version <= 0
        || typeof merchantAccount !== "string" || typeof mint !== "string"
        || !Array.isArray(digest) || digest.length !== 32
        || !Array.isArray(recipients) || recipients.length === 0 || recipients.length > 8) {
      throw new Error("The API returned an invalid Split Plan shape.");
    }
    const digestBytes = digest as unknown[];
    if (digestBytes.some((byte) => typeof byte !== "number"
        || !Number.isInteger(byte) || byte < 0 || byte > 255)) {
      throw new Error("The API returned an invalid Split Plan digest.");
    }
    let totalBasisPoints = 0;
    const recipientAccounts = new Set<string>();
    const parsedRecipients = (recipients as unknown[]).map((candidateRecipient) => {
      if (!candidateRecipient || typeof candidateRecipient !== "object") {
        throw new Error("The API returned an invalid Split Plan recipient.");
      }
      const recipient = candidateRecipient as Record<string, unknown>;
      if (typeof recipient.token_account !== "string"
          || typeof recipient.basis_points !== "number"
          || !Number.isSafeInteger(recipient.basis_points)
          || recipient.basis_points <= 0
          || recipient.basis_points > 10_000) {
        throw new Error("The API returned an invalid Split Plan allocation.");
      }
      const tokenAccount = requirePublicKey(recipient.token_account, "Split Plan recipient").toBase58();
      if (recipientAccounts.has(tokenAccount)) {
        throw new Error("The API returned duplicate Split Plan recipients.");
      }
      recipientAccounts.add(tokenAccount);
      totalBasisPoints += recipient.basis_points;
      return { token_account: tokenAccount, basis_points: recipient.basis_points };
    });
    if (totalBasisPoints !== 10_000) {
      throw new Error("The API returned a non-conserving Split Plan allocation.");
    }
    requirePublicKey(merchantAccount, "Split Plan merchant account");
    requirePublicKey(mint, "Split Plan mint");
    return {
      id,
      plan_key: planKey,
      version,
      merchant_account: merchantAccount,
      mint,
      plan_digest: digestBytes as number[],
      recipients: parsedRecipients,
    };
  });
}

export type CompiledAccountMeta = {
  pubkey: string;
  is_signer: boolean;
  is_writable: boolean;
};

export type ComputeBudgetPolicy = {
  policy_version: number;
  compute_unit_limit: number;
  micro_lamports_per_compute_unit: number;
};

export type CompiledSettlementInstruction = {
  status: string;
  program_id: string;
  instruction_data_base64: string;
  accounts: CompiledAccountMeta[];
  protocol_config: string;
  vault_authority: string;
  policy_attestation: string;
  receipt: string;
  compute_budget_policy: ComputeBudgetPolicy;
};

export type CompiledSplitSettlementInstruction = {
  status: string;
  program_id: string;
  instruction_data_base64: string;
  accounts: CompiledAccountMeta[];
  protocol_config: string;
  vault_authority: string;
  policy_attestation: string;
  split_plan: string;
  split_plan_digest: number[];
  receipt: string;
  compute_budget_policy: ComputeBudgetPolicy;
};

export type CompiledSettlementResponse = CompiledSettlementInstruction & {
  intent_id: string | null;
  request_digest: number[];
  instruction_digest: number[];
  route_id: string | null;
};

export type CompiledSplitSettlementResponse = CompiledSplitSettlementInstruction & {
  intent_id: string | null;
  request_digest: number[];
  instruction_digest: number[];
  route_id: string;
  split_plan_id: string;
};

export type CompiledSettlementArtifact =
  | CompiledSettlementResponse
  | CompiledSplitSettlementResponse;

function parseCompiledAccounts(value: unknown): CompiledAccountMeta[] {
  if (!Array.isArray(value)
      || value.length < SETTLEMENT_INSTRUCTION_ACCOUNT_COUNT
      || value.length > SETTLEMENT_INSTRUCTION_ACCOUNT_COUNT + 8) {
    throw new Error("The API returned an invalid or unbounded settlement account list.");
  }
  return value.map((candidate, index) => {
    const account = recordValue(candidate, `The API returned an invalid settlement account at position ${index}.`);
    if (typeof account.is_signer !== "boolean" || typeof account.is_writable !== "boolean") {
      throw new Error(`The API returned invalid settlement account flags at position ${index}.`);
    }
    return {
      pubkey: requirePublicKey(account.pubkey as string, `settlement account ${index}`).toBase58(),
      is_signer: account.is_signer,
      is_writable: account.is_writable,
    };
  });
}

function parseCompiledInstructionFields(record: Record<string, unknown>): CompiledSettlementInstruction {
  if (typeof record.status !== "string"
      || typeof record.program_id !== "string"
      || typeof record.instruction_data_base64 !== "string") {
    throw new Error("The API returned an invalid compiled settlement instruction envelope.");
  }
  const computeBudget = recordValue(
    record.compute_budget_policy,
    "The API returned an invalid compute-budget policy.",
  );
  const policyVersion = computeBudget.policy_version;
  const computeUnitLimit = computeBudget.compute_unit_limit;
  const microLamports = computeBudget.micro_lamports_per_compute_unit;
  if (typeof policyVersion !== "number"
      || !Number.isSafeInteger(policyVersion)
      || policyVersion !== 1
      || typeof computeUnitLimit !== "number"
      || !Number.isSafeInteger(computeUnitLimit)
      || computeUnitLimit <= 0
      || computeUnitLimit > 1_400_000
      || typeof microLamports !== "number"
      || !Number.isSafeInteger(microLamports)
      || microLamports < 0
      || microLamports > 10_000_000) {
    throw new Error("The API returned an unsupported or unsafe compute-budget policy.");
  }
  const instruction: CompiledSettlementInstruction = {
    status: record.status,
    program_id: record.program_id,
    instruction_data_base64: record.instruction_data_base64,
    accounts: parseCompiledAccounts(record.accounts),
    protocol_config: requirePublicKey(record.protocol_config as string, "protocol PDA").toBase58(),
    vault_authority: requirePublicKey(record.vault_authority as string, "vault authority PDA").toBase58(),
    policy_attestation: requirePublicKey(record.policy_attestation as string, "policy attestation PDA").toBase58(),
    receipt: requirePublicKey(record.receipt as string, "receipt PDA").toBase58(),
    compute_budget_policy: {
      policy_version: policyVersion,
      compute_unit_limit: computeUnitLimit,
      micro_lamports_per_compute_unit: microLamports,
    },
  };
  if (instruction.status !== "prepared_unsubmitted" || instruction.program_id !== SETTLEMENT_PROGRAM_ID) {
    throw new Error("The API returned a settlement instruction outside the prepared protocol boundary.");
  }
  decodeBase64(instruction.instruction_data_base64);
  return instruction;
}

export function parseCompiledSettlementArtifact(value: unknown): CompiledSettlementArtifact {
  const record = recordValue(value, "The API returned an invalid compiled settlement artifact.");
  const intentId = parseUuidV7(record.intent_id, "settlement intent ID");
  const routeId = record.route_id === null ? null : parseUuidV7(record.route_id, "settlement route ID");
  const requestDigest = parseDigestBytes(record.request_digest, "request");
  const instructionDigest = parseDigestBytes(record.instruction_digest, "instruction");
  const base = parseCompiledInstructionFields(record);
  const splitFieldsPresent = ["split_plan", "split_plan_digest", "split_plan_id"]
    .some((field) => Object.prototype.hasOwnProperty.call(record, field));
  if (!splitFieldsPresent) {
    const artifact: CompiledSettlementResponse = {
      ...base,
      intent_id: intentId,
      request_digest: requestDigest,
      instruction_digest: instructionDigest,
      route_id: routeId,
    };
    validateCompiledSettlementInstruction(artifact, new PublicKey(artifact.accounts[0].pubkey));
    return artifact;
  }
  if (routeId === null
      || typeof record.split_plan_id !== "string"
      || !Array.isArray(record.split_plan_digest)) {
    throw new Error("The API returned an incomplete Split Plan settlement artifact.");
  }
  const splitPlan: CompiledSplitSettlementResponse = {
    ...base,
    intent_id: intentId,
    request_digest: requestDigest,
    instruction_digest: instructionDigest,
    route_id: routeId,
    split_plan_id: parseUuidV7(record.split_plan_id, "Split Plan ID"),
    split_plan: requirePublicKey(record.split_plan as string, "Split Plan PDA").toBase58(),
    split_plan_digest: parseDigestBytes(record.split_plan_digest, "Split Plan"),
  };
  validateCompiledSettlementInstruction(splitPlan, new PublicKey(splitPlan.accounts[0].pubkey));
  return splitPlan;
}

const RECORD_ADJUSTMENT_DISCRIMINATOR_BASE64 = "UEIK4GeoFyA=";
const RECORD_ADJUSTMENT_DATA_BYTES = 172;
const RECORD_ADJUSTMENT_ACCOUNT_COUNT = 10;

function parseCompiledAdjustmentAccounts(value: unknown): CompiledSettlementAdjustmentInstruction["accounts"] {
  if (!Array.isArray(value) || value.length !== RECORD_ADJUSTMENT_ACCOUNT_COUNT) {
    throw new Error("The API returned an invalid record-adjustment account list.");
  }
  return value.map((candidate, index) => {
    const account = recordValue(candidate, `The API returned an invalid adjustment account at position ${index}.`);
    if (typeof account.is_signer !== "boolean" || typeof account.is_writable !== "boolean") {
      throw new Error(`The API returned invalid adjustment account flags at position ${index}.`);
    }
    return {
      pubkey: requirePublicKey(account.pubkey as string, `adjustment account ${index}`).toBase58(),
      is_signer: account.is_signer,
      is_writable: account.is_writable,
    };
  });
}

export function validateCompiledSettlementAdjustmentInstruction(
  instruction: CompiledSettlementAdjustmentInstruction,
  approver: PublicKey,
): void {
  if (instruction.status !== "prepared_unsubmitted") {
    throw new Error("Only a prepared, unsubmitted adjustment may be signed.");
  }
  if (instruction.program_id !== SETTLEMENT_PROGRAM_ID) {
    throw new Error("Adjustment program identity does not match the configured protocol.");
  }
  if (instruction.accounts.length !== RECORD_ADJUSTMENT_ACCOUNT_COUNT) {
    throw new Error("Record-adjustment account count does not match the protocol contract.");
  }
  const expectedFlags: Array<[boolean, boolean]> = [
    [true, true], [false, false], [false, false], [false, false], [false, false],
    [false, false], [false, false], [false, false], [false, true], [false, false],
  ];
  const accountKeys = instruction.accounts.map((account, index) => {
    const key = requirePublicKey(account.pubkey, `adjustment account ${index}`);
    const [isSigner, isWritable] = expectedFlags[index];
    if (account.is_signer !== isSigner || account.is_writable !== isWritable) {
      throw new Error(`Adjustment account flags differ at position ${index}.`);
    }
    return key;
  });
  if (!accountKeys[0].equals(approver)) {
    throw new Error("Connected wallet must match the compiled approver signer.");
  }
  if (new Set(accountKeys.map((key) => key.toBase58())).size !== accountKeys.length) {
    throw new Error("Record-adjustment accounts must be distinct.");
  }
  if (accountKeys[4].toBase58() !== SPL_TOKEN_PROGRAM_ID
      && accountKeys[4].toBase58() !== TOKEN_2022_PROGRAM_ID) {
    throw new Error("Record-adjustment token program is not supported.");
  }
  if (accountKeys[9].toBase58() !== SYSTEM_PROGRAM_ID) {
    throw new Error("Record-adjustment System Program account is not canonical.");
  }
  const data = decodeBase64(instruction.instruction_data_base64);
  if (data.length !== RECORD_ADJUSTMENT_DATA_BYTES
      || Buffer.from(data.subarray(0, 8)).toString("base64") !== RECORD_ADJUSTMENT_DISCRIMINATOR_BASE64) {
    throw new Error("Record-adjustment instruction data is not canonical.");
  }
  const adjustmentNonce = parseCanonicalU64(instruction.adjustment_nonce, "Record-adjustment nonce");
  if (adjustmentNonce === "0") {
    throw new Error("Record-adjustment nonce must be positive.");
  }
  requirePublicKey(instruction.adjustment, "adjustment PDA");
  requirePublicKey(instruction.adjustment_registry, "adjustment registry PDA");
  requirePublicKey(instruction.original_receipt, "original receipt PDA");
}

export function parseCompiledSettlementAdjustmentArtifact(value: unknown): CompiledSettlementAdjustmentArtifact {
  const record = recordValue(value, "The API returned an invalid adjustment compile artifact.");
  if (record.status !== "prepared_unsubmitted") throw new Error("The adjustment compile artifact is not prepared.");
  const instructionRecord = recordValue(record.instruction, "The API returned an invalid adjustment instruction.");
  const instruction: CompiledSettlementAdjustmentInstruction = {
    status: "prepared_unsubmitted",
    program_id: requirePublicKey(instructionRecord.program_id as string, "adjustment program").toBase58(),
    instruction_data_base64: typeof instructionRecord.instruction_data_base64 === "string"
      ? instructionRecord.instruction_data_base64 : "",
    accounts: parseCompiledAdjustmentAccounts(instructionRecord.accounts),
    adjustment: requirePublicKey(instructionRecord.adjustment as string, "adjustment PDA").toBase58(),
    adjustment_registry: requirePublicKey(instructionRecord.adjustment_registry as string, "adjustment registry PDA").toBase58(),
    original_receipt: requirePublicKey(instructionRecord.original_receipt as string, "original receipt PDA").toBase58(),
    adjustment_nonce: parseCanonicalU64(instructionRecord.adjustment_nonce, "Record-adjustment nonce"),
  };
  if (record.registry_commitment !== "finalized") {
    throw new Error("The API returned a non-finalized registry commitment.");
  }
  const registrySlot = parseCanonicalU64(record.registry_slot, "Finalized registry slot");
  if (registrySlot === "0") throw new Error("The API returned an invalid finalized registry slot.");
  const artifact: CompiledSettlementAdjustmentArtifact = {
    status: "prepared_unsubmitted",
    intent_id: parseUuidV7(record.intent_id, "adjustment evidence intent ID"),
    chain_intent_id: parseUuidV7(record.chain_intent_id, "adjustment chain intent ID"),
    request_digest: parseDigestBytes(record.request_digest, "adjustment request"),
    instruction_digest: parseDigestBytes(record.instruction_digest, "adjustment instruction"),
    registry: requirePublicKey(record.registry as string, "adjustment registry").toBase58(),
    registry_slot: registrySlot,
    registry_commitment: "finalized",
    registry_account_digest: parseDigestBytes(record.registry_account_digest, "registry account"),
    instruction,
  };
  validateCompiledSettlementAdjustmentInstruction(
    artifact.instruction,
    new PublicKey(artifact.instruction.accounts[0].pubkey),
  );
  if (artifact.registry !== artifact.instruction.adjustment_registry) {
    throw new Error("Registry evidence does not match the compiled instruction.");
  }
  return artifact;
}

export type SettlementIntentStatus =
  | "prepared_unsubmitted"
  | "signed"
  | "submitting"
  | "submitted"
  | "processed"
  | "confirmed"
  | "finalized"
  | "rejected"
  | "unknown";

export type SettlementSubmitResponse = {
  intent_id: string;
  status: SettlementIntentStatus;
  signature: string | null;
  provider_index: number | null;
};

const ADJUSTMENT_CHAIN_INTENT_STATUSES: readonly AdjustmentChainIntentStatus[] = [
  "prepared_unsubmitted", "signed", "submitting", "submitted", "processed",
  "confirmed", "finalized", "rejected", "unknown",
];

export function parseSettlementAdjustmentChainIntentResponse(value: unknown): AdjustmentChainIntentResponse {
  const record = recordValue(value, "The API returned an invalid adjustment chain-intent response.");
  const chainIntentId = parseUuidV7(record.chain_intent_id, "adjustment chain intent ID");
  if (typeof record.status !== "string"
      || !ADJUSTMENT_CHAIN_INTENT_STATUSES.includes(record.status as AdjustmentChainIntentStatus)) {
    throw new Error("The API returned an invalid adjustment chain-intent status.");
  }
  const signature = record.signature;
  if (signature !== null
      && (typeof signature !== "string" || signature.length === 0 || signature.length > 128)) {
    throw new Error("The API returned an invalid adjustment chain signature.");
  }
  if ((record.status === "submitting" || record.status === "submitted"
      || record.status === "processed" || record.status === "confirmed"
      || record.status === "finalized" || record.status === "unknown")
      && signature === null) {
    throw new Error("The API returned a chain lifecycle state without its bound signature.");
  }
  const providerIndex = record.provider_index;
  if (providerIndex !== null
      && (typeof providerIndex !== "number"
        || !Number.isSafeInteger(providerIndex)
        || providerIndex < 0)) {
    throw new Error("The API returned an invalid adjustment provider index.");
  }
  const finalizedEvidenceValue = record.finalized_evidence;
  let finalizedEvidence: AdjustmentChainIntentResponse["finalized_evidence"] = null;
  if (record.status === "finalized") {
    const evidence = recordValue(finalizedEvidenceValue, "The API returned an invalid finalized adjustment proof.");
    if (evidence.event_type !== "SettlementAdjustmentRecorded"
        || evidence.event_version !== "v1"
        || evidence.commitment !== "finalized"
        || evidence.signature !== signature) {
      throw new Error("The API returned a finalized adjustment proof that is not bound to the intent.");
    }
    if (!Array.isArray(evidence.event_discriminator)
        || evidence.event_discriminator.length !== ADJUSTMENT_RECORDED_EVENT_DISCRIMINATOR.length
        || evidence.event_discriminator.some((byte, index) => byte !== ADJUSTMENT_RECORDED_EVENT_DISCRIMINATOR[index])) {
      throw new Error("The API returned an invalid finalized adjustment event discriminator.");
    }
    const digestFields = [
      "adjustment", "original_receipt", "merchant", "mint", "adjustment_digest",
      "adjustment_evidence_digest", "event_evidence_digest", "approver",
    ] as const;
    for (const field of digestFields) validateByteArray(evidence[field], `finalized proof ${field}`, true);
    const slot = parseCanonicalU64(evidence.slot, "finalized proof slot");
    const nonce = parseCanonicalU64(evidence.nonce, "finalized proof nonce");
    const magnitudes = [
      ["gross", evidence.gross_sign, evidence.gross_magnitude],
      ["tax", evidence.tax_sign, evidence.tax_magnitude],
      ["net", evidence.net_sign, evidence.net_magnitude],
    ] as const;
    const eventIndex = proofNumber(evidence.event_index, "finalized proof event index", 0, 32767);
    const kind = proofNumber(evidence.kind, "finalized proof kind", 1, 5);
    const grossSign = proofNumber(evidence.gross_sign, "finalized proof gross sign", 0, 1);
    const taxSign = proofNumber(evidence.tax_sign, "finalized proof tax sign", 0, 1);
    const netSign = proofNumber(evidence.net_sign, "finalized proof net sign", 0, 1);
    for (const [label, sign, magnitude] of [
      ["gross", grossSign, evidence.gross_magnitude],
      ["tax", taxSign, evidence.tax_magnitude],
      ["net", netSign, evidence.net_magnitude],
    ] as const) {
      parseCanonicalU64(magnitude, `finalized proof ${label} magnitude`);
    }
    finalizedEvidence = {
      event_type: "SettlementAdjustmentRecorded",
      event_version: "v1",
      event_discriminator: [...ADJUSTMENT_RECORDED_EVENT_DISCRIMINATOR],
      commitment: "finalized",
      signature: signature as string,
      slot,
      event_index: eventIndex,
      adjustment: validateByteArray(evidence.adjustment, "finalized proof adjustment", true),
      original_receipt: validateByteArray(evidence.original_receipt, "finalized proof original_receipt", true),
      merchant: validateByteArray(evidence.merchant, "finalized proof merchant", true),
      mint: validateByteArray(evidence.mint, "finalized proof mint", true),
      nonce,
      kind,
      gross_sign: grossSign,
      gross_magnitude: parseCanonicalU64(evidence.gross_magnitude, "finalized proof gross magnitude"),
      tax_sign: taxSign,
      tax_magnitude: parseCanonicalU64(evidence.tax_magnitude, "finalized proof tax magnitude"),
      net_sign: netSign,
      net_magnitude: parseCanonicalU64(evidence.net_magnitude, "finalized proof net magnitude"),
      adjustment_digest: validateByteArray(evidence.adjustment_digest, "finalized proof adjustment_digest", true),
      adjustment_evidence_digest: validateByteArray(evidence.adjustment_evidence_digest, "finalized proof adjustment_evidence_digest", true),
      event_evidence_digest: validateByteArray(evidence.event_evidence_digest, "finalized proof event_evidence_digest", true),
      approver: validateByteArray(evidence.approver, "finalized proof approver", true),
    };
  } else if (finalizedEvidenceValue !== null && finalizedEvidenceValue !== undefined) {
    throw new Error("The API returned finalized proof for a non-finalized adjustment state.");
  }
  return {
    chain_intent_id: chainIntentId,
    status: record.status as AdjustmentChainIntentStatus,
    signature: signature as string | null,
    provider_index: providerIndex as number | null,
    finalized_evidence: finalizedEvidence,
  };
}

export function settlementIntentAllowsRetry(status: SettlementIntentStatus): boolean {
  return status === "prepared_unsubmitted" || status === "signed";
}

type InjectedWallet = {
  isConnected?: boolean;
  publicKey?: PublicKey | null;
  connect: () => Promise<{ publicKey: PublicKey }>;
  signTransaction: (transaction: Transaction) => Promise<Transaction>;
};

const SETTLEMENT_INTENT_STATUSES: readonly SettlementIntentStatus[] = [
  "prepared_unsubmitted",
  "signed",
  "submitting",
  "submitted",
  "processed",
  "confirmed",
  "finalized",
  "rejected",
  "unknown",
];
export function parseSettlementStatusResponse(value: unknown): SettlementSubmitResponse {
  if (!value || typeof value !== "object") {
    throw new Error("The API returned an invalid settlement intent response.");
  }
  const record = value as Record<string, unknown>;
  const intentId = typeof record.intent_id === "string" ? record.intent_id.toLowerCase() : "";
  if (!UUIDV7_PATTERN.test(intentId)) {
    throw new Error("The API returned a settlement intent ID that is not UUIDv7.");
  }
  const status = record.status;
  if (typeof status !== "string"
      || !SETTLEMENT_INTENT_STATUSES.includes(status as SettlementIntentStatus)) {
    throw new Error("The API returned an invalid settlement intent status.");
  }
  const signature = record.signature;
  if (signature !== null
      && (typeof signature !== "string" || signature.length === 0 || signature.length > 128)) {
    throw new Error("The API returned an invalid settlement signature.");
  }
  if (status === "submitting" && signature === null) {
    throw new Error("The API returned a submitting settlement without its bound signature.");
  }
  const providerIndex = record.provider_index;
  if (providerIndex !== null
      && (typeof providerIndex !== "number"
        || !Number.isSafeInteger(providerIndex)
        || providerIndex < 0)) {
    throw new Error("The API returned an invalid provider index.");
  }
  return {
    intent_id: intentId,
    status: status as SettlementIntentStatus,
    signature: signature as string | null,
    provider_index: providerIndex as number | null,
  };
}

declare global {
  interface Window {
    solana?: InjectedWallet;
  }
}

export function injectedWallet(): InjectedWallet | null {
  return typeof window !== "undefined" && window.solana
    ? window.solana
    : null;
}

export async function connectInjectedWallet(): Promise<PublicKey> {
  const wallet = injectedWallet();
  if (!wallet) throw new Error("No compatible injected Solana wallet was detected.");
  const response = await wallet.connect();
  return response.publicKey;
}

function decodeBase64(value: string): Uint8Array {
  if (!value.trim()) throw new Error("Settlement instruction data is empty.");
  const bytes = Buffer.from(value, "base64");
  if (!bytes.length || bytes.toString("base64") !== value) {
    throw new Error("Settlement instruction data must use canonical base64.");
  }
  return Uint8Array.from(bytes);
}

function requirePublicKey(value: string, field: string): PublicKey {
  try {
    return new PublicKey(value);
  } catch {
    throw new Error(`Settlement instruction contains an invalid ${field}.`);
  }
}

/**
 * Applies a browser-side defense-in-depth check before a wallet is shown the
 * signing prompt. The API repeats the cryptographic check against the signed
 * message; this function never replaces that authority boundary.
 */
export function validateCompiledSettlementInstruction(
  instruction: CompiledSettlementInstruction | CompiledSplitSettlementInstruction,
  payer: PublicKey,
): void {
  if (instruction.status !== "prepared_unsubmitted") {
    throw new Error("Only a prepared, unsubmitted settlement may be signed.");
  }
  if (instruction.program_id !== SETTLEMENT_PROGRAM_ID) {
    throw new Error("Settlement program identity does not match the configured protocol.");
  }
  const isSplit = "split_plan" in instruction;
  const expectedAccountCount = isSplit
    ? SETTLEMENT_INSTRUCTION_ACCOUNT_COUNT + 1
    : SETTLEMENT_INSTRUCTION_ACCOUNT_COUNT;
  const invalidAccountCount = isSplit
    ? instruction.accounts.length < expectedAccountCount
      || instruction.accounts.length > expectedAccountCount + 8
    : instruction.accounts.length !== expectedAccountCount;
  if (invalidAccountCount) {
    throw new Error("Settlement account count does not match the protocol contract.");
  }

  const accountKeys = instruction.accounts.map((account, index) => {
    const key = requirePublicKey(account.pubkey, `account ${index}`);
    return { account, key };
  });
  const expectedFlags: Array<[boolean, boolean]> = isSplit
    ? [
        [true, true],
        [false, false],
        [false, false],
        [false, true],
        [false, false],
        [false, true],
        [false, false],
        [false, false],
        [false, false],
        [false, true],
        [false, false],
        [false, false],
      ]
    : [
        [true, true],
        [false, false],
        [false, false],
        [false, true],
        [false, false],
        [false, true],
        [false, false],
        [false, false],
        [false, true],
        [false, true],
        [false, false],
        [false, false],
      ];
  if (isSplit) {
    const recipientCount = instruction.accounts.length - expectedFlags.length;
    for (let index = 0; index < recipientCount; index += 1) {
      expectedFlags.push([false, true]);
    }
  }
  accountKeys.forEach(({ account }, index) => {
    const [isSigner, isWritable] = expectedFlags[index];
    if (account.is_signer !== isSigner || account.is_writable !== isWritable) {
      throw new Error(`Settlement account flags differ at position ${index}.`);
    }
  });
  if (!accountKeys[0].key.equals(payer)) {
    throw new Error("Settlement payer does not match the first account meta.");
  }
  if (!accountKeys[1].key.equals(requirePublicKey(instruction.protocol_config, "protocol PDA"))) {
    throw new Error("Protocol PDA does not match the canonical account order.");
  }
  if (!accountKeys[6].key.equals(requirePublicKey(instruction.vault_authority, "vault authority PDA"))) {
    throw new Error("Vault authority PDA does not match the canonical account order.");
  }
  if (!accountKeys[7].key.equals(requirePublicKey(instruction.policy_attestation, "policy attestation PDA"))) {
    throw new Error("Policy attestation PDA does not match the canonical account order.");
  }
  const receiptIndex = 9;
  if (!accountKeys[receiptIndex].key.equals(requirePublicKey(instruction.receipt, "receipt PDA"))) {
    throw new Error("Receipt PDA does not match the canonical account order.");
  }
  if (isSplit && !accountKeys[8].key.equals(requirePublicKey(instruction.split_plan, "Split Plan PDA"))) {
    throw new Error("Split Plan PDA does not match the canonical account order.");
  }
  const tokenProgramIndex = isSplit ? 10 : 10;
  if (accountKeys[tokenProgramIndex].key.toBase58() !== SPL_TOKEN_PROGRAM_ID
      && accountKeys[tokenProgramIndex].key.toBase58() !== TOKEN_2022_PROGRAM_ID) {
    throw new Error("Settlement token program is not supported.");
  }
  const systemProgramIndex = isSplit ? 11 : 11;
  if (accountKeys[systemProgramIndex].key.toBase58() !== SYSTEM_PROGRAM_ID) {
    throw new Error("Settlement System Program account is not canonical.");
  }
  const tokenIndexes = isSplit ? [3, 5, ...Array.from({ length: instruction.accounts.length - 12 }, (_, index) => index + 12)] : [3, 5, 8];
  if (new Set(tokenIndexes.map((index) => accountKeys[index].key.toBase58())).size !== tokenIndexes.length) {
    throw new Error("Settlement token accounts must be distinct.");
  }

  const instructionData = decodeBase64(instruction.instruction_data_base64);
  const expectedDataLength = isSplit ? 88 : 80;
  if (instructionData.length !== expectedDataLength) {
    throw new Error("Settlement instruction data has an invalid length.");
  }
  const discriminator = Buffer.from(instructionData.subarray(0, 8)).toString("base64");
  if (discriminator !== (isSplit ? SPLIT_SETTLE_DISCRIMINATOR_BASE64 : SETTLE_DISCRIMINATOR_BASE64)) {
    throw new Error("Settlement instruction discriminator is not canonical.");
  }
}

export function parsePolicyDigest(value: string): number[] {
  const normalized = value.trim().replace(/^0x/, "");
  if (!/^[0-9a-fA-F]{64}$/.test(normalized)) {
    throw new Error("Policy digest must be exactly 32 bytes represented by 64 hexadecimal characters.");
  }
  return Array.from({ length: 32 }, (_, index) => Number.parseInt(normalized.slice(index * 2, index * 2 + 2), 16));
}

export function buildSettlementTransaction(
  instruction: CompiledSettlementInstruction | CompiledSplitSettlementInstruction,
  payer: PublicKey,
  connection: Connection,
): Promise<Transaction> {
  validateCompiledSettlementInstruction(instruction, payer);
  return connection.getLatestBlockhash("confirmed").then(({ blockhash }) => {
    const transactionInstruction = new TransactionInstruction({
      programId: new PublicKey(instruction.program_id),
      data: Buffer.from(decodeBase64(instruction.instruction_data_base64)),
      keys: instruction.accounts.map((account) => ({
        pubkey: new PublicKey(account.pubkey),
        isSigner: account.is_signer,
        isWritable: account.is_writable,
      })),
    });
    const computeBudgetInstructions = [
      ComputeBudgetProgram.setComputeUnitLimit({ units: instruction.compute_budget_policy.compute_unit_limit }),
      ComputeBudgetProgram.setComputeUnitPrice({
        microLamports: instruction.compute_budget_policy.micro_lamports_per_compute_unit,
      }),
    ];
    return new Transaction({
      feePayer: payer,
      recentBlockhash: blockhash,
    }).add(...computeBudgetInstructions, transactionInstruction);
  });
}

export async function signAndEncodeSettlement(
  instruction: CompiledSettlementInstruction | CompiledSplitSettlementInstruction,
  payer: PublicKey,
  rpcUrl: string,
): Promise<string> {
  const wallet = injectedWallet();
  if (!wallet) throw new Error("Connect a compatible Solana wallet before signing.");
  if (!rpcUrl.trim()) throw new Error("A VITE_SOLANA_RPC_URL is required before wallet signing is enabled.");
  const connection = new Connection(rpcUrl, "confirmed");
  const unsigned = await buildSettlementTransaction(instruction, payer, connection);
  const signed = await wallet.signTransaction(unsigned);
  const serialized = signed.serialize({ requireAllSignatures: true, verifySignatures: true });
  return btoa(String.fromCharCode(...serialized));
}

export function buildSettlementAdjustmentTransaction(
  instruction: CompiledSettlementAdjustmentInstruction,
  approver: PublicKey,
  connection: Connection,
): Promise<Transaction> {
  validateCompiledSettlementAdjustmentInstruction(instruction, approver);
  return connection.getLatestBlockhash("confirmed").then(({ blockhash }) => {
    const transactionInstruction = new TransactionInstruction({
      programId: new PublicKey(instruction.program_id),
      data: Buffer.from(decodeBase64(instruction.instruction_data_base64)),
      keys: instruction.accounts.map((account) => ({
        pubkey: new PublicKey(account.pubkey),
        isSigner: account.is_signer,
        isWritable: account.is_writable,
      })),
    });
    return new Transaction({
      feePayer: approver,
      recentBlockhash: blockhash,
    }).add(transactionInstruction);
  });
}

export async function signAndEncodeSettlementAdjustment(
  instruction: CompiledSettlementAdjustmentInstruction,
  approver: PublicKey,
  rpcUrl: string,
): Promise<string> {
  const wallet = injectedWallet();
  if (!wallet) throw new Error("Connect a compatible Solana wallet before signing the adjustment.");
  if (!rpcUrl.trim()) throw new Error("A VITE_SOLANA_RPC_URL is required before adjustment signing is enabled.");
  const connection = new Connection(rpcUrl, "confirmed");
  const unsigned = await buildSettlementAdjustmentTransaction(instruction, approver, connection);
  const signed = await wallet.signTransaction(unsigned);
  const serialized = signed.serialize({ requireAllSignatures: true, verifySignatures: true });
  return btoa(String.fromCharCode(...serialized));
}
