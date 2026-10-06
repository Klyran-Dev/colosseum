export const MAX_EVIDENCE_ACCESS_AUDIT_LIST_LIMIT = 100;

const UUID_V7 = /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
const OUTCOMES = new Set(["allowed", "denied"]);

export type EvidenceAccessOutcome = "allowed" | "denied";

export interface EvidenceAccessAuditEvent {
  id: string;
  calculation_id: string;
  actor_subject: string;
  request_id: string;
  purpose: string;
  outcome: EvidenceAccessOutcome;
  facts_digest: number[] | null;
  occurred_at: string;
}

function malformed(): Error {
  return new Error("The evidence-access audit response is malformed and cannot be displayed safely.");
}

function objectValue(value: unknown): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) throw malformed();
  return value as Record<string, unknown>;
}

function uuidV7(value: unknown): string {
  if (typeof value !== "string" || !UUID_V7.test(value)) throw malformed();
  return value;
}

function boundedText(value: unknown, max: number): string {
  if (typeof value !== "string" || value.length === 0 || value.length > max) throw malformed();
  return value;
}

function digest(value: unknown): number[] | null {
  if (value === null) return null;
  if (!Array.isArray(value) || value.length !== 32 || value.some((byte) => !Number.isInteger(byte) || byte < 0 || byte > 255)) {
    throw malformed();
  }
  return [...value] as number[];
}

function parseEvent(value: unknown): EvidenceAccessAuditEvent {
  const event = objectValue(value);
  const occurredAt = boundedText(event.occurred_at, 64);
  if (Number.isNaN(Date.parse(occurredAt))) throw malformed();
  const outcome = boundedText(event.outcome, 64);
  if (!OUTCOMES.has(outcome)) throw malformed();
  return {
    id: uuidV7(event.id),
    calculation_id: uuidV7(event.calculation_id),
    actor_subject: boundedText(event.actor_subject, 256),
    request_id: uuidV7(event.request_id),
    purpose: boundedText(event.purpose, 128),
    outcome: outcome as EvidenceAccessOutcome,
    facts_digest: digest(event.facts_digest),
    occurred_at: occurredAt,
  };
}

export function parseEvidenceAccessAuditList(value: unknown): EvidenceAccessAuditEvent[] {
  if (!Array.isArray(value) || value.length > MAX_EVIDENCE_ACCESS_AUDIT_LIST_LIMIT) throw malformed();
  return value.map(parseEvent);
}

export function buildEvidenceAccessAuditUrl(apiBase: string, limit: number, calculationId?: string): string {
  if (!Number.isSafeInteger(limit) || limit < 1 || limit > MAX_EVIDENCE_ACCESS_AUDIT_LIST_LIMIT) {
    throw new Error("Evidence-access audit list limit is out of bounds.");
  }
  if (calculationId !== undefined) uuidV7(calculationId);
  const base = apiBase.replace(/\/$/, "");
  const params = new URLSearchParams({ limit: String(limit) });
  if (calculationId !== undefined) params.set("calculation_id", calculationId);
  return `${base}/v1/audit/evidence-access?${params.toString()}`;
}
