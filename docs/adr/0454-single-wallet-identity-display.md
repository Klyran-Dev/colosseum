# ADR-0454: Keep wallet identity in one owned control

- Status: Accepted
- Date: 2026-10-09
- Owners: Product engineering, operations, and accessibility

## Context

The connected wallet was rendered in two places in each payment workflow: the
step summary printed the full payer address, while `WalletPicker` printed the
provider and selected account again. Long Solana public keys then dominated a
small status row and made the primary payment review harder to scan.

## Decision

The wallet step summary owns only the workflow state (`Wallet connected` or
`No wallet connected`). `WalletPicker` owns provider and selected-account
identity, including the full public key and reconnect action. A connected
public key must therefore have one rendered occurrence in the payment review.

## Alternatives considered

1. Keep both copies. Rejected because the duplication adds no authority or
   evidence and creates avoidable visual density.
2. Hide the full address everywhere. Rejected because the operator needs an
   inspectable account identity before approving a wallet transaction.
3. **Keep the full identity in the wallet control and use a state-only step
   summary.** Selected because ownership is clear, the account remains
   inspectable, and the payment action is easier to scan.

## Acceptance criteria

- Connected payment workflows render the payer public key exactly once.
- Provider, account, and reconnect affordances remain available.
- Signed-out and disconnected states retain readable wallet guidance.
- Browser interaction, reflow, and WCAG gates remain green.
