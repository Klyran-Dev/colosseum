# ADR-0393: Versioned revenue-streaming recovery language

- Status: Accepted
- Date: 2026-10-05
- Owners: Web Platform and Revenue Controls

## Context

Revenue Streaming is a governed payout-plan surface. It reads finalized
balances, selects server-owned payout references, records four-eyes lifecycle
transitions, and prepares bounded payout claims. Operators must be able to
distinguish a recovery state from a financial success state without seeing
provider diagnostics or inconsistent technical literals.

The workspace already had localized primary copy and redacted ordinary
exceptions, but transport failures did not consistently include a bounded
status and several secondary labels remained embedded in the component. The
invalid-address fallback, claim amount placeholder, and evidence-reference
labels also bypassed the locale catalog.

## Decision

Keep Revenue Streaming operator language in the versioned English, German,
Arabic, and Japanese catalog. The panel now:

1. Formats HTTP failures through a bounded `100..599` status formatter and
   renders `unknown status` for invalid values.
2. Uses localized catalog values for unavailable address text, claim amount
   upper-bound placeholders, and selected setup evidence labels.
3. Keeps parser and response diagnostics behind the existing
   `OperatorFacingError` redaction boundary.
4. Preserves server-owned destination, split, and tax-policy references; the
   browser still sends only reference IDs and never sends digests or wallet
   addresses as authority inputs.
5. Preserves finalized-balance reads, authorization decisions, lifecycle
   transitions, idempotency keys, claim bounds, and fail-closed recovery.

## Alternatives considered

### Keep component literals and omit status values

Rejected. It creates inconsistent localization and removes useful bounded
support context from transport failures.

### Display API/provider diagnostics directly

Rejected. Response bodies and parser exceptions are not presentation or
authority sources and may disclose implementation details.

### Localize every protocol value

Rejected. Status enums, reference IDs, slots, asset addresses, and digests are
protocol/evidence values. They remain canonical and are formatted only through
safe display helpers.

### Replace the catalog with a global shared error service

Deferred. This bounded context keeps ownership clear; a shared catalog needs
an explicit compatibility, translation-review, and versioning ADR.

## Invariants

- A displayed balance is derived from server-owned finalized evidence; the
  browser never calculates an amount to authorize a claim.
- A prepared payout is not a transfer and never implies finalized payment.
- HTTP status text is bounded display copy only and cannot influence authority,
  retry, idempotency, or financial decisions.
- Unknown, malformed, stale, or unauthorized state remains unavailable or
  fails closed.
- Locale changes do not alter route IDs, policy IDs, plan IDs, asset bytes,
  slots, status enums, or request payloads.

## Verification

- Failing-first catalog tests cover every supported locale and invalid status
  bounds.
- Revenue Streaming UI tests, typecheck, production build, browser journeys,
  accessibility/color contracts, and repository gates are required.

## Consequences

Operators receive consistent recovery guidance and localized secondary labels.
Support receives bounded HTTP context without raw diagnostics. New Revenue
Streaming failure or evidence states must add reviewed catalog entries and
tests before component copy is introduced. This ADR changes presentation and
error disclosure only; payout authority and chain semantics are unchanged.
