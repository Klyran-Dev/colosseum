import { ReactNode, useEffect, useMemo, useState } from "react";
import {
  ActionRisk,
  AuthorizationSnapshot,
  UiAccessDecision,
  WorkspaceGroup,
  authorizationSnapshotForToken,
  evaluateUiAccess,
  WORKSPACE_POLICIES,
} from "./authorization";
import { DEFAULT_TIME_ZONE, directionForLocale, isValidTimeZone, SUPPORTED_LOCALES, SupportedLocale } from "./formatters";
import { BROWSER_RUNTIME_CONTEXT, RuntimeContext } from "./runtimeContext";
import AuthSessionControl, { AuthSessionStatus } from "./AuthSessionControl";
import { BROWSER_AUTH_CONFIG, BrowserAuthConfig } from "./authConfig";
import { pathForWorkspace, ProductWorkspace } from "./navigation";
import { DisplayTheme } from "./displayPreferences";

export type { ProductWorkspace } from "./navigation";
export type ThemePreference = DisplayTheme;

type ServiceState = "checking" | "online" | "offline";

interface NavigationItem {
  workspace: ProductWorkspace;
  capabilityId: string;
  label: string;
  description: string;
  group: WorkspaceGroup;
  risk: ActionRisk;
}

export interface ProductShellProps {
  workspace: ProductWorkspace;
  onNavigate: (workspace: ProductWorkspace) => void;
  serviceState: ServiceState;
  accessToken: string;
  authorizationSnapshot?: AuthorizationSnapshot;
  authorizationError?: string | null;
  runtimeContext?: RuntimeContext;
  authConfig?: BrowserAuthConfig;
  authSessionStatus?: AuthSessionStatus;
  authSessionError?: string | null;
  onCredentialChange?: (value: string) => void;
  onSignIn?: () => void;
  onSignOut?: () => void;
  locale: SupportedLocale;
  timeZone: string;
  onLocaleChange: (locale: SupportedLocale) => void;
  onTimeZoneChange: (timeZone: string) => void;
  themePreference?: ThemePreference;
  onThemeChange?: (theme: ThemePreference) => void;
  children: ReactNode;
}

const NAVIGATION: readonly NavigationItem[] = [
  { workspace: "quote", capabilityId: "tax.quote.create", label: "Tax quotes", description: "Prepare a customer quote with a clear tax breakdown.", group: "Daily work", risk: "write" },
  { workspace: "preview", capabilityId: "tax.preview.run", label: "Tax simulator", description: "Explore a scenario without changing live rules.", group: "Daily work", risk: "read" },
  { workspace: "document-quote", capabilityId: "tax.document.quote.create", label: "Invoice tax", description: "Calculate tax across an invoice and its lines.", group: "Daily work", risk: "write" },
  { workspace: "checkout", capabilityId: "settlement.wallet", label: "Payment review", description: "Check payment details before the customer signs.", group: "Daily work", risk: "financial" },
  { workspace: "document-checkout", capabilityId: "settlement.document.wallet", label: "Document payment", description: "Review and approve payment for an invoice.", group: "Daily work", risk: "financial" },
  { workspace: "governance", capabilityId: "tax.rule.govern", label: "Tax rules", description: "Create, review, approve, and publish tax rules.", group: "Rules & access", risk: "governance" },
  { workspace: "access-control", capabilityId: "access.control", label: "Access & approvals", description: "See who can act, what requires approval, and why.", group: "Rules & access", risk: "read" },
  { workspace: "vault-release", capabilityId: "vault.release", label: "Treasury", description: "Prepare and review a controlled treasury release.", group: "Money movement", risk: "financial" },
  { workspace: "vault-reconciliation", capabilityId: "vault.reconciliation", label: "Balance checks", description: "Compare recorded balances with the latest verified observation.", group: "Money movement", risk: "read" },
  { workspace: "revenue-streaming", capabilityId: "revenue_stream.read", label: "Partner payouts", description: "Review governed payout plans and prepare a verified release.", group: "Money movement", risk: "financial" },
  { workspace: "adjustments", capabilityId: "settlement.adjustment", label: "Adjustments", description: "Prepare a correction for a completed transaction.", group: "Money movement", risk: "governance" },
  { workspace: "refund-claims", capabilityId: "refund.claim.create", label: "Refund requests", description: "Prepare a refund request for review.", group: "Money movement", risk: "financial" },
  { workspace: "refund-cancellations", capabilityId: "refund.claim.cancel", label: "Refund cancellations", description: "Stop a refund request with an approval record.", group: "Money movement", risk: "governance" },
  { workspace: "refund-executions", capabilityId: "refund.payment.release", label: "Refund payments", description: "Release an approved refund after all checks pass.", group: "Money movement", risk: "financial" },
  { workspace: "replay-ops", capabilityId: "evidence.replay", label: "Exceptions & recovery", description: "Investigate and recover records that need attention.", group: "Records & issues", risk: "write" },
];

