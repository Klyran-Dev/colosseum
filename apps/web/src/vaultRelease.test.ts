import { describe, expect, it } from "vitest";
import {
  parseVaultReleaseClaim,
  parseVaultReleaseTransitionResponse,
} from "./vaultRelease";

const claimId = "01992f1b-4a70-7e30-8ea2-b6e37a9d9d21";
const merchantId = "01992f1b-4a70-7e30-8ea2-b6e37a9d9d22";
const publicKey = "11111111111111111111111111111111";

function validClaim(overrides: Record<string, unknown> = {}): Record<string, unknown> {
  return {
    id: claimId,
    merchant_id: merchantId,
    idempotency_key: "release-01J8Q4J0M9M2G7YB8F5N3D6K9R",
    request_digest: Array(32).fill(1),
    mint: publicKey,
    token_program: publicKey,
    tax_vault: publicKey,
    destination: publicKey,
    claim_pda: null,
    nonce: "42",
    amount_atomic: "18446744073709551615",
    reference_digest: Array(32).fill(2),
    status: "proposed",
    proposer_subject: "proposer-subject",
    approver_subject: null,
    execute_after_unix: 1_800_000_000,
    expires_at_unix: 1_800_003_600,
    transaction_signature: null,
    slot: "9007199254740993",
    last_error_code: null,
    ...overrides,
  };
}

describe("strict vault-release API responses", () => {
  it("preserves canonical u64 strings beyond JavaScript safe integer range", () => {
    const parsed = parseVaultReleaseClaim(validClaim());
    expect(parsed.id).toBe(claimId);
    expect(parsed.nonce).toBe("42");
    expect(parsed.amount_atomic).toBe("18446744073709551615");
    expect(parsed.slot).toBe("9007199254740993");
  });

  it("rejects numeric u64 fields, malformed identity, and invalid lifecycle state", () => {
    expect(() => parseVaultReleaseClaim(validClaim({ amount_atomic: 100 }))).toThrow(/amount/);
    expect(() => parseVaultReleaseClaim(validClaim({ nonce: 42 }))).toThrow(/nonce/);
    expect(() => parseVaultReleaseClaim(validClaim({ slot: 1 }))).toThrow(/slot/);
    expect(() => parseVaultReleaseClaim(validClaim({ id: "01992f1b-4a70-6e30-8ea2-b6e37a9d9d21" }))).toThrow(/UUIDv7/);
    expect(() => parseVaultReleaseClaim(validClaim({ status: "finalized" }))).toThrow(/status/);
  });

  it("rejects invalid digest, public-key, and time-window evidence", () => {
    expect(() => parseVaultReleaseClaim(validClaim({ reference_digest: Array(31).fill(2) }))).toThrow(/reference/);
    expect(() => parseVaultReleaseClaim(validClaim({ destination: "not-a-public-key" }))).toThrow(/destination/);
    expect(() => parseVaultReleaseClaim(validClaim({ expires_at_unix: 1_800_000_000 }))).toThrow(/window/);
  });

  it("requires a server-owned transition envelope", () => {
    const parsed = parseVaultReleaseTransitionResponse({
      status: "transitioned",
      claim_id: claimId,
      from_status: "proposed",
      to_status: "approved",
      approver_subject: "approver-subject",
    });
    expect(parsed.to_status).toBe("approved");
    expect(() => parseVaultReleaseTransitionResponse({
      status: "transitioned",
      claim_id: claimId,
      from_status: "proposed",
      to_status: "finalized",
      approver_subject: null,
    })).toThrow(/status/);
  });
});
