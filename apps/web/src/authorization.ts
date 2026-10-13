import authorizationCatalog from "../../../contracts/authorization-capabilities-v1.json";

export type AuthorizationState =
  | "anonymous"
  | "server-evaluation-required"
  | "server-denied"
  | "server-allowed";

export type ActionRisk = "read" | "write" | "financial" | "governance";

export type ServerCapabilityReasonCode =
  | "scope_and_role_satisfied"
  | "scope_missing"
  | "role_missing"
  | "resource_context_required";

export type UiAccessReasonCode =
  | "sign_in_required"
  | "public_access"
  | "server_evaluation_required"
  | "server_denied"
  | "missing_expiry"
  | "clock_unavailable"
  | "access_expired"
  | "missing_capability_decision"
  | "unknown_policy"
  | ServerCapabilityReasonCode;

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
  email?: string | null;
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
  reasonCode: ServerCapabilityReasonCode;
  reason: string;
}

export type UiAccessDecision =
  | { kind: "allow"; reasonCode: UiAccessReasonCode; reason: string }
  | { kind: "deny"; reasonCode: UiAccessReasonCode; reason: string }
  | { kind: "indeterminate"; reasonCode: UiAccessReasonCode; reason: string };

type CanonicalPolicy = (typeof authorizationCatalog.policies)[number];

/**
 * This is a presentation projection of the server-owned catalog. It is not a
 * security boundary; every API and Solana authority-sensitive operation must
 * repeat authorization and tenant/resource checks on the server.
 */
export const WORKSPACE_POLICIES: readonly WorkspacePolicy[] = Object.freeze(
  authorizationCatalog.policies.map((policy: CanonicalPolicy) => ({
    id: policy.id,
    label: policy.label,
    description: policy.description,
    group: policy.group as WorkspaceGroup,
    requiredScopes: policy.required_scopes,
    requiredRoles: policy.required_roles,
    requiredAttributeKeys: policy.required_attributes.length > 0 ? policy.required_attributes : undefined,
    risk: policy.risk as ActionRisk,
    workspace: policy.workspace,
  })),
);

export function authorizationSnapshotForToken(accessToken: string): AuthorizationSnapshot {
  if (!accessToken.trim()) {
    return { state: "anonymous", subject: null, email: null, tenantId: null, roles: [], scopes: [], attributes: {} };
  }

  // JWT claims are deliberately not decoded in the browser. Until the API
  // returns a signed, server-evaluated capability snapshot, the UI cannot
  // claim that a token is authorized for any resource or action.
  return {
    state: "server-evaluation-required",
    subject: null,
    email: null,
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

export class AuthorizationCapabilitiesError extends Error {
  readonly code: "expired";

  constructor(message: string) {
    super(message);
    this.name = "AuthorizationCapabilitiesError";
    this.code = "expired";
  }
}

function recordValue(value: unknown, message: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) throw new Error(message);
  return value as Record<string, unknown>;
}

function stringValue(value: unknown, message: string, maxLength = 256): string {
  if (typeof value !== "string" || value.length === 0 || value.length > maxLength) throw new Error(message);
  return value;
}

function capabilityReason(reasonCode: ServerCapabilityReasonCode): string {
  switch (reasonCode) {
    case "scope_and_role_satisfied": return "The service confirmed this action for the current account.";
    case "scope_missing": return "This account does not have the permission for this action.";
    case "role_missing": return "This action needs an approved responsibility.";
    case "resource_context_required": return "Choose the relevant record or jurisdiction to complete the access check.";
    default: throw new Error("The service returned an unknown access reason.");
  }
}

function capabilityDecisionMatchesReason(decision: unknown, reasonCode: ServerCapabilityReasonCode): boolean {
  if (decision === "allow") return reasonCode === "scope_and_role_satisfied";
  if (decision === "deny") return reasonCode === "scope_missing" || reasonCode === "role_missing";
  if (decision === "indeterminate") return reasonCode === "resource_context_required";
  return false;
}

export function parseAuthorizationCapabilities(value: unknown, nowUnix = Math.floor(Date.now() / 1000)): AuthorizationSnapshot {
  if (!Number.isSafeInteger(nowUnix) || nowUnix < 0) throw new Error("The access-check clock is invalid.");
  const record = recordValue(value, "The service returned an invalid access response.");
  if (record.schema !== CAPABILITY_SCHEMA) throw new Error("The service returned an unsupported access response.");
  const snapshotId = stringValue(record.snapshot_id, "The access response has no snapshot reference.");
  const tenantId = stringValue(record.tenant_id, "The access response has no account reference.");
  if (!UUIDV7_PATTERN.test(snapshotId) || !UUID_PATTERN.test(tenantId)) throw new Error("The access response has invalid identifiers.");
  const subject = stringValue(record.subject, "The access response has no signed-in person.");
  const email = record.email === undefined ? null : stringValue(record.email, "The access response contains an invalid email identity.", 320);
  if (email !== null && (email !== email.toLowerCase() || email.trim() !== email || !/^[^\s@]+@[^\s@]+\.[^\s@]+$/u.test(email))) {
    throw new Error("The access response contains an invalid email identity.");
  }
  if (!Number.isSafeInteger(record.expires_at_unix) || (record.expires_at_unix as number) <= nowUnix || (record.expires_at_unix as number) > nowUnix + MAX_SNAPSHOT_LIFETIME_SECONDS + 30) {
    throw new AuthorizationCapabilitiesError("The access response is expired or outside its allowed lifetime.");
  }
  const capabilitiesRecord = recordValue(record.capabilities, "The access response has no capability decisions.");
  const roles = record.roles === undefined
    ? []
    : Array.isArray(record.roles)
      ? record.roles.map((role) => stringValue(role, "The access response contains an invalid role.", 128))
      : (() => { throw new Error("The access response contains invalid roles."); })();
  if (roles.length > 32 || new Set(roles).size !== roles.length) throw new Error("The access response contains duplicate or excessive roles.");
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
    const reasonCodeValue = stringValue(decisionRecord.reason_code, "The access response contains no reason code.", 64);
    if (!(["scope_and_role_satisfied", "scope_missing", "role_missing", "resource_context_required"] as const).includes(reasonCodeValue as ServerCapabilityReasonCode)) throw new Error("The service returned an unknown access reason.");
    const reasonCode = reasonCodeValue as ServerCapabilityReasonCode;
    const reason = capabilityReason(reasonCode);
    if (!capabilityDecisionMatchesReason(decision, reasonCode)) throw new Error("The access response contains a contradictory decision and reason.");
    capabilities[id] = { kind: decision, reasonCode, reason };
  }
  if (expectedCapabilityIds.some((id) => !Object.prototype.hasOwnProperty.call(capabilities, id))) throw new Error("The access response has an incomplete capability catalog.");
  return {
    state: "server-allowed",
    subject,
    email,
    tenantId,
    roles,
    scopes: [],
    attributes: {},
    snapshotId,
    expiresAtUnix: record.expires_at_unix as number,
    capabilities,
  };
}

