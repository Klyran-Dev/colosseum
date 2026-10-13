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

export type PortalMode = "tenant-administrator" | "merchant-operations" | "tenant-member";

export interface PortalIdentity {
  mode: PortalMode;
  label: string;
  eyebrow: string;
  title: string;
  description: string;
  sidebarDescription: string;
}

export interface AccessPostureItem {
  id: "responsibilities" | "workspace" | "policy";
  label: string;
  description: string;
}

export const ACCESS_POSTURE_ITEMS: readonly AccessPostureItem[] = [
  { id: "responsibilities", label: "Your responsibilities", description: "What you are allowed to do in this workspace." },
  { id: "workspace", label: "Workspace boundaries", description: "Which tenant and records this session can reach." },
  { id: "policy", label: "Safety checks", description: "Which approvals and evidence are required before sensitive actions." },
];

/**
 * This is a presentation mapping only. The server capability snapshot remains
 * the authority; an unknown role must never acquire administrator semantics in
 * the browser.
 */
export function portalModeForRoles(roles: readonly string[]): PortalMode {
  if (roles.includes("administrator")) return "tenant-administrator";
  if (roles.includes("merchant_operator")) return "merchant-operations";
  return "tenant-member";
}

export function portalIdentityForRoles(roles: readonly string[]): PortalIdentity {
  switch (portalModeForRoles(roles)) {
    case "tenant-administrator":
      return {
        mode: "tenant-administrator",
        label: "Tenant administrator",
        eyebrow: "Tenant administration",
        title: "Administrator console",
        description: "Manage people, policy, and controlled operations for your tenant.",
        sidebarDescription: "Tenant access and approvals stay server-owned.",
      };
    case "merchant-operations":
      return {
        mode: "merchant-operations",
        label: "Merchant operations",
        eyebrow: "Merchant workspace",
        title: "Merchant workspace",
        description: "Prepare quotes, payments, and evidence for your tenant.",
        sidebarDescription: "Your administrator controls additional responsibilities.",
      };
    default:
      return {
        mode: "tenant-member",
        label: "Tenant member",
        eyebrow: "Tenant workspace",
        title: "Tenant workspace",
        description: "Use the capabilities confirmed for your tenant account.",
        sidebarDescription: "Access is assigned by a tenant administrator.",
      };
  }
}

const NAVIGATION_CAPABILITIES: Readonly<Partial<Record<AdminSection, string>>> = Object.freeze({
  quotes: "tax.quote.create",
  rules: "tax.rule.govern",
  settlements: "settlement.wallet",
  treasury: "vault.release",
  access: "access.control",
  evidence: "evidence.audit.read",
});

type NavigationSnapshot = { capabilities: readonly Pick<AdminCapability, "id" | "decision">[]; expiresAtUnix?: number };

/**
 * Navigation is filtered from the server snapshot for clarity, never for
 * security. Overview and settings remain available as safe recovery surfaces;
 * the current route remains visible so a direct link gets a truthful denial
 * state instead of disappearing during refresh.
 */
export function visibleNavigationForSnapshot(snapshot: NavigationSnapshot | null, currentSection?: AdminSection, nowUnix = Math.floor(Date.now() / 1000)) {
  const expiresAtUnix = snapshot?.expiresAtUnix;
  const snapshotFresh = Boolean(snapshot && typeof expiresAtUnix === "number" && Number.isSafeInteger(expiresAtUnix) && expiresAtUnix > nowUnix);
  return NAV_ITEMS.filter((item) => {
    if (item.section === "overview" || item.section === "settings" || item.section === currentSection) return true;
    if (!snapshot || !snapshotFresh) return false;
    const capabilityId = NAVIGATION_CAPABILITIES[item.section];
    if (!capabilityId) return false;
    return snapshot.capabilities.find((capability) => capability.id === capabilityId)?.decision !== "deny";
  });
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
