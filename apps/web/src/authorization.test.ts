import { describe, expect, it } from "vitest";
import {
  WORKSPACE_POLICIES,
  authorizationSnapshotForToken,
  evaluateUiAccess,
  parseAuthorizationCapabilities,
  policiesForGroup,
} from "./authorization";

function completeCapabilities(overrides: Record<string, unknown> = {}) {
  return Object.fromEntries(WORKSPACE_POLICIES.map((policy) => [
    policy.id,
    overrides[policy.id] ?? { decision: "allow", reason_code: "scope_and_role_satisfied" },
  ]));
}

describe("browser authorization affordance contract", () => {
  it("denies anonymous access without pretending the browser can authorize a principal", () => {
    const snapshot = authorizationSnapshotForToken("");
    const policy = WORKSPACE_POLICIES.find((item) => item.id === "settlement.wallet");
    expect(policy).toBeDefined();
    expect(evaluateUiAccess(snapshot, policy!)).toEqual({
      kind: "deny",
      reason: "Sign in to request this action.",
    });
  });

  it("keeps a supplied token indeterminate until the API returns server-evaluated claims", () => {
    const snapshot = authorizationSnapshotForToken("opaque-or-jwt");
    const policy = WORKSPACE_POLICIES.find((item) => item.id === "tax.rule.govern");
    expect(evaluateUiAccess(snapshot, policy!)).toMatchObject({ kind: "indeterminate" });
  });

  it("evaluates RBAC, scope, tenant, and ABAC context after a trusted snapshot exists", () => {
    const policy = WORKSPACE_POLICIES.find((item) => item.id === "tax.rule.govern");
    expect(evaluateUiAccess({
      state: "server-allowed",
      subject: "user-1",
      tenantId: "tenant-1",
      roles: ["tax_rule_reviewer"],
      scopes: ["tax:rule:read"],
      attributes: { environment: "staging", jurisdiction: "US-CA" },
    }, policy!)).toEqual({ kind: "allow", reason: "The server-provided capability snapshot permits this action." });

    expect(evaluateUiAccess({
      state: "server-allowed",
      subject: "user-1",
      tenantId: null,
      roles: ["tax_rule_reviewer"],
      scopes: ["tax:rule:read"],
      attributes: {},
    }, policy!)).toMatchObject({ kind: "deny" });

    expect(evaluateUiAccess({
      state: "server-allowed",
      subject: "user-1",
      tenantId: "tenant-1",
      roles: ["tax_rule_reviewer"],
      scopes: ["tax:rule:read"],
      attributes: { environment: "staging" },
    }, policy!)).toEqual({ kind: "deny", reason: "Missing resource attribute: jurisdiction." });
  });

  it("keeps the navigation catalog grouped for a SaaS operator workflow", () => {
    expect(policiesForGroup("Rules & access").map((policy) => policy.id)).toEqual(["tax.rule.govern", "access.control"]);
  });

  it("describes every production workspace and its bounded action capabilities", () => {
    expect(WORKSPACE_POLICIES).toHaveLength(18);
    expect(new Set(WORKSPACE_POLICIES.map((policy) => policy.workspace)).size).toBe(15);
    expect(WORKSPACE_POLICIES.map((policy) => policy.id)).toEqual(expect.arrayContaining([
      "tax.document.quote.create",
      "settlement.document.wallet",
      "vault.reconciliation",
      "settlement.adjustment",
      "refund.claim.create",
      "refund.claim.cancel",
      "refund.payment.release",
    ]));
  });

  it("accepts only a bounded server-owned capability snapshot", () => {
    const snapshot = parseAuthorizationCapabilities({
      schema: "authorization-capabilities/v1",
      snapshot_id: "01900000-0000-7000-8000-000000000001",
      subject: "operator-1",
      tenant_id: "01900000-0000-4000-8000-000000000001",
      expires_at_unix: 1_000_000_030,
      capabilities: completeCapabilities({
        "tax.quote.create": { decision: "allow", reason_code: "scope_and_role_satisfied" },
        "tax.rule.govern": { decision: "indeterminate", reason_code: "resource_context_required" },
      }),
    }, 1_000_000_000);
    expect(snapshot.state).toBe("server-allowed");
    expect(snapshot.tenantId).toBe("01900000-0000-4000-8000-000000000001");
    expect(evaluateUiAccess(snapshot, WORKSPACE_POLICIES[0])).toEqual({
      kind: "allow",
      reason: "The service confirmed this action for the current account.",
    });
    const governancePolicy = WORKSPACE_POLICIES.find((item) => item.id === "tax.rule.govern");
    expect(governancePolicy).toBeDefined();
    expect(evaluateUiAccess(snapshot, governancePolicy!)).toEqual({
      kind: "indeterminate",
      reason: "Choose the relevant record or jurisdiction to complete the access check.",
    });
  });

  it("rejects expired, unknown, or malformed capability snapshots", () => {
    const base = {
      schema: "authorization-capabilities/v1",
      snapshot_id: "01900000-0000-7000-8000-000000000001",
      subject: "operator-1",
      tenant_id: "01900000-0000-4000-8000-000000000001",
      expires_at_unix: 1_000_000_030,
      capabilities: completeCapabilities(),
    };
    expect(() => parseAuthorizationCapabilities({ ...base, expires_at_unix: 999 }, 1_000_000_000)).toThrow(/expired/i);
    expect(() => parseAuthorizationCapabilities({ ...base, schema: "authorization-capabilities/v2" }, 1_000_000_000)).toThrow(/unsupported/i);
    expect(() => parseAuthorizationCapabilities({ ...base, capabilities: completeCapabilities({ "tax.preview.run": { decision: "allow", reason_code: "unknown" } }) }, 1_000_000_000)).toThrow(/unknown access reason/i);
    expect(() => parseAuthorizationCapabilities({ ...base, capabilities: { "tax.preview.run": { decision: "allow", reason_code: "scope_and_role_satisfied" } } }, 1_000_000_000)).toThrow(/incomplete capability catalog/i);
  });
});
