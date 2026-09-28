import { PublicKey } from "@solana/web3.js";

export const SPLIT_PLAN_SCHEMA = "solana-tax-protocol/split-plan/v1";
export const MAX_RECIPIENTS = 8;
export const BASIS_POINTS_DENOMINATOR = 10_000n;
export const MAX_U64 = 18_446_744_073_709_551_615n;

export type SplitRecipient = {
  token_account: string;
  basis_points: number;
};

export type SplitPlan = {
  merchant: string;
  mint: string;
  version: number;
  recipients: SplitRecipient[];
};

export function validateSplitPlan(plan: SplitPlan): void {
  if (!Number.isSafeInteger(plan.version) || plan.version <= 0) {
    throw new Error("split plan version must be positive");
  }
  if (plan.recipients.length === 0 || plan.recipients.length > MAX_RECIPIENTS) {
    throw new Error("split plan recipient count is outside the bounded limit");
  }
  new PublicKey(plan.merchant);
  new PublicKey(plan.mint);
  const addresses = new Set<string>();
  let total = 0;
  for (const recipient of plan.recipients) {
    const address = new PublicKey(recipient.token_account).toBase58();
    if (addresses.has(address)) throw new Error("split plan has duplicate recipient");
    addresses.add(address);
    if (!Number.isInteger(recipient.basis_points) || recipient.basis_points <= 0) {
      throw new Error("recipient basis points must be positive integers");
    }
    total += recipient.basis_points;
  }
  if (total !== Number(BASIS_POINTS_DENOMINATOR)) {
    throw new Error("recipient basis points must sum to 10000");
  }
}

export function canonicalSplitPlanBytes(plan: SplitPlan): Uint8Array {
  validateSplitPlan(plan);
  const merchant = new PublicKey(plan.merchant).toBytes();
  const mint = new PublicKey(plan.mint).toBytes();
  const bytes: number[] = [
    ...new TextEncoder().encode(SPLIT_PLAN_SCHEMA),
    0,
    ...merchant,
    ...mint,
    ...littleEndianU64(BigInt(plan.version)),
    plan.recipients.length,
  ];
  for (const recipient of plan.recipients) {
    bytes.push(...new PublicKey(recipient.token_account).toBytes());
    bytes.push(recipient.basis_points & 0xff, recipient.basis_points >> 8);
  }
  return new Uint8Array(bytes);
}

export async function splitPlanDigest(plan: SplitPlan): Promise<number[]> {
  const digest = await crypto.subtle.digest(
    "SHA-256",
    canonicalSplitPlanBytes(plan) as unknown as BufferSource,
  );
  return [...new Uint8Array(digest)];
}

export function allocateSplit(plan: SplitPlan, netAtomic: bigint): bigint[] {
  validateSplitPlan(plan);
  if (netAtomic < 0n || netAtomic > MAX_U64) throw new Error("net amount is outside u64");

  const amounts = plan.recipients.map((recipient) => {
    const numerator = netAtomic * BigInt(recipient.basis_points);
    return {
      amount: numerator / BASIS_POINTS_DENOMINATOR,
      remainder: numerator % BASIS_POINTS_DENOMINATOR,
    };
  });
  const floorSum = amounts.reduce((sum, value) => sum + value.amount, 0n);
  const residual = netAtomic - floorSum;
  if (residual < 0n || residual > BigInt(amounts.length)) {
    throw new Error("split allocation residual is outside the bounded limit");
  }

  const selected = new Set<number>();
  for (let count = 0n; count < residual; count += 1n) {
    let best = -1;
    for (let index = 0; index < amounts.length; index += 1) {
      if (selected.has(index)) continue;
      if (best < 0 || amounts[index].remainder > amounts[best].remainder) best = index;
      // Equal remainders retain the first (ascending plan ordinal) index.
    }
    if (best < 0) throw new Error("split allocation residual could not be assigned");
    selected.add(best);
    amounts[best].amount += 1n;
  }
  const total = amounts.reduce((sum, value) => sum + value.amount, 0n);
  if (total !== netAtomic) throw new Error("split allocation did not conserve net amount");
  return amounts.map((value) => value.amount);
}

function littleEndianU64(value: bigint): number[] {
  const bytes: number[] = [];
  for (let index = 0; index < 8; index += 1) {
    bytes.push(Number(value & 0xffn));
    value >>= 8n;
  }
  return bytes;
}
