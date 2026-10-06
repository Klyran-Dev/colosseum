# ADR-0420 — Dedicated applicability governance workspace

- Status: Accepted — implementation gate
- Date: 2026-10-06
- Decision owners: Product Design, Tax Domain, Security, Principal UI Architecture

## Context

The API now exposes a tenant-scoped applicability-context lifecycle, but the
product surface still presents tax-rule authoring as the only Rules & access
workspace. Adding signed treatment evidence to that workbench would combine
two different authority models: rule-document governance and issuer-signed
tax-treatment evidence. That would increase scan cost, blur responsibility,
and encourage operators to treat browser input as an evidence authority.

## Decision

Add a dedicated `tax-treatment` workspace at `/rules/tax-treatment`. It is an
operator control-plane surface with four bounded areas:

1. Import signed treatment evidence: paste the issuer-signed attestation and
   bounded provenance/effective-window metadata. The browser sends the signed
   object unchanged; it does not calculate or rewrite treatment, facts, or
   digests.
2. Inspect a server-owned context: load only by UUIDv7 through the authenticated
   tenant boundary and show policy, treatment, validity, evidence digests, and
   lifecycle subjects without exposing raw sensitive facts by default.
3. Review and publish: show only lifecycle transitions allowed by the current
   server capability snapshot and let the API enforce the final scope, role,
   four-eyes, and state checks.
4. Explain limitations: state clearly that publication is governance evidence,
   not tax filing, legal compliance, or tax advice.

The workspace uses the existing Material 3 semantic roles and progressive
disclosure patterns. The server remains authoritative for authorization,
tenant identity, attestation verification, effective windows, treatment, and
state transitions.

## Invariants and UX budgets

- No client-side treatment calculation, digest generation, attestation signing,
  tenant selection, or optimistic lifecycle state.
- Every mutation uses the authenticated bearer token and a bounded timeout;
  success is rendered only from the API response reloaded after the mutation.
- Unknown or malformed responses fail closed into a recoverable error state;
  stale loaded state is not presented as current after a failed reload.
- The workspace is keyboard complete, uses semantic labels and status regions,
  supports the existing locale/RTL/theme contracts, and does not introduce raw
  color literals or decorative state colors.
- Initial read/create/transition interaction target: p95 under 150 ms in-region
  excluding identity-provider latency, with no RPC/provider call in the UI.

## Rejected alternatives

- Embedding the flow in the tax-rule workbench: rejected because it collapses
  distinct authority and review tasks into a dense screen.
- Browser-side calculation or attestation editing: rejected because it makes
  the browser an authority and breaks replay guarantees.
- Generic JSON administration: rejected because it hides lifecycle semantics,
  makes RBAC/PBAC/ABAC consequences unclear, and is not a supportable product.

## Verification

Failing-first parser and component tests must cover malformed UUIDv7/digest/
attestation responses, unchanged attestation submission, authenticated
tenant-scoped requests, stale-state clearing, timeout/retry/reload behavior,
and role-dependent transition affordances. The real-browser suite must cover
direct navigation, keyboard focus, responsive layout, RTL, reduced motion,
forced colors, and no axe violations. Backend authority remains covered by
ADR-0418 and `make integration-check`.
