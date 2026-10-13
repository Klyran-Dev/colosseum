import { AdminAction, AdminCapability, AdminDecision, AdminMetric, AdminQueryError, AdminSeriesPoint, AdminSession, AdminSnapshot } from "./model";

const REQUEST_TIMEOUT_MS = 8_000;

function objectValue(value: unknown): Record<string, unknown> {
  return value && typeof value === "object" && !Array.isArray(value) ? value as Record<string, unknown> : {};
}

function stringValue(value: unknown): string {
  return typeof value === "string" ? value : "";
}

function invalidResponse(message: string): never {
  throw { code: "invalid", message } satisfies AdminQueryError;
}

function parseMetric(value: unknown): AdminMetric | null {
  const source = objectValue(value);
  const id = stringValue(source.id);
  const label = stringValue(source.label);
  if (!id || !label) return null;
  const tone = source.tone === "violet" || source.tone === "cyan" || source.tone === "amber" || source.tone === "green" ? source.tone : undefined;
  return { id, label, value: stringValue(source.value), detail: stringValue(source.detail), trend: stringValue(source.trend) || undefined, tone };
}

function parseCapability(value: unknown): AdminCapability | null {
  const source = objectValue(value);
  const id = stringValue(source.id);
  const label = stringValue(source.label);
  const decision = source.decision === "allow" || source.decision === "deny" || source.decision === "indeterminate" ? source.decision : null;
  const control = source.control === "RBAC" || source.control === "ABAC" || source.control === "PBAC" || source.control === "server" ? source.control : null;
  if (!id || !label || !decision || !control) return null;
  return { id, label, decision, reason: stringValue(source.reason), control };
}

function parseCapabilitySnapshot(payload: unknown): AdminSnapshot {
  const source = objectValue(payload);
  if (source.schema !== "authorization-capabilities/v1") invalidResponse("The authorization service returned an unsupported capability snapshot.");
  const snapshotId = stringValue(source.snapshot_id);
  const subject = stringValue(source.subject);
  const tenantId = stringValue(source.tenant_id);
  if (!snapshotId || !subject || !tenantId) invalidResponse("The authorization service returned an incomplete capability snapshot.");
  if (typeof source.expires_at_unix !== "number" || !Number.isSafeInteger(source.expires_at_unix) || source.expires_at_unix <= 0) invalidResponse("The authorization service returned an invalid capability expiry.");
  const rawCapabilities = objectValue(source.capabilities);
  if (!source.capabilities || typeof source.capabilities !== "object" || Array.isArray(source.capabilities)) invalidResponse("The authorization service returned no capability catalog.");
  if (!Array.isArray(source.roles) || source.roles.some((role) => typeof role !== "string")) invalidResponse("The authorization service returned invalid responsibility metadata.");
  const capabilities = Object.entries(rawCapabilities).map(([id, value]) => {
    const decision = objectValue(value);
    if (!id || (decision.decision !== "allow" && decision.decision !== "deny" && decision.decision !== "indeterminate") || !stringValue(decision.reason_code)) invalidResponse("The authorization service returned an invalid capability decision.");
    const parsedDecision: AdminDecision = decision.decision;
    return { id, label: id, decision: parsedDecision, reason: stringValue(decision.reason_code), control: "server" as const };
  });
  const expiresAtUnix = source.expires_at_unix;
  const roles = source.roles.filter((role): role is string => typeof role === "string");
  const allowed = capabilities.filter((capability) => capability.decision === "allow").length;
  const denied = capabilities.filter((capability) => capability.decision === "deny").length;
  const indeterminate = capabilities.filter((capability) => capability.decision === "indeterminate").length;
  return {
    tenant: { id: tenantId, name: "Authenticated tenant" },
    generatedAt: "",
    authorizationVersion: snapshotId,
    expiresAtUnix,
    subject,
    roles,
    metrics: [
      { id: "capabilities", label: "Capabilities evaluated", value: String(capabilities.length), detail: "Current server policy snapshot", tone: "violet" },
      { id: "allowed", label: "Allowed capabilities", value: String(allowed), detail: "Scope and role satisfied", tone: "green" },
      { id: "context", label: "Context required", value: String(indeterminate), detail: "Awaiting resource attributes", tone: "amber" },
      { id: "denied", label: "Denied capabilities", value: String(denied), detail: "Missing scope or responsibility", tone: "cyan" },
    ],
    capabilities,
    policySeries: [],
    recentActions: [],
  };
}

