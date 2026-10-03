export type AuthorizationState =
  | "anonymous"
  | "server-evaluation-required"
  | "server-denied"
  | "server-allowed";

export type ActionRisk = "read" | "write" | "financial" | "governance";

export type WorkspaceGroup = "Daily work" | "Rules & access" | "Money movement" | "Records & issues";

export interface WorkspacePolicy {
  id: string;
  label: string;
  description: string;
  group: WorkspaceGroup;
  requiredScopes: readonly string[];
  requiredRoles: readonly string[];
  requiredAttributeKeys?: readonly string[];
  risk: ActionRisk;
  workspace: string;
}

export interface AuthorizationSnapshot {
  state: AuthorizationState;
  subject: string | null;
  tenantId: string | null;
  roles: readonly string[];
  scopes: readonly string[];
  attributes: Readonly<Record<string, string>>;
  snapshotId?: string;
  expiresAtUnix?: number;
  capabilities?: Readonly<Record<string, ServerCapabilityDecision>>;
}

export interface ServerCapabilityDecision {
  kind: "allow" | "deny" | "indeterminate";
  reasonCode: string;
  reason: string;
}

export type UiAccessDecision =
  | { kind: "allow"; reason: string }
  | { kind: "deny"; reason: string }
  | { kind: "indeterminate"; reason: string };

/**
 * This catalog is an affordance contract only. It is intentionally not a
 * security boundary; every API and Solana authority-sensitive operation must
 * repeat authorization and tenant/resource checks on the server.
 */
export const WORKSPACE_POLICIES: readonly WorkspacePolicy[] = [
  {
    id: "tax.quote.create",
    label: "Create tax quote",
    description: "Prepare a customer quote with a clear tax breakdown.",
    group: "Daily work",
    requiredScopes: ["tax:quote"],
    requiredRoles: [],
    requiredAttributeKeys: ["environment"],
    risk: "write",
    workspace: "quote",
  },
  {
    id: "tax.document.quote.create",
    label: "Calculate invoice tax",
    description: "Calculate tax across an invoice and its lines.",
    group: "Daily work",
    requiredScopes: ["tax:quote"],
    requiredRoles: [],
    requiredAttributeKeys: ["environment"],
    risk: "write",
    workspace: "document-quote",
  },
  {
    id: "tax.preview.run",
    label: "Explore a scenario",
    description: "Try a scenario without changing published rules or payments.",
    group: "Daily work",
    requiredScopes: [],
    requiredRoles: [],
    risk: "read",
    workspace: "preview",
  },
  {
    id: "tax.rule.govern",
    label: "Manage tax rules",
    description: "Create, review, approve, and publish tax rules.",
    group: "Rules & access",
    requiredScopes: ["tax:rule:read"],
    requiredRoles: ["tax_rule_author", "tax_rule_reviewer", "tax_rule_approver"],
    requiredAttributeKeys: ["environment", "jurisdiction"],
    risk: "governance",
    workspace: "governance",
  },
  {
    id: "settlement.wallet",
    label: "Review payment",
    description: "Check the payment details before the customer signs.",
    group: "Daily work",
    requiredScopes: ["tax:settlement:compile"],
    requiredRoles: [],
    risk: "financial",
    workspace: "checkout",
  },
  {
    id: "settlement.document.wallet",
    label: "Review document payment",
    description: "Review and approve payment for an invoice.",
    group: "Daily work",
    requiredScopes: ["tax:settlement:compile"],
    requiredRoles: [],
    risk: "financial",
    workspace: "document-checkout",
  },
  {
    id: "vault.reconciliation",
    label: "Review balance checks",
    description: "Compare recorded balances with the latest verified observation.",
    group: "Money movement",
    requiredScopes: ["tax:vault:read"],
    requiredRoles: [],
    risk: "read",
    workspace: "vault-reconciliation",
  },
  {
    id: "vault.release",
    label: "Release treasury funds",
    description: "Prepare a controlled treasury release for approval.",
    group: "Money movement",
    requiredScopes: ["tax:vault:propose"],
    requiredRoles: ["vault_release_proposer"],
    risk: "financial",
    workspace: "vault-release",
  },
  {
    id: "revenue_stream.read",
    label: "View partner payout plans",
    description: "Review governed partner payout plans and their verified balances.",
    group: "Money movement",
    requiredScopes: ["tax:stream:read"],
    requiredRoles: ["revenue_stream_proposer", "revenue_stream_approver", "revenue_stream_operator", "revenue_stream_auditor"],
    risk: "read",
    workspace: "revenue-streaming",
  },
  {
    id: "revenue_stream.propose",
    label: "Create partner payout plan",
    description: "Propose a bounded payout plan for a partner.",
    group: "Money movement",
    requiredScopes: ["tax:stream:propose"],
    requiredRoles: ["revenue_stream_proposer"],
    risk: "governance",
    workspace: "revenue-streaming",
  },
  {
    id: "revenue_stream.govern",
    label: "Approve partner payout plan",
    description: "Review and govern a partner payout plan.",
    group: "Money movement",
    requiredScopes: ["tax:stream:govern"],
    requiredRoles: ["revenue_stream_approver", "revenue_stream_governance"],
    risk: "governance",
    workspace: "revenue-streaming",
  },
  {
    id: "revenue_stream.claim.prepare",
    label: "Prepare partner payout release",
    description: "Reserve an available amount for a verified release.",
    group: "Money movement",
    requiredScopes: ["tax:stream:claim:prepare"],
    requiredRoles: ["revenue_stream_operator"],
    risk: "financial",
    workspace: "revenue-streaming",
  },
  {
    id: "settlement.adjustment",
    label: "Prepare an adjustment",
    description: "Correct a completed transaction with a clear record.",
    group: "Money movement",
    requiredScopes: ["tax:adjustment:propose"],
    requiredRoles: ["settlement_adjustment_proposer"],
    risk: "governance",
    workspace: "adjustments",
  },
  {
    id: "refund.claim.create",
    label: "Request a refund",
    description: "Prepare a refund request for review.",
    group: "Money movement",
    requiredScopes: ["tax:refund:claim:prepare"],
    requiredRoles: ["settlement_adjustment_proposer"],
    risk: "financial",
    workspace: "refund-claims",
  },
  {
    id: "refund.claim.cancel",
    label: "Cancel a refund request",
    description: "Stop a refund request with an approval record.",
    group: "Money movement",
    requiredScopes: ["tax:refund:claim:cancel:prepare"],
    requiredRoles: ["settlement_adjustment_governance"],
    risk: "governance",
    workspace: "refund-cancellations",
  },
  {
    id: "refund.payment.release",
    label: "Release a refund payment",
    description: "Release an approved refund after all checks pass.",
    group: "Money movement",
    requiredScopes: ["tax:refund:execute:prepare"],
    requiredRoles: ["settlement_adjustment_approver"],
    risk: "financial",
    workspace: "refund-executions",
  },
  {
    id: "evidence.replay",
    label: "Review exceptions",
    description: "Investigate and recover records that need attention.",
    group: "Records & issues",
    requiredScopes: ["tax:indexer:read"],
    requiredRoles: ["indexer_operator"],
    risk: "write",
    workspace: "replay-ops",
  },
  {
    id: "access.control",
    label: "View access rules",
    description: "See who can act and what requires approval.",
    group: "Rules & access",
    requiredScopes: ["audit:read"],
    requiredRoles: [],
    risk: "read",
    workspace: "access-control",
  },
];

