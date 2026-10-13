import { describe, expect, it } from "vitest";
import authorizationCatalog from "../../../contracts/authorization-capabilities-v1.json";
import {
  AuthorizationCapabilitiesError,
  WORKSPACE_POLICIES,
  authorizationSnapshotForToken,
  authorizationSnapshotRefreshDelayMs,
  evaluateUiAccess,
  parseAuthorizationCapabilities,
  policiesForGroup,
  workspaceRequiresAuthentication,
} from "./authorization";

function completeCapabilities(overrides: Record<string, unknown> = {}) {
  return Object.fromEntries(WORKSPACE_POLICIES.map((policy) => [
    policy.id,
    overrides[policy.id] ?? { decision: "allow", reason_code: "scope_and_role_satisfied" },
  ]));
}

describe("browser authorization affordance contract", () => {
  it("keeps the public simulator distinct from protected workspaces", () => {
    expect(workspaceRequiresAuthentication("preview")).toBe(false);
    expect(workspaceRequiresAuthentication("document-checkout")).toBe(true);
  });

  it("denies anonymous access without pretending the browser can authorize a principal", () => {
    const snapshot = authorizationSnapshotForToken("");
    const policy = WORKSPACE_POLICIES.find((item) => item.id === "settlement.wallet");
    expect(policy).toBeDefined();
    expect(evaluateUiAccess(snapshot, policy!)).toEqual({
      kind: "deny",
      reasonCode: "sign_in_required",
      reason: "Sign in to request this action.",
    });
  });

  it("keeps a supplied token indeterminate until the API returns server-evaluated claims", () => {
    const snapshot = authorizationSnapshotForToken("opaque-or-jwt");
    const policy = WORKSPACE_POLICIES.find((item) => item.id === "tax.rule.govern");
    expect(evaluateUiAccess(snapshot, policy!)).toMatchObject({ kind: "indeterminate" });
  });

  it("uses only the server-evaluated decision after a trusted snapshot exists", () => {
    const policy = WORKSPACE_POLICIES.find((item) => item.id === "tax.rule.govern");
    expect(evaluateUiAccess({
      state: "server-allowed",
      subject: "user-1",
      tenantId: "tenant-1",
      expiresAtUnix: 1_000_000_030,
      roles: ["tax_rule_reviewer"],
      scopes: ["tax:rule:read"],
      attributes: { environment: "staging", jurisdiction: "US-CA" },
      capabilities: {
        [policy!.id]: {
          kind: "allow",
          reasonCode: "scope_and_role_satisfied",
          reason: "The service confirmed this action for the current account.",
        },
      },
    }, policy!, 1_000_000_000)).toEqual({ kind: "allow", reasonCode: "scope_and_role_satisfied", reason: "The service confirmed this action for the current account." });

    expect(evaluateUiAccess({
      state: "server-allowed",
      subject: "user-1",
      tenantId: null,
      expiresAtUnix: 1_000_000_030,
      roles: ["tax_rule_reviewer"],
      scopes: ["tax:rule:read"],
      attributes: {},
      capabilities: {
        [policy!.id]: {
          kind: "deny",
          reasonCode: "role_missing",
          reason: "This action needs an approved responsibility.",
        },
      },
    }, policy!, 1_000_000_000)).toMatchObject({ kind: "deny" });

    expect(evaluateUiAccess({
      state: "server-allowed",
      subject: "user-1",
      tenantId: "tenant-1",
      expiresAtUnix: 1_000_000_030,
      roles: ["tax_rule_reviewer"],
      scopes: ["tax:rule:read"],
      attributes: { environment: "staging" },
      capabilities: {
        [policy!.id]: {
          kind: "indeterminate",
          reasonCode: "resource_context_required",
          reason: "Choose the relevant record or jurisdiction to complete the access check.",
        },
      },
    }, policy!, 1_000_000_000)).toEqual({ kind: "indeterminate", reasonCode: "resource_context_required", reason: "Choose the relevant record or jurisdiction to complete the access check." });
  });

  it("stays indeterminate when a server-allowed snapshot omits the server decision", () => {
    const policy = WORKSPACE_POLICIES.find((item) => item.id === "tax.rule.govern");
    expect(policy).toBeDefined();
    expect(evaluateUiAccess({
      state: "server-allowed",
      subject: "user-1",
      tenantId: "tenant-1",
      expiresAtUnix: 1_000_000_030,
      roles: ["tax_rule_reviewer"],
      scopes: ["tax:rule:read"],
      attributes: { environment: "staging", jurisdiction: "US-CA" },
      capabilities: {},
    }, policy!, 1_000_000_000)).toEqual({
      kind: "indeterminate",
      reasonCode: "missing_capability_decision",
      reason: "The service has not returned a decision for this action.",
    });
  });

  it("fails closed when a capability snapshot is expired at render time", () => {
    const policy = WORKSPACE_POLICIES.find((item) => item.id === "tax.preview.run");
    expect(policy).toBeDefined();
    expect(evaluateUiAccess({
      state: "server-allowed",
      subject: "user-1",
      tenantId: "tenant-1",
      expiresAtUnix: 1_000_000_030,
      roles: [],
      scopes: [],
      attributes: {},
      capabilities: {
        [policy!.id]: {
          kind: "allow",
          reasonCode: "scope_and_role_satisfied",
          reason: "The service confirmed this action for the current account.",
        },
      },
    }, policy!, 1_000_000_030)).toEqual({
      kind: "indeterminate",
      reasonCode: "access_expired",
      reason: "The access check has expired; refresh before continuing.",
    });
  });

  it("fails closed when a server-allowed snapshot has no expiry", () => {
    const policy = WORKSPACE_POLICIES.find((item) => item.id === "tax.preview.run");
    expect(policy).toBeDefined();
    expect(evaluateUiAccess({
      state: "server-allowed",
      subject: "user-1",
      tenantId: "tenant-1",
      roles: [],
      scopes: [],
      attributes: {},
      capabilities: {
        [policy!.id]: {
          kind: "allow",
          reasonCode: "scope_and_role_satisfied",
          reason: "The service confirmed this action for the current account.",
        },
      },
    }, policy!, 1_000_000_030)).toEqual({
      kind: "indeterminate",
      reasonCode: "missing_expiry",
      reason: "The access check has no expiry.",
    });
  });

  it("schedules refresh only for server-allowed snapshots at their expiry boundary", () => {
    expect(authorizationSnapshotRefreshDelayMs(authorizationSnapshotForToken("opaque"), 1_000_000_000)).toBeNull();
    expect(authorizationSnapshotRefreshDelayMs({
      state: "server-allowed",
      subject: "user-1",
      tenantId: "tenant-1",
      expiresAtUnix: 1_000_000_030,
      roles: [],
      scopes: [],
      attributes: {},
    }, 1_000_000_000)).toBe(30_000);
    expect(authorizationSnapshotRefreshDelayMs({
      state: "server-allowed",
      subject: "user-1",
      tenantId: "tenant-1",
      expiresAtUnix: 1_000_000_030,
      roles: [],
      scopes: [],
      attributes: {},
    }, 1_000_000_030)).toBe(0);
    expect(authorizationSnapshotRefreshDelayMs({
      state: "server-allowed",
      subject: "user-1",
      tenantId: "tenant-1",
      roles: [],
      scopes: [],
      attributes: {},
    }, 1_000_000_000)).toBe(0);
  });

  it("keeps the navigation catalog grouped for a SaaS operator workflow", () => {
    expect(policiesForGroup("Rules & access").map((policy) => policy.id)).toEqual(["tax.rule.govern", "tax.applicability.govern", "access.control", "identity.role.manage"]);
  });

  it("describes every production workspace and its bounded action capabilities", () => {
    expect(WORKSPACE_POLICIES).toHaveLength(22);
    expect(new Set(WORKSPACE_POLICIES.map((policy) => policy.workspace)).size).toBe(16);
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

  it("keeps the browser affordance projection identical to the canonical catalog", () => {
    expect(WORKSPACE_POLICIES.map((policy) => ({
      id: policy.id,
      label: policy.label,
      description: policy.description,
      group: policy.group,
      workspace: policy.workspace,
      risk: policy.risk,
      required_scopes: policy.requiredScopes,
      required_roles: policy.requiredRoles,
      required_attributes: policy.requiredAttributeKeys ?? [],
    }))).toEqual(authorizationCatalog.policies);
  });

  it("accepts only a bounded server-owned capability snapshot", () => {
    const snapshot = parseAuthorizationCapabilities({
      schema: "authorization-capabilities/v1",
      snapshot_id: "01900000-0000-7000-8000-000000000001",
      subject: "operator-1",
      email: "operator@example.com",
      tenant_id: "01900000-0000-4000-8000-000000000001",
      expires_at_unix: 1_000_000_030,
      roles: ["merchant_operator", "moderator"],
      capabilities: completeCapabilities({
        "tax.quote.create": { decision: "allow", reason_code: "scope_and_role_satisfied" },
        "tax.rule.govern": { decision: "indeterminate", reason_code: "resource_context_required" },
      }),
    }, 1_000_000_000);
    expect(snapshot.state).toBe("server-allowed");
    expect(snapshot.tenantId).toBe("01900000-0000-4000-8000-000000000001");
    expect(snapshot.roles).toEqual(["merchant_operator", "moderator"]);
    expect(snapshot.email).toBe("operator@example.com");
    expect(evaluateUiAccess(snapshot, WORKSPACE_POLICIES[0], 1_000_000_000)).toEqual({
      kind: "allow",
      reasonCode: "scope_and_role_satisfied",
      reason: "The service confirmed this action for the current account.",
    });
    const governancePolicy = WORKSPACE_POLICIES.find((item) => item.id === "tax.rule.govern");
    expect(governancePolicy).toBeDefined();
    expect(evaluateUiAccess(snapshot, governancePolicy!, 1_000_000_000)).toEqual({
      kind: "indeterminate",
      reasonCode: "resource_context_required",
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

  it("rejects contradictory capability decisions and reason codes", () => {
    const base = {
      schema: "authorization-capabilities/v1",
      snapshot_id: "01900000-0000-7000-8000-000000000001",
      subject: "operator-1",
      tenant_id: "01900000-0000-4000-8000-000000000001",
      expires_at_unix: 1_000_000_030,
      capabilities: completeCapabilities(),
    };
    expect(() => parseAuthorizationCapabilities({
      ...base,
      capabilities: completeCapabilities({
        "tax.preview.run": { decision: "allow", reason_code: "role_missing" },
      }),
    }, 1_000_000_000)).toThrow(/decision and reason/i);
    expect(() => parseAuthorizationCapabilities({
      ...base,
      capabilities: completeCapabilities({
        "tax.preview.run": { decision: "deny", reason_code: "scope_and_role_satisfied" },
      }),
    }, 1_000_000_000)).toThrow(/decision and reason/i);
    expect(() => parseAuthorizationCapabilities({
      ...base,
      capabilities: completeCapabilities({
        "tax.preview.run": { decision: "indeterminate", reason_code: "scope_missing" },
      }),
    }, 1_000_000_000)).toThrow(/decision and reason/i);
  });

  it("classifies capability response failures without coupling the parser to UI copy", () => {
    const base = {
      schema: "authorization-capabilities/v1",
      snapshot_id: "01900000-0000-7000-8000-000000000001",
      subject: "operator-1",
      tenant_id: "01900000-0000-4000-8000-000000000001",
      expires_at_unix: 1_000_000_030,
      capabilities: completeCapabilities(),
    };

    let expiredError: unknown;
    try {
      parseAuthorizationCapabilities({ ...base, expires_at_unix: 999 }, 1_000_000_000);
    } catch (cause) {
      expiredError = cause;
    }

    expect(expiredError).toBeInstanceOf(AuthorizationCapabilitiesError);
    expect(expiredError).toMatchObject({ code: "expired" });
    expect((expiredError as Error).message).toMatch(/expired/i);
  });
});
