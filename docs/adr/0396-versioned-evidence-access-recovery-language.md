# ADR-0396: Versioned evidence-access recovery language

- Status: Accepted
- Date: 2026-10-05
- Owners: Web Platform and Privacy Controls

## Context

Evidence Access is a tenant-scoped, read-only privacy and audit surface. It
shows who accessed protected tax evidence, for what purpose, and when. It must
not expose raw provider diagnostics or imply that an access record proves tax
filing, legal compliance, payment authorization, or finalized settlement.

The panel already enforced bounded list parsing, UUIDv7 filter validation,
authentication, audit-scope authorization, and technical-detail disclosure.
Its HTTP failure path nevertheless interpolated the raw response status in the
component, bypassing the versioned locale catalog.

## Decision

Keep Evidence Access recovery language in the versioned English, German,
Arabic, and Japanese catalog. The panel now:

1. Routes HTTP lookup failures through `messages.common.httpError`.
2. Displays only integer HTTP statuses in the inclusive `100..599` range;
   invalid or out-of-range values display `unknown status`.
3. Keeps provider, parser, transport, and response-body details behind the
   existing operator-facing redaction boundary.
4. Preserves tenant-scoped authorization, bounded list limits, UUIDv7 filter
   validation, read-only behavior, and technical evidence disclosure.
5. Leaves audit IDs, request IDs, calculation IDs, purposes, outcomes, and
   facts commitments as exact evidence values rather than translating them.

## Alternatives considered

### Keep component status interpolation

Rejected. It bypasses catalog review and creates inconsistent recovery copy on
privacy-sensitive surfaces.

### Show response bodies or provider diagnostics

Rejected. These are not evidence authority and can disclose implementation or
privacy-sensitive details.

### Hide all status context

Rejected. A bounded HTTP status improves supportability without authorizing a
retry, granting access, or changing the audit ledger.

### Translate audit values

Rejected. Identifiers, outcomes, purposes, and commitments must remain exact
for audit comparison and replay. Only surrounding presentation is localized.

## Invariants

- A failed or malformed read never renders as a successful audit result.
- An empty access list is not proof that no access occurred outside the
  bounded query or retention window.
- Browser copy cannot grant evidence access or mutate the ledger.
- HTTP status text cannot influence authorization, filtering, retention, or
  evidence interpretation.
- Authenticated tenant scope and durable audit storage remain authoritative.

## Verification

- Failing-first catalog tests cover every supported locale and invalid status
  bounds.
- Evidence Access parser/UI tests, typecheck, production build, browser
  journeys, accessibility/color contracts, and repository gates are required.

## Consequences

Operators receive consistent localized recovery guidance while privacy and
read-only audit semantics remain explicit. Future Evidence Access failure
states must add reviewed catalog entries and tests before component copy is
introduced.
