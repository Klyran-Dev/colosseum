# ADR-0452: Preserve readable wallet availability guidance

- Status: Accepted
- Date: 2026-10-09
- Owners: Product engineering, wallet integration, and accessibility

## Context

The payment-review workspace presents wallet availability as the operator's
first actionable prerequisite. The shared checkout-step paragraph applied a
single-line ellipsis to that message, so the browser rendered
“No compatible Solana wallet is currently avail…” at common desktop widths.
That hides the completion state and gives the operator no reliable next step.

## Decision

Allow checkout-step supporting text to wrap and break long wallet addresses or
provider names. Keep the step compact through the existing grid and typography,
but do not truncate operational guidance. The same rule applies to the
connected-wallet summary because it is also an identity-critical value.

## Alternatives considered

1. Keep the ellipsis. Rejected because it hides a required prerequisite.
2. Add a tooltip or title attribute. Rejected because it is unavailable or
   unreliable for keyboard, touch, and assistive-technology users.
3. **Wrap the supporting text in place.** Selected because it keeps the layout
   compact while preserving the complete message in every input modality.

## Acceptance criteria

- The no-wallet guidance is fully readable in the real browser.
- Long wallet/provider values may wrap without horizontal overflow.
- The payment-review surface remains WCAG-audited in dark and light themes.
- The Docker browser gate covers the regression.
