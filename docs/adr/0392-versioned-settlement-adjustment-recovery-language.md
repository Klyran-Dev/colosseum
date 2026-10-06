# ADR-0392: Versioned settlement-adjustment recovery language

- Status: Accepted
- Date: 2026-10-05
- Owners: Web Platform and Settlement Controls

## Context

Settlement Adjustments is a money-adjacent governance surface. It creates and
reviews correction requests, records approval transitions, and prepares a
wallet-bound ledger evidence instruction. It must remain understandable when
authentication, validation, transport, parsing, wallet, provider, or
reconciliation steps fail.

The panel previously mixed reviewed copy with raw parser errors, technical
validation details, and unbounded HTTP status interpolation. That made the
operator surface inconsistent across locales and risked exposing implementation
or provider details. Asset and evidence JSON errors also used wording that did
not identify the actual input family.

## Decision

Keep all operator-facing Settlement Adjustments recovery language in the
versioned English, German, Arabic, and Japanese catalog. The panel now:

1. Uses typed catalog entries for authentication, JSON validation, reason and
   retry-reference validation, transport failures, and lifecycle recovery.
2. Wraps authored validation messages in `OperatorFacingError`; ordinary
   parser, provider, transport, and implementation exceptions continue to be
   redacted to the operation's reviewed fallback.
3. Routes HTTP failures through a catalog formatter that displays only integer
   status values in the inclusive `100..599` range. Invalid or out-of-range
   values display `unknown status`.
4. Keeps asset/evidence-array JSON errors distinct from transition-evidence
   object errors.
5. Preserves all server-owned state, tenant authorization, exact wallet
   instruction, signed-message verification, durable intent, retry, and
   finalized-reconciliation behavior.

## Alternatives considered

### Keep component literals and raw exceptions

Rejected. This is inconsistent across locales, exposes technical wording, and
does not provide a stable reviewable language boundary.

### Show server/provider error text directly

Rejected. Provider and parser diagnostics may contain implementation details,
identifiers, or sensitive context. The existing redaction boundary must remain
the default.

### Use one generic error for every validation failure

Rejected. A safe recovery path needs to distinguish malformed array input,
malformed transition evidence, invalid reason text, and missing retry identity.

### Add a shared global error catalog immediately

Deferred. The settlement-adjustment catalog is the smallest bounded change for
this context. A shared catalog may be considered later only with an explicit
compatibility and ownership ADR.

## Invariants

- Browser messages never authorize a transition, token movement, or ledger
  submission.
- Unknown wallet/provider submission outcomes remain fail-closed; the UI tells
  the operator to reconcile the existing intent and never suggests signing a
  second transaction.
- HTTP status interpolation is bounded and does not become an authority,
  digest, or retry decision.
- Localized messages do not contain client-derived financial totals or raw
  provider diagnostics.
- The API and chain remain authoritative for adjustment state and finalized
  evidence.

## Verification

- Failing-first catalog tests cover every supported locale and bounded status
  behavior.
- Settlement-adjustment localization and UI tests pass.
- TypeScript typecheck passes.
- Full frontend, browser, accessibility, and repository security gates remain
  required before release.

## Consequences

Operators receive consistent, localized recovery guidance while support can
still correlate failures through the existing bounded support-reference path.
The catalog gains a small number of typed entries and must be reviewed when
new adjustment failure states are introduced. This ADR changes presentation
and error disclosure only; it does not change settlement authority or ledger
semantics.
