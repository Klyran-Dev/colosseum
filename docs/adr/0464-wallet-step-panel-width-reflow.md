# ADR-0464: Reflow the wallet step from panel width

- Status: Accepted
- Date: 2026-10-10
- Owners: Product engineering, wallet integration, and accessibility

## Context

The payment-review wallet step combines a step marker, explanatory copy, and a
wallet picker in one three-column grid. When the checkout panel is narrow, the
wallet picker consumes its minimum width and leaves the explanatory copy with
too little space. The no-wallet state then wraps ordinary words across lines,
making a prerequisite look broken and obscuring the next action.

## Decision

Make the checkout layout an inline-size container. Below the narrow-panel
threshold, keep the step marker and explanatory copy on the first row and place
the wallet picker below the copy, spanning the content column. The picker may
shrink to the panel width while preserving full-width controls. Wide panels
retain the compact three-column step layout.

## Alternatives considered

1. **Shorten the no-wallet message.** Rejected because it treats the symptom
   and still clips provider names, account choices, and localized copy.
2. **Always use a vertical wallet step.** Rejected because it adds avoidable
   height on wide desktop panels.
3. **Keep the three-column grid and reduce picker width.** Rejected because a
   wallet selector and account identity need usable control width.
4. **Use a checkout-panel container query and a two-row narrow layout.**
   Selected because the content responds to the space it actually has and
   keeps the prerequisite/action hierarchy readable.

## Acceptance criteria

- The no-wallet status is rendered inside a dedicated wallet-step wrapper.
- Narrow checkout panels place the wallet picker below the explanatory copy.
- Wallet controls do not retain a minimum width that can overflow the panel.
- Wide panels retain the compact three-column step presentation.
- Browser accessibility and reflow gates remain green.
