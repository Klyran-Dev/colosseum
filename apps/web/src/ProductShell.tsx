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
import { shellMessagesFor } from "./shellMessages";
import { shellOperationalMessagesFor, ShellOperationalMessages } from "./shellOperationalMessages";

export type { ProductWorkspace } from "./navigation";
export type ThemePreference = DisplayTheme;

type ServiceState = "checking" | "online" | "offline";

interface NavigationItem {
  workspace: ProductWorkspace;
  icon: NavigationIcon;
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

type NavigationIcon =
  | "quote"
  | "simulate"
  | "invoice"
  | "payment"
  | "document-payment"
  | "rules"
  | "access"
  | "evidence"
  | "treasury"
  | "balance"
  | "payouts"
  | "adjustment"
  | "refund"
  | "cancel"
  | "refund-payment"
  | "recovery";

function NavigationGlyph({ icon }: { icon: NavigationIcon }) {
  const attributes = {
    className: "nav-item-glyph",
    viewBox: "0 0 24 24",
    fill: "none",
    stroke: "currentColor",
    strokeWidth: 1.8,
    strokeLinecap: "round" as const,
    strokeLinejoin: "round" as const,
    "aria-hidden": true,
    focusable: false,
  };

  switch (icon) {
    case "quote":
      return <svg {...attributes}><path d="M5 4.5h10l4 4v11H5z" /><path d="M15 4.5v4h4M8 13h8M8 16.5h5" /></svg>;
    case "simulate":
      return <svg {...attributes}><path d="M9 3.5h6M10 3.5v5.2l-4.2 7.4A3 3 0 0 0 8.4 20.5h7.2a3 3 0 0 0 2.6-4.4L14 8.7V3.5" /><path d="M8.1 15h7.8" /></svg>;
    case "invoice":
      return <svg {...attributes}><path d="M6 3.5h9l3 3v14H6z" /><path d="M15 3.5v3h3M8.5 11h7M8.5 14.5h7M8.5 18h4" /></svg>;
    case "payment":
      return <svg {...attributes}><rect x="3.5" y="5.5" width="17" height="13" rx="2" /><path d="M3.5 9.5h17M7 14h3" /></svg>;
    case "document-payment":
      return <svg {...attributes}><path d="M5 3.5h9l4 4v13H5z" /><path d="M14 3.5v4h4M8 13l2 2 5-5" /></svg>;
    case "rules":
      return <svg {...attributes}><path d="M12 3.5 19 6v5.1c0 4.3-2.7 7.7-7 9.4-4.3-1.7-7-5.1-7-9.4V6z" /><path d="m8.5 12 2.2 2.2 4.8-4.8" /></svg>;
    case "access":
      return <svg {...attributes}><circle cx="9" cy="8" r="3" /><path d="M3.8 19.5a5.2 5.2 0 0 1 10.4 0M15 6.5a3 3 0 0 1 0 5.8M16.2 14.5a5.2 5.2 0 0 1 4 5" /></svg>;
    case "evidence":
      return <svg {...attributes}><path d="M5 4.5h14v15H5z" /><path d="M8 8h8M8 11.5h8M8 15h5" /><path d="m16 16 2.5 2.5" /></svg>;
    case "treasury":
      return <svg {...attributes}><path d="M4 9.5 12 4l8 5.5" /><path d="M5.5 10.5h13v8.5h-13zM8 10.5v8.5M12 10.5v8.5M16 10.5v8.5M4 20.5h16" /></svg>;
    case "balance":
      return <svg {...attributes}><path d="M12 4v16M5 7h14M7.5 7 4 13h7zM16.5 7 13 13h7zM5 20h14" /></svg>;
    case "payouts":
      return <svg {...attributes}><path d="M4 7.5h16M6 4.5h12a2 2 0 0 1 2 2v11a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2v-11a2 2 0 0 1 2-2z" /><path d="m8 13 3 3 5-5" /></svg>;
    case "adjustment":
      return <svg {...attributes}><path d="M5 6h14M5 12h14M5 18h14" /><circle cx="9" cy="6" r="2" /><circle cx="15" cy="12" r="2" /><circle cx="11" cy="18" r="2" /></svg>;
    case "refund":
      return <svg {...attributes}><path d="M8 7H4v4" /><path d="M4.5 11a7.5 7.5 0 1 0 2.2-5.3L4 7.8" /><path d="M12 9v6M9.5 12h5" /></svg>;
    case "cancel":
      return <svg {...attributes}><circle cx="12" cy="12" r="8.5" /><path d="m9 9 6 6M15 9l-6 6" /></svg>;
    case "refund-payment":
      return <svg {...attributes}><path d="M12 4v12M7 11l5 5 5-5" /><path d="M5 20h14" /></svg>;
    case "recovery":
      return <svg {...attributes}><path d="M4 12a8 8 0 1 0 2.3-5.7L4 8.5" /><path d="M4 4v4.5h4.5" /><path d="M12 8v4l2.5 1.5" /></svg>;
  }
}

const NAVIGATION: readonly NavigationItem[] = [
  { workspace: "quote", icon: "quote", capabilityId: "tax.quote.create", label: "Tax quotes", description: "Prepare a customer quote with a clear tax breakdown.", group: "Daily work", risk: "write" },
  { workspace: "preview", icon: "simulate", capabilityId: "tax.preview.run", label: "Tax simulator", description: "Explore a scenario without changing live rules.", group: "Daily work", risk: "read" },
  { workspace: "document-quote", icon: "invoice", capabilityId: "tax.document.quote.create", label: "Invoice tax", description: "Calculate tax across an invoice and its lines.", group: "Daily work", risk: "write" },
  { workspace: "checkout", icon: "payment", capabilityId: "settlement.wallet", label: "Payment review", description: "Check payment details before the customer signs.", group: "Daily work", risk: "financial" },
  { workspace: "document-checkout", icon: "document-payment", capabilityId: "settlement.document.wallet", label: "Document payment", description: "Review and approve payment for an invoice.", group: "Daily work", risk: "financial" },
  { workspace: "governance", icon: "rules", capabilityId: "tax.rule.govern", label: "Tax rules", description: "Create, review, approve, and publish tax rules.", group: "Rules & access", risk: "governance" },
  { workspace: "access-control", icon: "access", capabilityId: "access.control", label: "Access & approvals", description: "See who can act, what requires approval, and why.", group: "Rules & access", risk: "read" },
  { workspace: "vault-release", icon: "treasury", capabilityId: "vault.release", label: "Treasury", description: "Prepare and review a controlled treasury release.", group: "Money movement", risk: "financial" },
  { workspace: "vault-reconciliation", icon: "balance", capabilityId: "vault.reconciliation", label: "Balance checks", description: "Compare recorded balances with the latest verified observation.", group: "Money movement", risk: "read" },
  { workspace: "revenue-streaming", icon: "payouts", capabilityId: "revenue_stream.read", label: "Partner payouts", description: "Review governed payout plans and prepare a verified release.", group: "Money movement", risk: "financial" },
  { workspace: "adjustments", icon: "adjustment", capabilityId: "settlement.adjustment", label: "Adjustments", description: "Prepare a correction for a completed transaction.", group: "Money movement", risk: "governance" },
  { workspace: "refund-claims", icon: "refund", capabilityId: "refund.claim.create", label: "Refund requests", description: "Prepare a refund request for review.", group: "Money movement", risk: "financial" },
  { workspace: "refund-cancellations", icon: "cancel", capabilityId: "refund.claim.cancel", label: "Refund cancellations", description: "Stop a refund request with an approval record.", group: "Money movement", risk: "governance" },
  { workspace: "refund-executions", icon: "refund-payment", capabilityId: "refund.payment.release", label: "Refund payments", description: "Release an approved refund after all checks pass.", group: "Money movement", risk: "financial" },
  { workspace: "replay-ops", icon: "recovery", capabilityId: "evidence.replay", label: "Exceptions & recovery", description: "Investigate and recover records that need attention.", group: "Records & issues", risk: "write" },
  { workspace: "evidence-access", icon: "evidence", capabilityId: "evidence.audit.read", label: "Evidence & audit", description: "Review protected evidence access activity for this account.", group: "Records & issues", risk: "read" },
];

const GROUPS: readonly WorkspaceGroup[] = ["Daily work", "Rules & access", "Money movement", "Records & issues"];
const UNKNOWN_WORKSPACE_POLICY_REASON = "The service has not described this workspace.";

function SystemStatus({
  runtimeContext,
  serviceState,
  signedIn,
  messages,
}: {
  runtimeContext: RuntimeContext;
  serviceState: ServiceState;
  signedIn: boolean;
  messages: ShellOperationalMessages;
}) {
  const issues = [
    !runtimeContext.environment.configured,
    !runtimeContext.network.configured,
    serviceState === "offline",
  ].filter(Boolean).length;
  const summary = serviceState === "checking" ? messages.system.checking : issues === 0 ? messages.system.ready : messages.system.attention;
  const serviceLabel = { checking: messages.system.checkingServices, online: messages.system.operational, offline: messages.system.unavailable }[serviceState];
  const statusClass = serviceState === "checking" ? "checking" : issues === 0 ? "ready" : "attention";

  return <details className={`system-status system-status-${statusClass}`}>
    <summary role="button" aria-label={`${messages.system.label}: ${summary}`}>
      <span className="system-status-indicator" aria-hidden="true"><span className="status-dot" /></span>
      <span>{messages.system.label}</span>
      <strong>{summary}</strong>
      {issues > 0 && <span className="system-status-count" aria-label={messages.system.itemsNeedAttention(issues)}>{issues}</span>}
    </summary>
    <div className="system-status-panel" aria-label={messages.system.details}>
      <div className="system-status-heading"><strong>{messages.system.label}</strong><small>{messages.system.contextNote}</small></div>
      <div className="system-status-row">
        <span>{messages.system.deployment}</span>
        <strong>{runtimeContext.environment.label}</strong>
        <small>{runtimeContext.environment.configured ? messages.system.confirmed : messages.system.configureBuild}</small>
      </div>
      <div className="system-status-row">
        <span>{messages.system.network}</span>
        <strong>{runtimeContext.network.label}</strong>
        <small>{runtimeContext.network.configured ? messages.system.confirmed : messages.system.configureNetwork}</small>
      </div>
      <div className="system-status-row">
        <span>{messages.system.api}</span>
        <strong>{serviceLabel}</strong>
        <small>{serviceState === "online" ? messages.system.serviceResponded : serviceState === "checking" ? messages.system.serviceChecking : messages.system.retryLater}</small>
      </div>
      <div className="system-status-row">
        <span>{messages.system.session}</span>
        <strong>{signedIn ? messages.system.signedIn : messages.system.signInRequired}</strong>
        <small>{signedIn ? messages.system.protectedRefresh : messages.system.signInToSee}</small>
      </div>
      {runtimeContext.warnings.length > 0 && <p className="system-status-warning">{runtimeContext.warnings.join(" ")}</p>}
    </div>
  </details>;
}

function navigationAccess(item: NavigationItem, snapshot: AuthorizationSnapshot): UiAccessDecision {
  const policy = WORKSPACE_POLICIES.find((candidate) => candidate.id === item.capabilityId);
  return policy
    ? evaluateUiAccess(snapshot, policy)
    : { kind: "indeterminate", reasonCode: "unknown_policy", reason: UNKNOWN_WORKSPACE_POLICY_REASON };
}

function navigationAccessLabel(decision: UiAccessDecision, hasToken: boolean, messages: ReturnType<typeof shellMessagesFor>): string {
  if (decision.kind === "allow") return messages.accessStatus.available;
  if (decision.kind === "deny") return messages.accessStatus.unavailable;
  return hasToken ? messages.accessStatus.needsCheck : messages.accessStatus.signInToCheck;
}

function navigationAccessReason(decision: UiAccessDecision, messages: ReturnType<typeof shellMessagesFor>): string {
  return messages.accessReason(decision.reasonCode);
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
  const [navigationCompact, setNavigationCompact] = useState(false);
  const direction = directionForLocale(locale);
  const current = NAVIGATION.find((item) => item.workspace === workspace) ?? NAVIGATION[0];
  const messages = shellMessagesFor(locale);
  const operational = shellOperationalMessagesFor(locale);
  const currentMessage = messages.navigation[current.workspace];
  const [openNavigationGroup, setOpenNavigationGroup] = useState<WorkspaceGroup>(current.group);
  const derivedSnapshot = useMemo(() => authorizationSnapshotForToken(accessToken), [accessToken]);
  const snapshot: AuthorizationSnapshot = authorizationSnapshot ?? derivedSnapshot;
  const accessLabel = !accessToken ? operational.account.accessNeeded : authorizationError ? operational.account.accessUnavailable : snapshot.state === "server-allowed" ? operational.account.accessChecked : operational.account.accessChecking;
  const accessSubtext = !accessToken ? operational.account.continueAfterSignIn : authorizationError ? operational.account.protectedUnavailable : snapshot.state === "server-allowed" ? operational.system.confirmed : operational.account.waitingForService;
  const accountLabel = snapshot.state === "server-allowed" && snapshot.tenantId ? `${messages.chrome.account} ${snapshot.tenantId.slice(0, 8)}` : operational.account.signInRequired;
  const accountSubtext = snapshot.state === "server-allowed" && snapshot.tenantId ? operational.account.confirmed : operational.account.contextAfterSignIn;
  const subjectLabel = snapshot.subject ?? (accessToken ? operational.account.identityNotConfirmed : operational.account.notSignedIn);
  const groupedItems = (group: WorkspaceGroup) => NAVIGATION.filter((item) => item.group === group);
  const effectiveTimeZone = isValidTimeZone(timeZone) ? timeZone : DEFAULT_TIME_ZONE;

  useEffect(() => {
    setOpenNavigationGroup(current.group);
  }, [current.group]);

  function navigateToWorkspace(nextWorkspace: ProductWorkspace) {
    setMobileNavigationOpen(false);
    const nextGroup = NAVIGATION.find((item) => item.workspace === nextWorkspace)?.group ?? current.group;
    setOpenNavigationGroup(nextGroup);
    onNavigate(nextWorkspace);
  }

  return (
    <div className={`app-shell product-shell${navigationCompact ? " navigation-compact" : ""}`} data-theme={themePreference} dir={direction}>
      <a className="skip-link" href="#main-content">{messages.chrome.skipToWorkspace}</a>
      <aside className="app-sidebar" aria-label={messages.chrome.productNavigation}>
        <div className="sidebar-header">
          <a className="brand sidebar-brand" href="/" aria-label="Solana Tax Protocol home">
            <span className="brand-mark">S</span>
            <span><strong>Solana Tax Protocol</strong><small>{operational.footer.operations}</small></span>
          </a>
          <button
            type="button"
            className="mobile-nav-toggle"
            aria-expanded={mobileNavigationOpen}
            aria-controls="primary-navigation"
            onClick={() => setMobileNavigationOpen((open) => !open)}
          >
            {mobileNavigationOpen ? messages.chrome.closeNavigation : messages.chrome.openNavigation}
          </button>
          <button
            type="button"
            className="navigation-compact-toggle"
            aria-label={navigationCompact ? messages.chrome.expandNavigation : messages.chrome.collapseNavigation}
            aria-pressed={navigationCompact}
            title={navigationCompact ? messages.chrome.expandNavigation : messages.chrome.collapseNavigation}
            onClick={() => setNavigationCompact((compact) => !compact)}
          >
            <span aria-hidden="true">{navigationCompact ? "→" : "←"}</span>
          </button>
        </div>
        <nav id="primary-navigation" className={`primary-nav ${mobileNavigationOpen ? "primary-nav-open" : ""}`} aria-label={messages.chrome.primaryNavigation}>
          {GROUPS.map((group) => (
            <details
              className="nav-group-disclosure"
              key={group}
              open={openNavigationGroup === group}
            >
              <summary
                className="nav-group-summary"
                onClick={(event) => {
                  // Keep the native disclosure semantics while making the
                  // controlled single-open invariant deterministic across
                  // browser toggle timing and keyboard activation.
                  event.preventDefault();
                  setOpenNavigationGroup(openNavigationGroup === group ? current.group : group);
                }}
              >
                <span className="nav-group-label">{messages.groups[group]}</span>
                <span className="nav-group-count" aria-label={messages.chrome.workspaces(groupedItems(group).length)}>{groupedItems(group).length}</span>
              </summary>
              <div className="nav-group" role="group" aria-label={messages.groups[group]}>
                {groupedItems(group).map((item) => {
                const decision = navigationAccess(item, snapshot);
                const accessLabelForItem = navigationAccessLabel(decision, Boolean(accessToken), messages);
                const accessReasonForItem = navigationAccessReason(decision, messages);
                const riskLabelForItem = messages.riskLabels[item.risk];
                const itemMessage = messages.navigation[item.workspace];
                return <button
                  type="button"
                  key={item.workspace}
                  className={`nav-item ${workspace === item.workspace ? "nav-item-active" : ""}`}
                  aria-current={workspace === item.workspace ? "page" : undefined}
                  aria-label={itemMessage.label}
                  aria-describedby={`nav-${item.workspace}-context`}
                  title={`${itemMessage.description} ${accessLabelForItem}: ${accessReasonForItem} ${riskLabelForItem}.`}
                  onClick={() => navigateToWorkspace(item.workspace)}
                >
                  <span className="nav-item-icon"><NavigationGlyph icon={item.icon} /></span>
                  <span className="nav-item-copy"><strong>{itemMessage.label}</strong><span id={`nav-${item.workspace}-context`} className="nav-item-description sr-only">{itemMessage.description} {operational.account.accessPrefix}: {accessLabelForItem}. {accessReasonForItem} {operational.account.sensitivityPrefix}: {riskLabelForItem}.</span></span>
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
          <small className="sidebar-legal">{messages.chrome.sidebarLegal}</small>
        </div>
      </aside>

      <div className="app-main-column">
        <header className="topbar product-topbar">
          <div className="breadcrumb" aria-label={messages.chrome.breadcrumb}><span>{messages.chrome.operations}</span><span aria-hidden="true">/</span><strong>{currentMessage.label}</strong></div>
          <div
            className="topbar-actions"
            onClickCapture={(event) => {
              const target = event.target;
              if (!(target instanceof Element)) return;
              const summary = target.closest("summary");
              const details = summary?.parentElement;
              if (!(details instanceof HTMLDetailsElement) || details.open) return;
              details.parentElement?.querySelectorAll<HTMLDetailsElement>("details[open]").forEach((openDetails) => {
                if (openDetails !== details) openDetails.removeAttribute("open");
              });
            }}
          >
            <SystemStatus runtimeContext={runtimeContext} serviceState={serviceState} signedIn={Boolean(accessToken)} messages={operational} />
            <details className="display-settings">
              <summary role="button" aria-label={messages.chrome.openDisplaySettings}>{messages.chrome.displaySettings}</summary>
              <div className="display-settings-menu">
                <label><span>{messages.chrome.language}</span><select value={locale} onChange={(event) => onLocaleChange(event.target.value as SupportedLocale)}>{SUPPORTED_LOCALES.map((option) => <option value={option} key={option}>{option}</option>)}</select></label>
                <label><span>{messages.chrome.timeZone}</span><input value={timeZone} onChange={(event) => onTimeZoneChange(event.target.value)} list="supported-time-zones" aria-invalid={!isValidTimeZone(timeZone)} /></label>
                <label><span>{messages.chrome.theme}</span><select aria-label={messages.chrome.theme} value={themePreference} onChange={(event) => onThemeChange(event.target.value as ThemePreference)}><option value="system">{messages.chrome.themeSystem}</option><option value="light">{messages.chrome.themeLight}</option><option value="dark">{messages.chrome.themeDark}</option></select></label>
                <small className="field-help">{messages.chrome.dateTimeHelp(effectiveTimeZone)}</small>
                <small className="field-help">{messages.chrome.themeHelp}</small>
                <datalist id="supported-time-zones"><option value="UTC" /><option value="Asia/Kolkata" /><option value="America/Los_Angeles" /><option value="Europe/Berlin" /></datalist>
                {!isValidTimeZone(timeZone) && <small className="inline-warning">{messages.chrome.unknownTimeZone("UTC")}</small>}
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
              locale={locale}
            />
            <details className="account-menu">
              <summary className="account-menu-trigger" role="button" aria-label={operational.account.openMenu}>
                <span className="avatar-button" aria-hidden="true">{snapshot.state === "server-allowed" ? "✓" : "?"}</span>
              </summary>
              <div className="account-menu-panel">
                <div className="account-menu-heading">
                  <div><span className="account-menu-label">{operational.account.label}</span><strong>{accountLabel}</strong></div>
                  <span className={`status-pill ${snapshot.state === "server-allowed" ? "" : "status-pill-warning"}`}>{accessLabel}</span>
                </div>
                <p className="account-menu-copy">{accountSubtext}. {operational.account.copy}</p>
                <dl className="account-menu-facts">
                  <div><dt>{operational.account.signedInAs}</dt><dd>{subjectLabel}</dd></div>
                  <div><dt>{operational.account.accessCheck}</dt><dd>{accessSubtext}</dd></div>
                </dl>
                <div className="account-menu-actions">
                  <a
                    href={pathForWorkspace("access-control")}
                    onClick={(event) => {
                      event.preventDefault();
                      event.currentTarget.closest("details")?.removeAttribute("open");
                      navigateToWorkspace("access-control");
                    }}
                  >{operational.account.reviewAccess}</a>
                  {accessToken ? <button
                    type="button"
                    className="secondary-button account-menu-signout"
                    onClick={(event) => {
                      event.currentTarget.closest("details")?.removeAttribute("open");
                      onSignOut();
                    }}
                  >{operational.account.signOut}</button> : <small className="account-menu-note">{operational.account.signInToManage}</small>}
                </div>
              </div>
            </details>
          </div>
        </header>

        <main className="content product-content" id="main-content" tabIndex={-1}>
          <p className="sr-only" role="status" aria-live="polite" aria-atomic="true">{messages.workspaceStatus(currentMessage.label)}</p>
          <section className="page-heading" aria-labelledby="page-title">
            <div><p className="eyebrow">{messages.groups[current.group]} / {messages.chrome.yourWorkArea}</p><h1 id="page-title">{currentMessage.label}</h1><p className="page-description">{currentMessage.description}</p></div>
            <div className="page-context-card">
              <span className="context-card-label">{operational.protection.yourAccess}</span>
              <strong>{accessLabel}</strong>
              <small>{authorizationError ?? operational.protection.protectedActions}</small>
              <details className="protection-details">
                <summary role="button" aria-label={operational.protection.open}>{operational.protection.label}</summary>
                <div className="protection-details-panel" aria-label={operational.protection.label}>
                  <div><strong>{operational.protection.accountAccess}</strong><small>{accessLabel}. {accessSubtext}.</small></div>
                  <div><strong>{operational.protection.decisionRecords}</strong><small>{operational.protection.everyDecision}</small></div>
                  <div><strong>{operational.protection.sensitiveActions}</strong><small>{operational.protection.sensitiveNeed}</small></div>
                  <div><strong>{operational.protection.walletCustody}</strong><small>{operational.protection.walletsOwner}</small></div>
                </div>
              </details>
            </div>
          </section>
          {children}
        </main>
        <footer className="footer"><span>{operational.footer.product} · {operational.footer.operations}</span><span>{operational.footer.traceable}</span></footer>
      </div>
    </div>
  );
}
