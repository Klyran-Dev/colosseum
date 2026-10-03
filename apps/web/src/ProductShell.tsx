import { ReactNode, useMemo, useState } from "react";
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
import { ProductWorkspace } from "./navigation";

export type { ProductWorkspace } from "./navigation";

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

function ShellStatus({ serviceState }: { serviceState: ServiceState }) {
  const label = { checking: "Checking services", online: "All systems operational", offline: "Some services unavailable" }[serviceState];
  return <span className={`shell-service shell-service-${serviceState}`} role="status" aria-live="polite"><span className="status-dot" />{label}</span>;
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
  children,
}: ProductShellProps) {
  const [mobileNavigationOpen, setMobileNavigationOpen] = useState(false);
  const direction = directionForLocale(locale);
  const current = NAVIGATION.find((item) => item.workspace === workspace) ?? NAVIGATION[0];
  const derivedSnapshot = useMemo(() => authorizationSnapshotForToken(accessToken), [accessToken]);
  const snapshot: AuthorizationSnapshot = authorizationSnapshot ?? derivedSnapshot;
  const accessLabel = !accessToken ? "Sign-in needed" : authorizationError ? "Access check unavailable" : snapshot.state === "server-allowed" ? "Access checked" : "Checking access";
  const accessSubtext = !accessToken ? "Sign in to continue" : authorizationError ? "Protected actions stay unavailable" : snapshot.state === "server-allowed" ? "Confirmed by the service" : "Waiting for the service";
  const accountLabel = snapshot.state === "server-allowed" && snapshot.tenantId ? `Account ${snapshot.tenantId.slice(0, 8)}` : "Sign-in required";
  const accountSubtext = snapshot.state === "server-allowed" && snapshot.tenantId ? "Account confirmed by the service" : "Account context appears after sign-in";
  const contextWarning = runtimeContext.warnings.join(" ");
  const groupedItems = (group: WorkspaceGroup) => NAVIGATION.filter((item) => item.group === group);
  const effectiveTimeZone = isValidTimeZone(timeZone) ? timeZone : DEFAULT_TIME_ZONE;

  function navigateToWorkspace(nextWorkspace: ProductWorkspace) {
    setMobileNavigationOpen(false);
    onNavigate(nextWorkspace);
  }

  return (
    <div className="app-shell product-shell" dir={direction}>
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
            <div className="nav-group" key={group}>
              <p className="nav-group-label">{group}</p>
              {groupedItems(group).map((item) => {
                const decision = navigationAccess(item, snapshot);
                const accessLabelForItem = navigationAccessLabel(decision, Boolean(accessToken));
                return <button
                  type="button"
                  key={item.workspace}
                  className={`nav-item ${workspace === item.workspace ? "nav-item-active" : ""}`}
                  aria-current={workspace === item.workspace ? "page" : undefined}
                  aria-label={item.label}
                  title={`${item.description} ${accessLabelForItem}: ${decision.reason}`}
                  onClick={() => navigateToWorkspace(item.workspace)}
                >
                  <span className="nav-item-icon" aria-hidden="true">{item.label.slice(0, 1)}</span>
                  <span className="nav-item-copy"><strong>{item.label}</strong><small>{item.description}</small></span>
                  <span className={`nav-policy-mark nav-policy-${decision.kind}`} aria-label={`${item.label}: ${accessLabelForItem}`} title={decision.reason}>{accessLabelForItem}</span>
                  {item.risk !== "read" && <span className={`nav-risk nav-risk-${item.risk}`} title={item.risk === "financial" ? "Sensitive financial action" : item.risk === "governance" ? "Approval may be required" : "Change action"} aria-label={item.risk === "financial" ? "Sensitive financial action" : item.risk === "governance" ? "Approval may be required" : "Change action"}>•</span>}
                </button>;
              })}
            </div>
          ))}
        </nav>
        <div className="sidebar-footer">
          <div className={`identity-state identity-${snapshot.state}`}>
            <span className="identity-avatar" aria-hidden="true">{accessToken ? "✓" : "?"}</span>
            <span><strong>{accessLabel}</strong><small>{accessSubtext}</small></span>
          </div>
          <small className="sidebar-legal">Tax withholding is not tax filing or legal advice.</small>
        </div>
      </aside>

      <div className="app-main-column">
        <header className="topbar product-topbar">
          <div className="breadcrumb" aria-label="Breadcrumb"><span>Operations</span><span aria-hidden="true">/</span><strong>{current.label}</strong></div>
          <div className="topbar-actions">
            <span className={`environment-badge ${runtimeContext.environment.configured ? "" : "context-unconfigured"}`} title={runtimeContext.environment.configured ? undefined : contextWarning}><span className="environment-dot" />{runtimeContext.environment.label}</span>
            <span className={`network-badge ${runtimeContext.network.configured ? "" : "context-unconfigured"}`} title={runtimeContext.network.configured ? undefined : contextWarning}>{runtimeContext.network.label}</span>
            <ShellStatus serviceState={serviceState} />
            <details className="display-settings">
              <summary aria-label="Open display settings">Display settings</summary>
              <div className="display-settings-menu">
                <label><span>Language</span><select value={locale} onChange={(event) => onLocaleChange(event.target.value as SupportedLocale)}>{SUPPORTED_LOCALES.map((option) => <option value={option} key={option}>{option}</option>)}</select></label>
                <label><span>Time zone</span><input value={timeZone} onChange={(event) => onTimeZoneChange(event.target.value)} list="supported-time-zones" aria-invalid={!isValidTimeZone(timeZone)} /></label>
                <small className="field-help">Dates and times are shown in {effectiveTimeZone}. Currency values follow the selected language and account settings.</small>
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
            <button type="button" className="avatar-button" aria-label="Open signed-in account menu">{snapshot.state === "server-allowed" ? "✓" : "?"}</button>
          </div>
        </header>

        <main className="content product-content" id="main-content">
          <section className="page-heading" aria-labelledby="page-title">
            <div><p className="eyebrow">{current.group} / your work area</p><h1 id="page-title">{current.label}</h1><p className="page-description">{current.description}</p></div>
            <div className="page-context-card"><span className="context-card-label">Your access</span><strong>{accessLabel}</strong><small>{authorizationError ?? "Protected actions may need review or approval."}</small></div>
          </section>
          <section className="security-strip" aria-label="Security context">
            <span><i className="strip-icon">✓</i> Account access checked</span>
            <span><i className="strip-icon">✓</i> Every decision has a record</span>
            <span><i className="strip-icon">✓</i> Sensitive actions need approval</span>
            <span><i className="strip-icon">✓</i> Wallets stay with their owners</span>
          </section>
          {contextWarning && <p className="context-warning" role="alert">{contextWarning} This display context cannot authorize actions.</p>}
          {children}
        </main>
        <footer className="footer"><span>Solana Tax Protocol · tax operations</span><span>Every important action has a traceable record.</span></footer>
      </div>
    </div>
  );
}