export function evaluateUiAccess(
  snapshot: AuthorizationSnapshot,
  policy: WorkspacePolicy,
  nowUnix = Math.floor(Date.now() / 1000),
): UiAccessDecision {
  if (snapshot.state === "anonymous") {
    if (!workspaceRequiresAuthentication(policy.workspace)) {
      return { kind: "allow", reasonCode: "public_access", reason: "This workspace is available without a signed-in account." };
    }
    return { kind: "deny", reasonCode: "sign_in_required", reason: "Sign in to request this action." };
  }
  if (snapshot.state === "server-evaluation-required") {
    return { kind: "indeterminate", reasonCode: "server_evaluation_required", reason: "The API must evaluate scope, role, tenant, resource, and policy conditions." };
  }
  if (snapshot.state === "server-denied") {
    return { kind: "deny", reasonCode: "server_denied", reason: "The server denied this capability for the current principal and tenant." };
  }
  if (snapshot.expiresAtUnix === undefined) {
    return { kind: "indeterminate", reasonCode: "missing_expiry", reason: "The access check has no expiry." };
  }
  if (!Number.isSafeInteger(nowUnix) || nowUnix < 0) {
    return { kind: "indeterminate", reasonCode: "clock_unavailable", reason: "The access-check clock is unavailable." };
  }
  if (!Number.isSafeInteger(snapshot.expiresAtUnix) || snapshot.expiresAtUnix <= nowUnix) {
    return { kind: "indeterminate", reasonCode: "access_expired", reason: "The access check has expired; refresh before continuing." };
  }
  const serverDecision = snapshot.capabilities?.[policy.id];
  if (serverDecision) return { kind: serverDecision.kind, reasonCode: serverDecision.reasonCode, reason: serverDecision.reason };
  // The browser must never reconstruct RBAC/PBAC/ABAC from copied role,
  // scope, tenant, or attribute fields. A server-allowed snapshot without an
  // decision for this exact policy is incomplete and cannot drive an
  // optimistic affordance.
  return { kind: "indeterminate", reasonCode: "missing_capability_decision", reason: "The service has not returned a decision for this action." };
}

export function authorizationSnapshotRefreshDelayMs(
  snapshot: AuthorizationSnapshot,
  nowUnix = Math.floor(Date.now() / 1000),
): number | null {
  if (snapshot.state !== "server-allowed") return null;
  if (!Number.isSafeInteger(nowUnix) || nowUnix < 0) return 0;
  const expiresAtUnix = snapshot.expiresAtUnix;
  if (typeof expiresAtUnix !== "number" || !Number.isSafeInteger(expiresAtUnix)) return 0;
  return Math.max(0, (expiresAtUnix - nowUnix) * 1_000);
}

export function policiesForGroup(group: WorkspaceGroup): readonly WorkspacePolicy[] {
  return WORKSPACE_POLICIES.filter((policy) => policy.group === group);
}

export function workspaceRequiresAuthentication(workspace: string): boolean {
  const policies = WORKSPACE_POLICIES.filter((policy) => policy.workspace === workspace);
  if (policies.length === 0) return true;
  return policies.some((policy) => policy.requiredScopes.length > 0 || policy.requiredRoles.length > 0 || (policy.requiredAttributeKeys?.length ?? 0) > 0);
}