const GROUPS: readonly WorkspaceGroup[] = ["Daily work", "Rules & access", "Money movement", "Records & issues"];

function SystemStatus({
  runtimeContext,
  serviceState,
  signedIn,
}: {
  runtimeContext: RuntimeContext;
  serviceState: ServiceState;
  signedIn: boolean;
}) {
  const issues = [
    !runtimeContext.environment.configured,
    !runtimeContext.network.configured,
    serviceState === "offline",
  ].filter(Boolean).length;
  const summary = serviceState === "checking" ? "Checking" : issues === 0 ? "Ready" : "Needs attention";
  const serviceLabel = { checking: "Checking services", online: "All systems operational", offline: "Some services unavailable" }[serviceState];
  const statusClass = serviceState === "checking" ? "checking" : issues === 0 ? "ready" : "attention";

  return <details className={`system-status system-status-${statusClass}`}>
    <summary role="button" aria-label={`System status: ${summary}`}>
      <span className="system-status-indicator" aria-hidden="true"><span className="status-dot" /></span>
      <span>System status</span>
      <strong>{summary}</strong>
      {issues > 0 && <span className="system-status-count" aria-label={`${issues} items need attention`}>{issues}</span>}
    </summary>
    <div className="system-status-panel" aria-label="System status details">
      <div className="system-status-heading"><strong>System status</strong><small>These details describe the current release and service state. They do not grant access.</small></div>
      <div className="system-status-row">
        <span>Deployment environment</span>
        <strong>{runtimeContext.environment.label}</strong>
        <small>{runtimeContext.environment.configured ? "Release context confirmed." : "Ask the release owner to configure this build."}</small>
      </div>
      <div className="system-status-row">
        <span>Solana network</span>
        <strong>{runtimeContext.network.label}</strong>
        <small>{runtimeContext.network.configured ? "Network context confirmed." : "Ask the release owner to configure the target network."}</small>
      </div>
      <div className="system-status-row">
        <span>API services</span>
        <strong>{serviceLabel}</strong>
        <small>{serviceState === "online" ? "The service responded to the latest check." : serviceState === "checking" ? "The service check is still running." : "Try again later; no client-side fallback is used."}</small>
      </div>
      <div className="system-status-row">
        <span>Account session</span>
        <strong>{signedIn ? "Signed in" : "Sign-in required"}</strong>
        <small>{signedIn ? "Protected actions still receive a fresh server decision." : "Sign in to see account actions."}</small>
      </div>
      {runtimeContext.warnings.length > 0 && <p className="system-status-warning" role="alert">{runtimeContext.warnings.join(" ")}</p>}
    </div>
  </details>;
}

function navigationAccess(item: NavigationItem, snapshot: AuthorizationSnapshot): UiAccessDecision {
  const policy = WORKSPACE_POLICIES.find((candidate) => candidate.id === item.capabilityId);
  return policy
    ? evaluateUiAccess(snapshot, policy)
    : { kind: "indeterminate", reason: "The service has not described this workspace." };
}

