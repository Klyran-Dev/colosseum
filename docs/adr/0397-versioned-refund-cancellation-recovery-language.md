# ADR-0397: Versioned refund-cancellation recovery language

- Status: Accepted
- Date: 2026-10-05
- Owners: Web Platform and Refund Controls

## Context

Refund Cancellation is a money-adjacent governance workflow. It creates or
recovers an immutable cancellation artifact, requires an approved governance
wallet signature, submits unchanged bytes, and reconciles the exact finalized
on-chain event. A provider acknowledgement or wallet signature is never proof
that the refund claim was cancelled.

The panel had reviewed localized lifecycle, wallet, unknown-outcome, and
reconciliation copy, but its prepare, recovery, submit, and reconciliation
HTTP failures still interpolated raw status values directly in the component.

## Decision

Keep refund-cancellation recovery language in the versioned English, German,
Arabic, and Japanese catalog. The panel now:

1. Routes all four HTTP failure paths through `messages.errors.httpError`.
2. Displays only integer HTTP statuses in the inclusive `100..599` range;
   invalid or out-of-range values display `unknown status`.
3. Keeps provider, parser, wallet, transport, and response-body diagnostics
   behind the existing operator-facing redaction boundary.
4. Preserves unchanged signed bytes, explicit confirmation, idempotent intent
   creation, governance-wallet authority, unknown-outcome handling, finalized
   event reconciliation, and claim-state convergence.
5. Leaves signatures, digests, Solana addresses, statuses, slots, and event
   evidence as exact protocol values rather than translating them.

## Alternatives considered

### Keep component status interpolation

Rejected. It bypasses catalog review on a workflow that can lead to a
money-state transition.

### Display provider or wallet diagnostics

Rejected. These are not finalized authority and may disclose sensitive or
implementation-specific details.

### Hide all status context

Rejected. A bounded status improves supportability without authorizing signing,
submission, cancellation, or reconciliation.

### Localize protocol and finalized evidence values

Rejected. Identifiers, signatures, digests, statuses, slots, and event fields
must remain exact for replay and operator verification. Only surrounding copy
is localized.

## Invariants

- A failed HTTP response cannot create, sign, submit, or reconcile a
  cancellation.
- Unknown provider outcomes remain unknown and never become cancellation proof.
- Wallet signing and provider acknowledgement never render finalized state.
- Recovery copy cannot influence signed bytes, authority checks, idempotency,
  lifecycle state, or finalized evidence.
- The server-owned artifact and finalized Solana event remain authoritative.

## Verification

- Failing-first catalog tests cover every supported locale and invalid status
  bounds.
- Refund Cancellation UI tests, typecheck, production build, browser
  journeys, accessibility/color contracts, and repository gates are required.

## Consequences

Operators receive consistent localized recovery guidance while the
non-custodial, fail-closed cancellation authority boundary remains explicit.
Future refund-cancellation failure states must add reviewed catalog entries and
tests before component copy is introduced.
