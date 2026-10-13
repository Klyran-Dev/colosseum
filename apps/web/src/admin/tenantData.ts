import { BROWSER_AUTH_CONFIG, BROWSER_AUTH_SERVICE_BASE_URL } from "../authConfig";
import { createClientUuidV7 } from "../clientIdentity";
import { AdminSession } from "./model";
import { requestJson } from "./data";
import { TenantContext, TenantSetupStep } from "./tenantModel";

function authEndpoint(path: string): string {
  const base = BROWSER_AUTH_SERVICE_BASE_URL || BROWSER_AUTH_CONFIG.emailOtp?.baseUrl || "";
  return `${base}/v1/${path}`;
}

function objectValue(value: unknown): Record<string, unknown> {
  return value && typeof value === "object" && !Array.isArray(value) ? value as Record<string, unknown> : {};
}

function stringValue(value: unknown): string {
  return typeof value === "string" ? value : "";
}

function uuidValue(value: unknown): string {
  const candidate = stringValue(value);
  if (!/^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/iu.test(candidate)) throw new Error("The identity service returned an invalid tenant reference.");
  return candidate;
}

function parseStep(value: unknown): TenantSetupStep {
  const source = objectValue(value);
  const id = source.id;
  if (id !== "workspace_profile" && id !== "settlement_route" && id !== "tax_policy") throw new Error("The identity service returned an unknown setup step.");
  const status = source.status;
  if (status !== "complete" && status !== "pending" && status !== "blocked") throw new Error("The identity service returned an invalid setup step status.");
  const label = stringValue(source.label);
  const actionPath = stringValue(source.action_path);
  if (!label || !actionPath.startsWith("/admin/")) throw new Error("The identity service returned incomplete setup steps.");
  return { id, label, status, actionPath };
}

export function parseTenantContext(payload: unknown): TenantContext {
  const source = objectValue(payload);
  if (source.schema !== "auth-tenant-context/v1") throw new Error("The identity service returned an unsupported tenant context.");
  const tenantId = uuidValue(source.tenant_id);
  const displayName = stringValue(source.display_name);
  if (!displayName || displayName.length > 120) throw new Error("The identity service returned an invalid workspace name.");
  if (source.status !== "active" && source.status !== "suspended") throw new Error("The identity service returned an invalid workspace status.");
  const status: TenantContext["status"] = source.status;
  const readiness = objectValue(source.readiness);
  if (readiness.state !== "not_started" && readiness.state !== "in_progress" && readiness.state !== "ready" && readiness.state !== "blocked") throw new Error("The identity service returned an invalid readiness state.");
  const completedSteps = typeof readiness.completed_steps === "number" ? readiness.completed_steps : NaN;
  const totalSteps = typeof readiness.total_steps === "number" ? readiness.total_steps : NaN;
  if (!Number.isSafeInteger(completedSteps) || !Number.isSafeInteger(totalSteps) || totalSteps !== 3 || completedSteps < 0 || completedSteps > totalSteps || !Array.isArray(readiness.steps) || readiness.steps.length !== totalSteps) throw new Error("The identity service returned incomplete setup steps.");
  const steps = readiness.steps.map(parseStep);
  if (new Set(steps.map((step) => step.id)).size !== totalSteps) throw new Error("The identity service returned duplicate setup steps.");
  return { schema: "auth-tenant-context/v1", tenantId, displayName, status, readiness: { state: readiness.state, completedSteps, totalSteps, steps } };
}

export async function fetchTenantContext(session: AdminSession | null): Promise<TenantContext> {
  return parseTenantContext(await requestJson(authEndpoint("tenant/context"), session));
}

export async function updateTenantProfile(session: AdminSession, displayName: string): Promise<void> {
  await requestJson(authEndpoint("tenant/profile"), session, { method: "POST", headers: { "content-type": "application/json", "idempotency-key": createClientUuidV7() }, body: JSON.stringify({ display_name: displayName }) });
}