function navigationAccessLabel(decision: UiAccessDecision, hasToken: boolean): string {
  if (decision.kind === "allow") return "Available";
  if (decision.kind === "deny") return "Not available";
  return hasToken ? "Needs access check" : "Sign in to check";
}

export default function ProductShell({
  workspace,
  onNavigate,
  serviceState,
  accessToken,
  authorizationSnapshot,
  authorizationError,
  runtimeContext = BROWSER_RUNTIME_CONTEXT,
  authConfig = BROWSER_AUTH_CONFIG,
  authSessionStatus = accessToken ? "signed-in" : "signed-out",
  authSessionError,
  onCredentialChange = () => undefined,
  onSignIn = () => undefined,
  onSignOut = () => undefined,
  locale,
  timeZone,
  onLocaleChange,
  onTimeZoneChange,
  themePreference = "system",
  onThemeChange = () => undefined,
  children,
}: ProductShellProps) {
  const [mobileNavigationOpen, setMobileNavigationOpen] = useState(false);
  const [openNavigationGroups, setOpenNavigationGroups] = useState<Set<WorkspaceGroup>>(() => new Set(["Daily work"]));
  const direction = directionForLocale(locale);
  const current = NAVIGATION.find((item) => item.workspace === workspace) ?? NAVIGATION[0];
  const derivedSnapshot = useMemo(() => authorizationSnapshotForToken(accessToken), [accessToken]);
  const snapshot: AuthorizationSnapshot = authorizationSnapshot ?? derivedSnapshot;
  const accessLabel = !accessToken ? "Sign-in needed" : authorizationError ? "Access check unavailable" : snapshot.state === "server-allowed" ? "Access checked" : "Checking access";
  const accessSubtext = !accessToken ? "Sign in to continue" : authorizationError ? "Protected actions stay unavailable" : snapshot.state === "server-allowed" ? "Confirmed by the service" : "Waiting for the service";
  const accountLabel = snapshot.state === "server-allowed" && snapshot.tenantId ? `Account ${snapshot.tenantId.slice(0, 8)}` : "Sign-in required";
  const accountSubtext = snapshot.state === "server-allowed" && snapshot.tenantId ? "Account confirmed by the service" : "Account context appears after sign-in";
  const subjectLabel = snapshot.subject ?? (accessToken ? "Identity not confirmed" : "Not signed in");
  const groupedItems = (group: WorkspaceGroup) => NAVIGATION.filter((item) => item.group === group);
  const effectiveTimeZone = isValidTimeZone(timeZone) ? timeZone : DEFAULT_TIME_ZONE;

  useEffect(() => {
    setOpenNavigationGroups((groups) => {
      if (groups.has(current.group)) return groups;
      const next = new Set(groups);
      next.add(current.group);
      return next;
    });
  }, [current.group]);

  function navigateToWorkspace(nextWorkspace: ProductWorkspace) {
    setMobileNavigationOpen(false);
    onNavigate(nextWorkspace);
  }

  return (
    <div className="app-shell product-shell" data-theme={themePreference} dir={direction}>
      <a className="skip-link" href="#workspace-content">Skip to workspace content</a>
      <aside className="app-sidebar" aria-label="Product navigation">
        <div className="sidebar-header">
          <a className="brand sidebar-brand" href="/" aria-label="Solana Tax Protocol home">
            <span className="brand-mark">S</span>
            <span><strong>Solana Tax Protocol</strong><small>Tax operations</small></span>
          </a>
          <button
            type="button"
            className="mobile-nav-toggle"
            aria-expanded={mobileNavigationOpen}
            aria-controls="primary-navigation"
            onClick={() => setMobileNavigationOpen((open) => !open)}
          >
            {mobileNavigationOpen ? "Close navigation" : "Open navigation"}
          </button>
        </div>
        <div className="sidebar-context">
          <span className="sidebar-context-label">Account</span>
          <strong>{accountLabel}</strong>
          <small>{accountSubtext}</small>
        </div>
        <nav id="primary-navigation" className={`primary-nav ${mobileNavigationOpen ? "primary-nav-open" : ""}`} aria-label="Primary">
          {GROUPS.map((group) => (
            <details
              className="nav-group-disclosure"
              key={group}
              open={current.group === group || openNavigationGroups.has(group)}
              onToggle={(event) => {
                if (event.currentTarget.open || current.group === group) {
                  setOpenNavigationGroups((groups) => new Set(groups).add(group));
                } else {
                  setOpenNavigationGroups((groups) => {
                    const next = new Set(groups);
                    next.delete(group);
                    return next;
                  });
                }
              }}
            >
              <summary className="nav-group-summary">
                <span className="nav-group-label">{group}</span>
                <span className="nav-group-count" aria-label={`${groupedItems(group).length} workspaces`}>{groupedItems(group).length}</span>
              </summary>
              <div className="nav-group" role="group" aria-label={group}>
                {groupedItems(group).map((item) => {
                const decision = navigationAccess(item, snapshot);
                const accessLabelForItem = navigationAccessLabel(decision, Boolean(accessToken));
                return <button
                  type="button"
                  key={item.workspace}
                  className={`nav-item ${workspace === item.workspace ? "nav-item-active" : ""}`}
                  aria-current={workspace === item.workspace ? "page" : undefined}
                  aria-label={item.label}
                  aria-describedby={`nav-${item.workspace}-context`}
                  title={`${item.description} ${accessLabelForItem}: ${decision.reason}`}
                  onClick={() => navigateToWorkspace(item.workspace)}
                >
                  <span className="nav-item-icon" aria-hidden="true">{item.label.slice(0, 1)}</span>
                  <span className="nav-item-copy"><strong>{item.label}</strong><span id={`nav-${item.workspace}-context`} className="nav-item-description sr-only">{item.description} Access: {accessLabelForItem}. {decision.reason}</span></span>
                  <span className={`nav-policy-mark nav-policy-${decision.kind}`} aria-hidden="true" title={`${accessLabelForItem}: ${decision.reason}`}>{decision.kind === "allow" ? "✓" : decision.kind === "deny" ? "!" : "?"}</span>
                  {item.risk !== "read" && <span className={`nav-risk nav-risk-${item.risk}`} title={item.risk === "financial" ? "Sensitive financial action" : item.risk === "governance" ? "Approval may be required" : "Change action"} aria-hidden="true">•</span>}
                </button>;
                })}
              </div>
            </details>
          ))}
        </nav>
        <div className="sidebar-footer">
          <div className={`identity-state identity-${snapshot.state}`}>
            <span className="identity-avatar" aria-hidden="true">{accessToken ? "✓" : "?"}</span>
            <span><strong>{accessLabel}</strong><small>{accessSubtext}</small></span>
          </div>
          <small className="sidebar-legal">Automated withholding and evidence—not tax filing or legal advice.</small>
        </div>
      </aside>

      <div className="app-main-column">
        <header className="topbar product-topbar">
          <div className="breadcrumb" aria-label="Breadcrumb"><span>Operations</span><span aria-hidden="true">/</span><strong>{current.label}</strong></div>
          <div className="topbar-actions">
            <SystemStatus runtimeContext={runtimeContext} serviceState={serviceState} signedIn={Boolean(accessToken)} />
            <details className="display-settings">
              <summary role="button" aria-label="Open display settings">Display settings</summary>
              <div className="display-settings-menu">
                <label><span>Language</span><select value={locale} onChange={(event) => onLocaleChange(event.target.value as SupportedLocale)}>{SUPPORTED_LOCALES.map((option) => <option value={option} key={option}>{option}</option>)}</select></label>
                <label><span>Time zone</span><input value={timeZone} onChange={(event) => onTimeZoneChange(event.target.value)} list="supported-time-zones" aria-invalid={!isValidTimeZone(timeZone)} /></label>
                <label><span>Theme</span><select aria-label="Theme" value={themePreference} onChange={(event) => onThemeChange(event.target.value as ThemePreference)}><option value="system">System default</option><option value="light">Light</option><option value="dark">Dark</option></select></label>
                <small className="field-help">Dates and times are shown in {effectiveTimeZone}. Currency values follow the selected language and account settings.</small>
                <small className="field-help">Theme changes appearance only.</small>
                <datalist id="supported-time-zones"><option value="UTC" /><option value="Asia/Kolkata" /><option value="America/Los_Angeles" /><option value="Europe/Berlin" /></datalist>
                {!isValidTimeZone(timeZone) && <small className="inline-warning">Unknown time zone; UTC is used for rendering.</small>}
              </div>
            </details>
            <AuthSessionControl
              config={authConfig}
              accessToken={accessToken}
              status={authSessionStatus}
              error={authSessionError}
              onCredentialChange={onCredentialChange}
              onSignIn={onSignIn}
              onSignOut={onSignOut}
            />
            <details className="account-menu">
              <summary className="account-menu-trigger" role="button" aria-label="Open account menu">
                <span className="avatar-button" aria-hidden="true">{snapshot.state === "server-allowed" ? "✓" : "?"}</span>
              </summary>
              <div className="account-menu-panel">
                <div className="account-menu-heading">
                  <div><span className="account-menu-label">Account</span><strong>{accountLabel}</strong></div>
                  <span className={`status-pill ${snapshot.state === "server-allowed" ? "" : "status-pill-warning"}`}>{accessLabel}</span>
                </div>
                <p className="account-menu-copy">{accountSubtext}. This menu shows session context; it cannot grant access or approve an action.</p>
                <dl className="account-menu-facts">
                  <div><dt>Signed in as</dt><dd>{subjectLabel}</dd></div>
                  <div><dt>Access check</dt><dd>{accessSubtext}</dd></div>
                </dl>
                <div className="account-menu-actions">
                  <a
                    href={pathForWorkspace("access-control")}
                    onClick={(event) => {
                      event.preventDefault();
                      event.currentTarget.closest("details")?.removeAttribute("open");
                      navigateToWorkspace("access-control");
                    }}
                  >Review access &amp; approvals</a>
                  {accessToken ? <button
                    type="button"
                    className="secondary-button account-menu-signout"
                    onClick={(event) => {
                      event.currentTarget.closest("details")?.removeAttribute("open");
                      onSignOut();
                    }}
                  >Sign out</button> : <small className="account-menu-note">Sign in to manage this account.</small>}
                </div>
              </div>
            </details>
          </div>
        </header>

        <main className="content product-content" id="main-content">
          <section className="page-heading" aria-labelledby="page-title">
            <div><p className="eyebrow">{current.group} / your work area</p><h1 id="page-title">{current.label}</h1><p className="page-description">{current.description}</p></div>
            <div className="page-context-card">
              <span className="context-card-label">Your access</span>
              <strong>{accessLabel}</strong>
              <small>{authorizationError ?? "Protected actions may need review or approval."}</small>
              <details className="protection-details">
                <summary role="button" aria-label="Open protection details">Protection details</summary>
                <div className="protection-details-panel" aria-label="Protection details">
                  <div><strong>Account access</strong><small>{accessLabel}. {accessSubtext}.</small></div>
                  <div><strong>Decision records</strong><small>Every decision has a record.</small></div>
                  <div><strong>Sensitive actions</strong><small>Sensitive actions need approval.</small></div>
                  <div><strong>Wallet custody</strong><small>Wallets stay with their owners.</small></div>
                </div>
              </details>
            </div>
          </section>
          {children}
        </main>
        <footer className="footer"><span>Solana Tax Protocol · tax operations</span><span>Every important action has a traceable record.</span></footer>
      </div>
    </div>
  );
}