export function authorizationSnapshotForToken(accessToken: string): AuthorizationSnapshot {
  if (!accessToken.trim()) {
    return { state: "anonymous", subject: null, tenantId: null, roles: [], scopes: [], attributes: {} };
  }

  // JWT claims are deliberately not decoded in the browser. Until the API
  // returns a signed, server-evaluated capability snapshot, the UI cannot
  // claim that a token is authorized for any resource or action.
  return {
    state: "server-evaluation-required",
    subject: null,
    tenantId: null,
    roles: [],
    scopes: [],
    attributes: {},
  };
}

const UUID_PATTERN = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
const UUIDV7_PATTERN = /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
const CAPABILITY_SCHEMA = "authorization-capabilities/v1";
const MAX_CAPABILITIES = 64;
const MAX_SNAPSHOT_LIFETIME_SECONDS = 60;

function recordValue(value: unknown, message: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) throw new Error(message);
  return value as Record<string, unknown>;
}

function stringValue(value: unknown, message: string, maxLength = 256): string {
  if (typeof value !== "string" || value.length === 0 || value.length > maxLength) throw new Error(message);
  return value;
}

function capabilityReason(reasonCode: string): string {
  switch (reasonCode) {
    case "scope_and_role_satisfied": return "The service confirmed this action for the current account.";
    case "scope_missing": return "This account does not have the permission for this action.";
    case "role_missing": return "This action needs an approved responsibility.";
    case "resource_context_required": return "Choose the relevant record or jurisdiction to complete the access check.";
    default: throw new Error("The service returned an unknown access reason.");
  }
}

