# ADR-0395: Versioned governance recovery language

- Status: Accepted
- Date: 2026-10-05
- Owners: Web Platform and Tax Governance Controls

## Context

Governance is a money-adjacent write surface. It creates immutable rule
drafts, advances independently authorized lifecycle states, and runs
server-owned simulations. The browser must never treat a local draft,
simulation, or optimistic transition as published authority.

The governance panel already redacted provider and transport exceptions, but
its lookup, draft, transition, and simulation boundaries interpolated raw HTTP
status values in component code. This bypassed the versioned locale catalog and
made recovery behavior inconsistent across the high-risk operator surfaces.

## Decision

Keep governance recovery language in the versioned English, German, Arabic,
and Japanese catalog. The panel now:

1. Routes all four HTTP failure paths through `messages.common.httpError`.
2. Displays only integer HTTP statuses in the inclusive `100..599` range;
   invalid or out-of-range values display `unknown status`.
3. Keeps provider, parser, response-body, and transport details behind the
   existing `OperatorFacingError` redaction boundary.
4. Preserves authenticated tenant scope, idempotent draft creation, immutable
   rule documents, independent lifecycle transitions, provenance requirements,
   and server-owned simulation semantics.
5. Leaves rule IDs, versions, digests, statuses, and governance evidence as
   exact protocol/evidence values rather than translating or using them as
   authority.

## Alternatives considered

### Keep component status interpolation

Rejected. It bypasses catalog review and makes the recovery boundary drift
between governance actions.

### Display response or provider diagnostics

Rejected. Response bodies and exceptions are not presentation authority and
may disclose implementation details or sensitive policy information.

### Hide all transport status context

Rejected. A bounded status is useful support context when it cannot authorize a
retry, transition, publication, or simulation result.

### Localize lifecycle and evidence values

Rejected. Status codes, rule versions, digests, identifiers, and evidence must
remain exact for replay and operator comparison. Only surrounding copy is
localized.

## Invariants

- A failed HTTP response cannot be interpreted as a successful governance
  transition, publication, or simulation.
- A transport/parser failure never exposes provider diagnostics in the UI.
- Browser recovery copy cannot influence authorization, idempotency, rule
  digests, lifecycle state, or payment authority.
- A simulation result never authorizes a quote, settlement, or payment.
- Server state, authenticated policy decisions, immutable rule evidence, and
  persisted governance events remain authoritative.

## Verification

- Failing-first catalog tests cover every supported locale and invalid HTTP
  status bounds.
- Governance UI tests, typecheck, production build, browser journeys,
  accessibility/color contracts, and repository gates are required.

## Consequences

Operators receive consistent, localized, bounded recovery guidance while the
governance authority boundary remains unchanged. Future governance failure
states must add reviewed catalog entries and tests before component copy is
introduced.
