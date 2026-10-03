import { useMemo } from "react";
import { AuthorizationSnapshot, WORKSPACE_POLICIES, WorkspaceGroup, authorizationSnapshotForToken, policiesForGroup } from "./authorization";

interface AccessControlPanelProps {
  accessToken: string;
  authorizationSnapshot?: AuthorizationSnapshot;
  authorizationError?: string | null;
}

const GROUPS: readonly WorkspaceGroup[] = ["Daily work", "Rules & access", "Money movement", "Records & issues"];

function join(values: readonly string[]): string {
  return values.length > 0 ? values.join(", ") : "None";
}

export default function AccessControlPanel({ accessToken, authorizationSnapshot, authorizationError }: AccessControlPanelProps) {
  const derivedSnapshot = useMemo(() => authorizationSnapshotForToken(accessToken), [accessToken]);
  const snapshot = authorizationSnapshot ?? derivedSnapshot;
  const accessStatus = !accessToken ? "Sign in needed" : authorizationError ? "Access check unavailable" : snapshot.state === "server-allowed" ? "Access checked when you act" : "Checking access";

  return <section className="access-control-layout" aria-label="Access and approvals workspace">
    <article className="panel access-control-overview">
      <div className="panel-heading"><div><p className="eyebrow">Access & approvals</p><h2>Who can do what</h2></div><span className={`status-pill ${accessToken && !authorizationError ? "status-pill-warning" : "status-pill-danger"}`}>{accessStatus}</span></div>
      <p className="panel-copy">Your access is checked every time you open a protected action. People can have different responsibilities, some actions need a second approval, and access can depend on the account, environment, and record.</p>
      <div className="authorization-posture" role="status">
        <div><span>Signed-in person</span><strong>{snapshot.subject ?? "Not signed in"}</strong></div>
        <div><span>Account</span><strong>{snapshot.tenantId ? "Confirmed for this account" : "Confirmed when you sign in"}</strong></div>
        <div><span>Access check</span><strong>{authorizationError ? "Unavailable; actions stay closed" : snapshot.state === "server-allowed" ? "Checked by the service" : accessToken ? "Waiting for the service" : "Waiting for sign-in"}</strong></div>
      </div>
      <div className="policy-dimension-grid">
        <article><span className="dimension-icon">R</span><div><strong>Roles</strong><p>Your responsibilities determine which work you can own or approve.</p></div></article>
        <article><span className="dimension-icon">P</span><div><strong>Permissions</strong><p>Each action has a specific permission instead of broad access.</p></div></article>
        <article><span className="dimension-icon">C</span><div><strong>Conditions</strong><p>Account, environment, record status, and risk can change the decision.</p></div></article>
      </div>
      <details className="technical-details"><summary>How access is decided</summary><p><strong>RBAC</strong> checks responsibilities, <strong>PBAC</strong> checks named permissions, and <strong>ABAC</strong> checks the account and record conditions. These are technical names for the three plain-language checks above.</p></details>
      <div className="inline-warning"><strong>Safe by default.</strong> {authorizationError ? "The access check is unavailable, so protected actions stay unavailable." : "If the service cannot confirm access, the action stays unavailable."} No approval, payment, or record is created in the browser.</div>
    </article>
    <article className="panel permission-matrix-panel">
      <div className="panel-heading"><div><p className="eyebrow">Access rules</p><h2>Actions and approvals</h2></div><span className="status-pill">{WORKSPACE_POLICIES.length} rules</span></div>
      <p className="panel-copy">These rules explain which work is available and where another person or a verified record is required.</p>
      <div className="permission-groups">
        {GROUPS.map((group) => <section key={group} className="permission-group" aria-labelledby={`permission-${group}`}>
          <h3 id={`permission-${group}`}>{group}</h3>
          {policiesForGroup(group).map((policy) => {
            const decision = snapshot.capabilities?.[policy.id];
            const decisionLabel = decision?.kind === "allow" ? "Available" : decision?.kind === "deny" ? "Not available" : decision?.kind === "indeterminate" ? "Needs context" : accessToken ? "Checking" : "Sign in to check";
            return <div className="permission-row" key={policy.id}>
            <div><strong>{policy.label}</strong><small>{policy.description}</small></div>
            <div className="permission-requirements"><span className={`decision-chip decision-${decision?.kind ?? "indeterminate"}`}>{decisionLabel}</span><b className={`risk-chip risk-${policy.risk}`}>{policy.risk === "financial" ? "Sensitive" : policy.risk === "governance" ? "Approval" : policy.risk === "write" ? "Change" : "View"}</b></div>
          </div>;
          })}
        </section>)}
      </div>
      <details className="technical-details access-technical-details"><summary>Technical details for administrators</summary><div className="technical-policy-list">{WORKSPACE_POLICIES.map((policy) => <div key={policy.id}><strong>{policy.label}</strong><small><code>{join(policy.requiredScopes)}</code> · {join(policy.requiredRoles)} · {policy.risk}</small></div>)}</div></details>
    </article>
  </section>;
}
