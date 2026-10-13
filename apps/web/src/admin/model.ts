export type AdminSection = "overview" | "quotes" | "rules" | "settlements" | "treasury" | "access" | "evidence" | "settings";

export type AdminDecision = "allow" | "deny" | "indeterminate";

export interface AdminCapability {
  id: string;
  label: string;
  decision: AdminDecision;
  reason: string;
  control: "RBAC" | "ABAC" | "PBAC" | "server";
}

export interface AdminMetric {
  id: string;
  label: string;
  value: string;
  detail: string;
  trend?: string;
  tone?: "violet" | "cyan" | "amber" | "green";
}

export interface AdminSeriesPoint {
  label: string;
  value: number;
}

export interface AdminAction {
  id: string;
  action: string;
  subject: string;
  status: "verified" | "pending" | "blocked" | "unknown";
  occurredAt: string;
  actor: string;
}

export interface AdminSnapshot {
  tenant: { id: string; name: string };
  generatedAt: string;
  authorizationVersion: string;
  expiresAtUnix?: number;
  subject?: string;
  roles: string[];
  metrics: AdminMetric[];
  capabilities: AdminCapability[];
  policySeries: AdminSeriesPoint[];
  recentActions: AdminAction[];
}

export interface AdminSession {
  accessToken: string;
  email: string;
  expiresAtUnix: number;
}

export interface AdminQueryError {
  code: "unavailable" | "unauthorized" | "invalid";
  message: string;
  correlationId?: string;
}

export const NAV_ITEMS: readonly { to: string; label: string; section: AdminSection; detail: string }[] = [
  { to: "/admin", label: "Overview", section: "overview", detail: "Control-plane health and verified activity" },
  { to: "/admin/tax/quotes", label: "Tax quotes", section: "quotes", detail: "Prepare and inspect governed quotes" },
  { to: "/admin/tax/rules", label: "Tax rules", section: "rules", detail: "Draft, review, and publish rule versions" },
  { to: "/admin/settlements", label: "Settlements", section: "settlements", detail: "Review payment and withholding state" },
  { to: "/admin/treasury", label: "Treasury", section: "treasury", detail: "Govern vault release workflows" },
  { to: "/admin/access", label: "Access & approvals", section: "access", detail: "Manage server-owned responsibility" },
  { to: "/admin/evidence", label: "Evidence & audit", section: "evidence", detail: "Trace decisions and chain evidence" },
  { to: "/admin/settings", label: "Settings", section: "settings", detail: "Session, tenant, and support boundaries" },
];

export const SECTION_COPY: Record<Exclude<AdminSection, "overview">, { title: string; description: string; capability: string }> = {
  quotes: { title: "Tax quotes", description: "Prepare a customer quote with a verified tax breakdown.", capability: "tax.quote.create" },
  rules: { title: "Tax rules", description: "Draft, review, approve, and publish immutable tax policy versions.", capability: "tax.rule.govern" },
  settlements: { title: "Settlements", description: "Review payment intents, withholding, and finalized settlement evidence.", capability: "settlement.wallet" },
  treasury: { title: "Treasury", description: "Prepare bounded vault releases with independent approval evidence.", capability: "vault.release" },
  access: { title: "Access & approvals", description: "Inspect tenant responsibilities and server-evaluated policy decisions.", capability: "access.control" },
  evidence: { title: "Evidence & audit", description: "Replay calculations and review protected access activity.", capability: "evidence.audit.read" },
  settings: { title: "Settings", description: "Review the authenticated session, tenant boundary, and operational support references.", capability: "access.control" },
};
