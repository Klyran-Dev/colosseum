import { useMemo } from "react";
import { AuthorizationSnapshot, WORKSPACE_POLICIES, WorkspaceGroup, authorizationSnapshotForToken, policiesForGroup } from "./authorization";
import { DEFAULT_LOCALE } from "./formatters";
import { accessControlMessagesFor } from "./accessControlMessages";

interface AccessControlPanelProps {
  accessToken: string;
  authorizationSnapshot?: AuthorizationSnapshot;
  authorizationError?: string | null;
  locale?: string;
}

const GROUPS: readonly WorkspaceGroup[] = ["Daily work", "Rules & access", "Money movement", "Records & issues"];

function join(values: readonly string[], none: string): string {
  return values.length > 0 ? values.join(", ") : none;
}

export default function AccessControlPanel({ accessToken, authorizationSnapshot, authorizationError, locale = DEFAULT_LOCALE }: AccessControlPanelProps) {
  const messages = accessControlMessagesFor(locale);
  const derivedSnapshot = useMemo(() => authorizationSnapshotForToken(accessToken), [accessToken]);
  const snapshot = authorizationSnapshot ?? derivedSnapshot;
  const accessStatus = !accessToken ? messages.overview.status.signInNeeded : authorizationError ? messages.overview.status.unavailable : snapshot.state === "server-allowed" ? messages.overview.status.checked : messages.overview.status.checking;

  return <section className="access-control-layout" aria-label={messages.ariaLabel}>
    <article className="panel access-control-overview">
      <div className="panel-heading"><div><p className="eyebrow">{messages.overview.eyebrow}</p><h2>{messages.overview.title}</h2></div><span className={`status-pill ${accessToken && !authorizationError ? "status-pill-warning" : "status-pill-danger"}`}>{accessStatus}</span></div>
      <p className="panel-copy">{messages.overview.copy}</p>
      <div className="authorization-posture" role="status">
        <div><span>{messages.overview.person}</span><strong>{snapshot.subject ?? messages.overview.notSignedIn}</strong></div>
        <div><span>{messages.overview.account}</span><strong>{snapshot.tenantId ? messages.overview.accountConfirmed : messages.overview.accountNeedsSignIn}</strong></div>
        <div><span>{messages.overview.permission}</span><strong>{authorizationError ? messages.overview.permissionUnavailable : snapshot.state === "server-allowed" ? messages.overview.permissionChecked : accessToken ? messages.overview.permissionWaiting : messages.overview.permissionSignIn}</strong></div>
      </div>
      <div className="policy-dimension-grid">
        <article><span className="dimension-icon">R</span><div><strong>{messages.overview.dimensions.roles}</strong><p>{messages.overview.dimensions.rolesCopy}</p></div></article>
        <article><span className="dimension-icon">P</span><div><strong>{messages.overview.dimensions.permissions}</strong><p>{messages.overview.dimensions.permissionsCopy}</p></div></article>
        <article><span className="dimension-icon">C</span><div><strong>{messages.overview.dimensions.conditions}</strong><p>{messages.overview.dimensions.conditionsCopy}</p></div></article>
      </div>
      <details className="technical-details"><summary>{messages.overview.technicalSummary}</summary><p>{messages.overview.technicalCopy}</p></details>
      <div className="inline-warning"><strong>{messages.overview.safeTitle}</strong> {authorizationError ? messages.overview.safeUnavailable : messages.overview.safeDefault} {messages.overview.noBrowserAuthority}</div>
    </article>
    <article className="panel permission-matrix-panel">
      <div className="panel-heading"><div><p className="eyebrow">{messages.rules.eyebrow}</p><h2>{messages.rules.title}</h2></div><span className="status-pill">{messages.rules.count(WORKSPACE_POLICIES.length)}</span></div>
      <p className="panel-copy">{messages.rules.copy}</p>
      <div className="permission-groups">
        {GROUPS.map((group) => <section key={group} className="permission-group" aria-labelledby={`permission-${group}`}>
          <h3 id={`permission-${group}`}>{messages.groups[group]}</h3>
          {policiesForGroup(group).map((policy) => {
            const decision = snapshot.capabilities?.[policy.id];
            const decisionLabel = decision?.kind === "allow" ? messages.decisions.available : decision?.kind === "deny" ? messages.decisions.unavailable : decision?.kind === "indeterminate" ? messages.decisions.needsContext : accessToken ? messages.decisions.checking : messages.decisions.signIn;
            const copy = messages.policies[policy.id] ?? { label: policy.label, description: policy.description };
            return <div className="permission-row" key={policy.id}>
            <div><strong>{copy.label}</strong><small>{copy.description}</small></div>
            <div className="permission-requirements"><span className={`decision-chip decision-${decision?.kind ?? "indeterminate"}`}>{decisionLabel}</span><b className={`risk-chip risk-${policy.risk}`}>{policy.risk === "financial" ? messages.risks.financial : policy.risk === "governance" ? messages.risks.governance : policy.risk === "write" ? messages.risks.write : messages.risks.read}</b></div>
          </div>;
          })}
        </section>)}
      </div>
      <details className="technical-details access-technical-details"><summary>{messages.rules.technicalSummary}</summary><div className="technical-policy-list">{WORKSPACE_POLICIES.map((policy) => <div key={policy.id}><strong>{messages.policies[policy.id]?.label ?? policy.label}</strong><small><code>{join(policy.requiredScopes, messages.rules.none)}</code> · {join(policy.requiredRoles, messages.rules.none)} · {policy.risk}</small></div>)}</div></details>
    </article>
  </section>;
}
