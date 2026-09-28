import { describe, expect, it } from "vitest";
import {
  buildVaultReconciliationRunsUrl,
  buildVaultReconciliationTargetsUrl,
  parseVaultReconciliationRuns,
  parseVaultReconciliationTargetViews,
} from "./vaultReconciliation";

const target = {
  id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d12",
  merchant_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d11",
  route_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d13",
  stream_key: "mainnet/settlement/v1",
  mint: "mint-account",
  token_program: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
  tax_vault: "vault-account",
  opening_balance_atomic: "1000",
  opening_balance_slot: "20",
  opening_reference: "opening ceremony INC-42",
  receipt_tax_atomic: "250",
  release_atomic: "100",
};

const run = {
  id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d14",
  status: "matched",
  opening_balance_atomic: "1000",
  receipt_tax_atomic: "250",
  release_atomic: "100",
  expected_balance_atomic: "1150",
  observed_balance_atomic: "1150",
  observed_slot: "99",
  error_code: null,
  created_at: "2026-09-27T00:00:00.000Z",
};

describe("vault reconciliation operator contract", () => {
  it("parses exact server-owned target and run evidence", () => {
    const parsed = parseVaultReconciliationTargetViews([{ target, latest_run: run }]);
    expect(parsed[0].target.opening_balance_atomic).toBe("1000");
    expect(parsed[0].latest_run?.observed_slot).toBe("99");
  });

  it("rejects numeric financial values and fabricated statuses", () => {
    expect(() => parseVaultReconciliationTargetViews([{ target: { ...target, opening_balance_atomic: 1000 }, latest_run: null }])).toThrow(/decimal string/i);
    expect(() => parseVaultReconciliationRuns([{ ...run, status: "healthy" }])).toThrow(/unsupported/i);
    expect(() => parseVaultReconciliationRuns([{ ...run, observed_balance_atomic: "not-a-number" }])).toThrow(/decimal string/i);
  });

  it("bounds URLs and target identity", () => {
    expect(buildVaultReconciliationTargetsUrl("/api", 64)).toBe("/api/v1/vault/reconciliation/targets?limit=64");
    expect(buildVaultReconciliationRunsUrl("/api/", target.id, 10)).toContain(`/targets/${target.id}/runs?limit=10`);
    expect(() => buildVaultReconciliationTargetsUrl("/api", 65)).toThrow(/between 1 and 64/i);
    expect(() => buildVaultReconciliationRunsUrl("/api", "not-a-uuid")).toThrow(/UUIDv7/i);
  });
});
