export const GOVERNANCE_ROLES = [
  "tax_rule_author",
  "tax_rule_reviewer",
  "tax_rule_approver",
  "tax_rule_auditor",
] as const;

export const GOVERNANCE_STATUSES = [
  "draft",
  "simulated",
  "reviewed",
  "approved",
  "published",
  "retired",
] as const;

export type GovernanceStatus = (typeof GOVERNANCE_STATUSES)[number];

export type GovernanceEvent = {
  id: string;
  actor_subject: string;
  action: "create" | "simulate" | "review" | "approve" | "publish" | "retire";
  from_status: GovernanceStatus | "none";
  to_status: GovernanceStatus;
  evidence: Record<string, unknown>;
  occurred_at: string;
};

export type CreatedRuleDraft = {
  rule_set_id: string;
  external_id: string;
  version: number;
  jurisdiction: string;
  regime: string;
  status: "draft";
  rule_digest: number[];
  effective_from: string;
  effective_until: string | null;
  governance_revision: number;
};

export type GovernanceState = {
  rule_set_id: string;
  external_id: string;
  version: number;
  jurisdiction: string;
  regime: string;
  status: GovernanceStatus;
  rule_digest_hex: string;
  author_subject: string | null;
  reviewer_subject: string | null;
  approver_subject: string | null;
  governance_revision: number;
  effective_from: string;
  effective_until: string | null;
  events: GovernanceEvent[];
};

const UUID_V7 = /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
const DIGEST_HEX = /^[0-9a-f]{64}$/i;
const STATUS_SET = new Set<string>(GOVERNANCE_STATUSES);

export function nextGovernanceStatuses(status: GovernanceStatus): GovernanceStatus[] {
  switch (status) {
    case "draft": return ["simulated"];
    case "simulated": return ["reviewed"];
    case "reviewed": return ["approved"];
    case "approved": return ["published"];
    case "published": return ["retired"];
    case "retired": return [];
  }
}

export function transitionLabel(from: GovernanceStatus, to: GovernanceStatus): string {
  const labels: Record<string, string> = {
    "draft:simulated": "Simulate rule",
    "simulated:reviewed": "Submit for review",
    "reviewed:approved": "Approve for publication",
    "approved:published": "Publish rule",
    "published:retired": "Retire rule",
  };
  return labels[`${from}:${to}`] ?? `Advance to ${to}`;
}

function invalidSnapshot(): Error {
  return new Error("The governance snapshot is malformed and cannot be displayed safely.");
}

function stringOrNull(value: unknown): value is string | null {
  return value === null || typeof value === "string";
}

function parseStatus(value: unknown): GovernanceStatus {
  if (typeof value !== "string" || !STATUS_SET.has(value)) throw invalidSnapshot();
  return value as GovernanceStatus;
}

function parseEventFromStatus(value: unknown, action: string): GovernanceStatus | "none" {
  if (action === "create" && value === "none") return "none";
  return parseStatus(value);
}

function parseEvent(value: unknown): GovernanceEvent {
  if (typeof value !== "object" || value === null) throw invalidSnapshot();
  const event = value as Record<string, unknown>;
  if (
    typeof event.id !== "string" || !UUID_V7.test(event.id)
    || typeof event.actor_subject !== "string" || event.actor_subject.trim() === ""
    || typeof event.action !== "string"
    || !["create", "simulate", "review", "approve", "publish", "retire"].includes(event.action)
    || typeof event.occurred_at !== "string"
    || typeof event.evidence !== "object" || event.evidence === null || Array.isArray(event.evidence)
  ) throw invalidSnapshot();
  return {
    id: event.id,
    actor_subject: event.actor_subject,
    action: event.action as GovernanceEvent["action"],
    from_status: parseEventFromStatus(event.from_status, event.action),
    to_status: parseStatus(event.to_status),
    evidence: event.evidence as Record<string, unknown>,
    occurred_at: event.occurred_at,
  };
}

export function parseCreatedRuleDraft(value: unknown): CreatedRuleDraft {
  if (typeof value !== "object" || value === null) throw new Error("The created rule draft response is malformed.");
  const draft = value as Record<string, unknown>;
  if (
    typeof draft.rule_set_id !== "string" || !UUID_V7.test(draft.rule_set_id)
    || typeof draft.external_id !== "string" || draft.external_id.trim() === ""
    || typeof draft.version !== "number" || !Number.isSafeInteger(draft.version) || draft.version < 1
    || typeof draft.jurisdiction !== "string" || draft.jurisdiction.trim() === ""
    || typeof draft.regime !== "string" || draft.regime.trim() === ""
    || draft.status !== "draft"
    || !Array.isArray(draft.rule_digest) || draft.rule_digest.length !== 32
    || draft.rule_digest.some((byte) => typeof byte !== "number" || !Number.isInteger(byte) || byte < 0 || byte > 255)
    || typeof draft.effective_from !== "string" || draft.effective_from.trim() === ""
    || (draft.effective_until !== null && typeof draft.effective_until !== "string")
    || typeof draft.governance_revision !== "number"
    || !Number.isSafeInteger(draft.governance_revision) || draft.governance_revision !== 0
  ) throw new Error("The created rule draft response is malformed.");
  return {
    rule_set_id: draft.rule_set_id,
    external_id: draft.external_id,
    version: draft.version,
    jurisdiction: draft.jurisdiction,
    regime: draft.regime,
    status: "draft",
    rule_digest: draft.rule_digest,
    effective_from: draft.effective_from,
    effective_until: draft.effective_until,
    governance_revision: 0,
  };
}

export function parseGovernanceState(value: unknown): GovernanceState {
  if (typeof value !== "object" || value === null) throw invalidSnapshot();
  const state = value as Record<string, unknown>;
  if (
    typeof state.rule_set_id !== "string" || !UUID_V7.test(state.rule_set_id)
    || typeof state.external_id !== "string" || state.external_id.trim() === ""
    || typeof state.version !== "number" || !Number.isSafeInteger(state.version) || state.version < 1
    || typeof state.jurisdiction !== "string" || state.jurisdiction.trim() === ""
    || typeof state.regime !== "string" || state.regime.trim() === ""
    || typeof state.rule_digest_hex !== "string" || !DIGEST_HEX.test(state.rule_digest_hex)
    || !stringOrNull(state.author_subject) || !stringOrNull(state.reviewer_subject)
    || !stringOrNull(state.approver_subject)
    || typeof state.governance_revision !== "number"
    || !Number.isSafeInteger(state.governance_revision) || state.governance_revision < 0
    || typeof state.effective_from !== "string" || state.effective_from.trim() === ""
    || !stringOrNull(state.effective_until)
    || !Array.isArray(state.events)
  ) throw invalidSnapshot();
  return {
    rule_set_id: state.rule_set_id,
    external_id: state.external_id,
    version: state.version,
    jurisdiction: state.jurisdiction,
    regime: state.regime,
    status: parseStatus(state.status),
    rule_digest_hex: state.rule_digest_hex,
    author_subject: state.author_subject,
    reviewer_subject: state.reviewer_subject,
    approver_subject: state.approver_subject,
    governance_revision: state.governance_revision,
    effective_from: state.effective_from,
    effective_until: state.effective_until,
    events: state.events.map(parseEvent),
  };
}