function parseEvidenceActions(payload: unknown): AdminAction[] {
  if (!Array.isArray(payload)) return [];
  return payload.flatMap((value) => {
    const source = objectValue(value);
    const id = stringValue(source.id);
    if (!id) return [];
    const outcome = stringValue(source.outcome).toLowerCase();
    const status: AdminAction["status"] = outcome === "accepted" || outcome === "verified" ? "verified" : outcome === "denied" || outcome === "rejected" ? "blocked" : outcome === "pending" ? "pending" : "unknown";
    return [{ id, action: stringValue(source.purpose) || "Evidence access", subject: stringValue(source.calculation_id) || "Redacted calculation", status, occurredAt: stringValue(source.occurred_at), actor: stringValue(source.actor_subject) || "Redacted actor" }];
  });
}

function evidenceSeries(actions: AdminAction[]): AdminSeriesPoint[] {
  const counts = new Map<string, number>();
  for (const action of actions) {
    const date = new Date(action.occurredAt);
    if (!Number.isFinite(date.getTime())) continue;
    const label = date.toISOString().slice(5, 10);
    counts.set(label, (counts.get(label) ?? 0) + 1);
  }
  return Array.from(counts, ([label, value]) => ({ label, value })).sort((left, right) => left.label.localeCompare(right.label));
}

function parseSeriesPoint(value: unknown): AdminSeriesPoint | null {
  const source = objectValue(value);
  const label = stringValue(source.label);
  const numericValue = source.value;
  if (!label || typeof numericValue !== "number" || !Number.isFinite(numericValue) || numericValue < 0) return null;
  return { label, value: numericValue };
}

function parseAction(value: unknown): AdminAction | null {
  const source = objectValue(value);
  const id = stringValue(source.id);
  const status = source.status === "verified" || source.status === "pending" || source.status === "blocked" || source.status === "unknown" ? source.status : null;
  if (!id || !status) return null;
  return { id, action: stringValue(source.action), subject: stringValue(source.subject), status, occurredAt: stringValue(source.occurred_at), actor: stringValue(source.actor) };
}

export function parseAdminSnapshot(payload: unknown): AdminSnapshot {
  const source = objectValue(payload);
  const tenant = objectValue(source.tenant);
  return {
    tenant: { id: stringValue(tenant.id), name: stringValue(tenant.name) },
    generatedAt: stringValue(source.generated_at),
    authorizationVersion: stringValue(source.authorization_version),
    roles: [],
    metrics: Array.isArray(source.metrics) ? source.metrics.map(parseMetric).filter((value): value is AdminMetric => value !== null) : [],
    capabilities: Array.isArray(source.capabilities) ? source.capabilities.map(parseCapability).filter((value): value is AdminCapability => value !== null) : [],
    policySeries: Array.isArray(source.policy_series) ? source.policy_series.map(parseSeriesPoint).filter((value): value is AdminSeriesPoint => value !== null) : [],
    recentActions: Array.isArray(source.recent_actions) ? source.recent_actions.map(parseAction).filter((value): value is AdminAction => value !== null) : [],
  };
}

export async function requestJson(path: string, session: AdminSession | null, init: RequestInit = {}): Promise<unknown> {
  const controller = new AbortController();
  const timer = window.setTimeout(() => controller.abort(), REQUEST_TIMEOUT_MS);
  try {
    const headers = new Headers(init.headers);
    headers.set("accept", "application/json");
    if (session?.accessToken) headers.set("authorization", `Bearer ${session.accessToken}`);
    const response = await fetch(path, { ...init, headers, signal: controller.signal });
    const payload = await response.json().catch(() => null);
    if (!response.ok) {
      const code: AdminQueryError["code"] = response.status === 401 || response.status === 403 ? "unauthorized" : response.status >= 500 ? "unavailable" : "invalid";
      const envelope = objectValue(payload);
      const detail = objectValue(envelope.error);
      throw { code, message: stringValue(envelope.message) || stringValue(detail.message) || `The admin service returned HTTP ${response.status}.`, correlationId: stringValue(response.headers.get("x-correlation-id")) || undefined } satisfies AdminQueryError;
    }
    return payload;
  } catch (error) {
    if (error instanceof DOMException && error.name === "AbortError") throw { code: "unavailable", message: "The admin service did not respond before the request deadline." } satisfies AdminQueryError;
    if (error && typeof error === "object" && "code" in error) throw error;
    throw { code: "unavailable", message: "The admin service could not be reached." } satisfies AdminQueryError;
  } finally {
    window.clearTimeout(timer);
  }
}

export async function fetchAdminSnapshot(session: AdminSession | null): Promise<AdminSnapshot> {
  const snapshot = parseCapabilitySnapshot(await requestJson("/api/v1/authorization/capabilities", session));
  if (snapshot.capabilities.some((capability) => capability.id === "evidence.audit.read" && capability.decision === "allow")) {
    try {
      const actions = parseEvidenceActions(await requestJson("/api/v1/audit/evidence-access?limit=25", session));
      return { ...snapshot, recentActions: actions, policySeries: evidenceSeries(actions) };
    } catch {
      // Capability evaluation remains authoritative even when the optional evidence projection is unavailable.
    }
  }
  return snapshot;
}
