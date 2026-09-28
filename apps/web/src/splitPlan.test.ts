import splitPlanVector from "../../../test-vectors/split-plan-v1.json";
import { describe, expect, it } from "vitest";
import { allocateSplit, splitPlanDigest, type SplitPlan } from "./splitPlan";

const plan = splitPlanVector.plan as SplitPlan;

describe("bounded Split Plan contract", () => {
  it("matches the shared digest vector", async () => {
    await expect(splitPlanDigest(plan)).resolves.toEqual(splitPlanVector.expected_digest);
  });

  it("matches truncation and deterministic residual vectors", () => {
    expect(allocateSplit(plan, 1n)).toEqual([1n, 0n]);
    expect(allocateSplit(plan, 1_000n)).toEqual([700n, 300n]);
    expect(allocateSplit(plan, 18_446_744_073_709_551_615n)).toEqual([
      12_912_720_851_596_686_131n,
      5_534_023_222_112_865_484n,
    ]);
  });

  it("rejects an over-bound recipient list before serialization", () => {
    const overBound = {
      ...plan,
      recipients: Array.from({ length: 9 }, (_, index) => ({
        token_account: index === 0 ? plan.recipients[0].token_account : plan.recipients[1].token_account,
        basis_points: index === 0 ? 9_992 : 1,
      })),
    };
    expect(() => allocateSplit(overBound, 100n)).toThrow(/recipient count/);
  });
});
