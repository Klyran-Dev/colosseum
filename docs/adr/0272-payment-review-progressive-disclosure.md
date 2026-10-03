# ADR-0272: Payment review uses progressive disclosure for protocol evidence

- Status: Accepted
- Date: 2026-10-03
- Owners: Product, UI, Platform, Security

## Context

Payment authorization is a high-consequence workflow. Finance and operations
users need to answer three questions quickly: what amount will leave the
wallet, how the amount is distributed, and whether the payment is safe to
authorize. They should not need to understand Solana account metas, compute
budgets, policy digests, or intent identifiers to make that decision.

The same workflow must still expose exact protocol evidence to security,
support, auditors, and incident responders. Removing that evidence would make
the UI less supportable and would weaken the boundary between a reviewed
payment and a signed transaction.

## Decision

The payment workspace has two information layers:

1. The primary layer uses task-oriented language and shows amount before tax,
   tax withheld, amount sent, payment route, recipient distribution, quote
   expiry, wallet approval, and the server-owned status.
2. An explicit, keyboard-accessible `Technical payment details` disclosure
   contains the exact route accounts, token program, program ID, policy
   sequence/reference, compute budget, account-meta count, and durable intent
   identifier.

The browser continues to submit the same validated values and exact bytes. This
is a presentation boundary, not a security boundary. Server validation,
wallet-boundary checks, signed-message verification, and finalized chain
evidence remain authoritative.

Protocol values that are derived from server-owned evidence are not inputs to
the primary form. The primary layer confirms that payment safeguards and a
secure retry reference are selected automatically. Their disclosure state is
not persisted as an authorization signal. Exact evidence remains available in
the technical details disclosure after a payment artifact has been prepared.

## Alternatives considered

| Option | Result |
| --- | --- |
| Show every protocol field in the main form and review | Rejected: increases cognitive load, makes the product feel like an operator console, and encourages unsafe copy/paste behavior without improving authority. |
| Hide all protocol evidence | Rejected: support, audit, and incident response need exact values to reconcile a reviewed artifact. |
| Use tooltips only | Rejected: tooltips are poor for keyboard, touch, screen-reader, and evidence workflows. |
| Progressive disclosure with explicit accessible details | Selected: keeps the primary decision legible while preserving inspectable evidence and a stable automation target. |

## Invariants and acceptance criteria

- The primary payment heading and action use finance/operator language.
- The amount card names the wallet authorization and separates gross, tax, and
  net values.
- Exact protocol values remain rendered in the DOM only inside an explicit
  disclosure and remain copyable for support workflows.
- The disclosure uses native `details`/`summary`, has visible focus, and works
  without hover, pointer precision, or JavaScript-only focus behavior.
- No value, account, route, policy, intent, status, or signing behavior changes
  as a consequence of the presentation change.
- Browser and component tests cover the collapsed primary state, expanded
  technical evidence, and the disabled/error states around quote expiry.

## Consequences

The product can be understood by finance and operations users without hiding
the evidence required by technical reviewers. Test selectors and support
instructions should prefer the plain-language labels; technical identifiers
remain deliberately available for diagnostics and audit.
