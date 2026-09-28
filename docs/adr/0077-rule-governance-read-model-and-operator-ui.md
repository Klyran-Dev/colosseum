# ADR-0077: Rule-governance read model and operator workspace

- Status: Accepted
- Date: 2026-09-26
- Owners: Tax Platform / Product / Security
- Scope: Authenticated tax-rule governance evidence retrieval and SPA operator UX

## Context

ADR-0076 introduced a write-side transition endpoint, but a production
operator cannot safely promote a rule from a form that has no server-owned
current state or evidence history. The UI must show the current lifecycle,
recorded duties, governance revision, and append-only event trail before an
operator acts. It must also make clear that the browser is not an authority:
roles are enforced by the API and database, and a stale view must fail closed.

## Decision

Add an authenticated read endpoint:

`GET /v1/tax/rules/{rule_set_id}/governance`

It requires the exact `tax:rule:read` scope and one of the explicitly allowed
tax-rule governance roles: `tax_rule_author`, `tax_rule_reviewer`,
`tax_rule_approver`, or `tax_rule_auditor`. The response is a consistent
database snapshot containing rule identity, lifecycle status, assigned duty
subjects, governance revision, effective window, and ordered append-only
events. Rule documents and evidence are returned only through this
authenticated control-plane boundary.

The React/Vite SPA adds a Rule governance workspace that:

- loads the real snapshot with a bounded timeout;
- derives displayable next actions from the returned status only;
- submits the selected action and bounded JSON evidence to ADR-0076's API;
- reloads the snapshot after a successful transition;
- displays authorization failure, stale/conflict, unavailable, timeout,
  malformed-evidence, and offline states with actionable text; and
- keeps the bearer token in memory only and never handles signing keys.

The workspace is an operator surface, not a legal/tax approval substitute.
The server remains the authority for role membership, transition validity,
subject separation, revision fencing, and append-only evidence.

## Alternatives considered

- **Client-only lifecycle state**: rejected because it can display or submit
  stale/fabricated status and cannot provide an auditable evidence trail.
- **Return only the current status**: rejected because reviewers need duty
  assignments, revision, effective window, and the ordered decision history.
- **Use a separate event-store read model**: deferred; the governance row and
  immutable events are small control-plane data, and a repeatable-read
  PostgreSQL snapshot avoids projection lag without putting this workflow in
  the settlement hot path.
- **Use a browser-decoded JWT to enable actions**: rejected because claims
  are untrusted for authority; the API performs every scope and role check.
- **Build a second server-rendered UI**: rejected by the React/Vite SPA and
  NGINX constraint and unnecessary for this authenticated operator surface.

## Consequences

Operators gain a real evidence-first workflow for rule review and publication.
The read path adds bounded control-plane database work but is not used by
quote or settlement hot paths. The UI must handle expired credentials and
concurrent changes honestly. Production still requires identity-provider role
provisioning, legal/tax ownership, audit access policy, and browser-level
accessibility testing in the target deployment.
