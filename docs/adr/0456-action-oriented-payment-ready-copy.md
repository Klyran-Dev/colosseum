# ADR-0456: Make the ready state action-oriented

- Status: Accepted
- Date: 2026-10-09
- Owners: Product engineering, operations, and accessibility

## Context

The document payment review already displayed a safeguards card before
compilation. Once the artifact was ready, a second success block repeated the
same safeguard statement immediately above the amount summary. The repetition
used valuable attention without helping the operator decide whether to approve
the exact payment.

## Decision

Keep the safeguards assertion in the review form and make the ready-state copy
action-oriented: it directs the operator to review the exact scope and amounts
before approving in the wallet. The ready heading, amount summary, evidence
references, and approval action remain unchanged.

## Alternatives considered

1. Keep the repeated safeguard statement. Rejected because it creates visual
   noise and does not add new evidence.
2. Remove all ready-state supporting copy. Rejected because the prepared state
   still needs a concise instruction before the wallet action.
3. **Replace the repeated assertion with review guidance.** Selected because it
   preserves the safety boundary while improving decision hierarchy.

## Acceptance criteria

- The prepared review contains one safeguards assertion.
- The ready state tells the operator what to review before wallet approval.
- The exact amount summary, evidence references, and approval action remain
  visible and accessible.
- Existing localization and browser accessibility gates remain green.
