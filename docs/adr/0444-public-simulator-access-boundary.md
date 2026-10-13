# ADR-0444 — Public simulator access boundary

- Status: Accepted
- Date: 2026-10-09
- Decision owners: Principal UI Architecture, Tax Engine, Security

## Context

The tax simulator intentionally calls the calculation endpoint without an
authenticated account because it is a read-only what-if operation. The shared
shell initially treated every workspace as protected, which showed a sign-in
gate on the simulator despite the form being designed to work anonymously.
That contradicted the product contract and made the shell look unreliable.

## Decision

Derive the shell’s authentication requirement from the versioned authorization
policy catalog. A workspace requires authentication when any of its policies
requires a scope, role, or attribute. The simulator has no protected policy
requirements, so it remains available without sign-in and is labeled as such.
All workspaces without a known policy fail closed and remain protected.

The API remains authoritative. This classification only controls presentation
and navigation affordances; a public simulator result never authorizes a quote,
settlement, rule change, or other money-moving action.

## Alternatives considered

- **Gate every workspace uniformly:** Rejected because it contradicts the
  simulator’s anonymous what-if contract.
- **Hard-code `preview` in the shell:** Rejected because policy ownership would
  drift from the authorization catalog.
- **Infer public access from an empty token or route name:** Rejected because
  authentication requirements belong to the policy contract, not URL shape.

## Evidence

- `workspaceRequiresAuthentication` is derived from policy scope, role, and
  attribute requirements.
- Authorization tests prove `preview` is public and `document-checkout` is
  protected.
- Shell tests prove the simulator has no sign-in gate and protected workspaces
  retain one.