export function parseAuthorizationCapabilities(value: unknown, nowUnix = Math.floor(Date.now() / 1000)): AuthorizationSnapshot {
  if (!Number.isSafeInteger(nowUnix) || nowUnix < 0) throw new Error("The access-check clock is invalid.");
  const record = recordValue(value, "The service returned an invalid access response.");
  if (record.schema !== CAPABILITY_SCHEMA) throw new Error("The service returned an unsupported access response.");
  const snapshotId = stringValue(record.snapshot_id, "The access response has no snapshot reference.");
  const tenantId = stringValue(record.tenant_id, "The access response has no account reference.");
  if (!UUIDV7_PATTERN.test(snapshotId) || !UUID_PATTERN.test(tenantId)) throw new Error("The access response has invalid identifiers.");
  const subject = stringValue(record.subject, "The access response has no signed-in person.");
  if (!Number.isSafeInteger(record.expires_at_unix) || (record.expires_at_unix as number) <= nowUnix || (record.expires_at_unix as number) > nowUnix + MAX_SNAPSHOT_LIFETIME_SECONDS + 30) {
    throw new Error("The access response is expired or outside its allowed lifetime.");
  }
  const capabilitiesRecord = recordValue(record.capabilities, "The access response has no capability decisions.");
  const entries = Object.entries(capabilitiesRecord);
  const expectedCapabilityIds = WORKSPACE_POLICIES.map((policy) => policy.id);
  if (entries.length !== expectedCapabilityIds.length || entries.length > MAX_CAPABILITIES) throw new Error("The access response has an incomplete capability catalog.");
  const expectedCapabilityIdSet = new Set(expectedCapabilityIds);
  const capabilities: Record<string, ServerCapabilityDecision> = {};
  for (const [id, rawDecision] of entries) {
    if (id.length === 0 || id.length > 128) throw new Error("The access response contains an invalid capability identifier.");
    if (!expectedCapabilityIdSet.has(id)) throw new Error("The access response contains an unknown capability identifier.");
    const decisionRecord = recordValue(rawDecision, "The access response contains an invalid decision.");
    const decision = decisionRecord.decision;
    if (decision !== "allow" && decision !== "deny" && decision !== "indeterminate") throw new Error("The access response contains an unknown decision.");
    const reasonCode = stringValue(decisionRecord.reason_code, "The access response contains no reason code.", 64);
    capabilities[id] = { kind: decision, reasonCode, reason: capabilityReason(reasonCode) };
  }
  if (expectedCapabilityIds.some((id) => !Object.prototype.hasOwnProperty.call(capabilities, id))) throw new Error("The access response has an incomplete capability catalog.");
  return {
    state: "server-allowed",
    subject,
    tenantId,
    roles: [],
    scopes: [],
    attributes: {},
    snapshotId,
    expiresAtUnix: record.expires_at_unix as number,
    capabilities,
  };
}

export function evaluateUiAccess(snapshot: AuthorizationSnapshot, policy: WorkspacePolicy): UiAccessDecision {
  if (snapshot.state === "anonymous") {
    return { kind: "deny", reason: "Sign in to request this action." };
  }
  if (snapshot.state === "server-evaluation-required") {
    return { kind: "indeterminate", reason: "The API must evaluate scope, role, tenant, resource, and policy conditions." };
  }
  if (snapshot.state === "server-denied") {
    return { kind: "deny", reason: "The server denied this capability for the current principal and tenant." };
  }
  const serverDecision = snapshot.capabilities?.[policy.id];
  if (serverDecision) return { kind: serverDecision.kind, reason: serverDecision.reason };
  const missingScope = policy.requiredScopes.find((scope) => !snapshot.scopes.includes(scope));
  if (missingScope) return { kind: "deny", reason: `Missing required scope: ${missingScope}.` };
  const missingRole = policy.requiredRoles.length > 0 && !policy.requiredRoles.some((role) => snapshot.roles.includes(role));
  if (missingRole) return { kind: "deny", reason: "The principal does not hold an approved role for this action." };
  if (!snapshot.tenantId) return { kind: "deny", reason: "A verified account context is required." };
  const missingAttribute = (policy.requiredAttributeKeys ?? []).find((key) => !snapshot.attributes[key]?.trim());
  if (missingAttribute) return { kind: "deny", reason: `Missing resource attribute: ${missingAttribute}.` };
  return { kind: "allow", reason: "The server-provided capability snapshot permits this action." };
}

export function policiesForGroup(group: WorkspaceGroup): readonly WorkspacePolicy[] {
  return WORKSPACE_POLICIES.filter((policy) => policy.group === group);
}
