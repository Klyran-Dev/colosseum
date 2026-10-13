# ADR-0463: Keep one authenticated session control in the shell

- Status: Accepted
- Date: 2026-10-10
- Owners: Product engineering, accessibility, and operations

## Context

After sign-in, the top bar exposed three related controls at once: a signed-in
status, a direct sign-out button, and an account avatar. The account menu
already owns the account identity, access facts, protection details, and sign-
out action. The duplicate button makes the shell feel busy and weakens the
relationship between identity and session actions.

## Decision

Keep the signed-in status as a passive status indicator and make the account
avatar the single interactive identity/session control. The account menu keeps
the sign-out action as a clearly labelled button, alongside access review and
protection details. Signed-out flows retain their existing sign-in affordance.

## Alternatives considered

1. **Keep direct sign-out and the account menu action.** Rejected because two
   controls perform the same session transition.
2. **Remove the account menu and keep direct sign-out.** Rejected because it
   discards the owned account/access context required for operator review.
3. **Hide sign-out behind an unlabeled avatar action.** Rejected because the
   menu remains the action owner but must expose a labelled sign-out button.
4. **Passive signed-in status plus labelled account-menu action.** Selected
   because it preserves discoverability and account context with less chrome.

## Acceptance criteria

- A signed-in shell renders one passive session status and one account-menu
  trigger, not a second direct sign-out action.
- The account menu exposes a labelled, keyboard-accessible sign-out button.
- Signed-out email-OTP, OIDC, local, and unconfigured states are unchanged.
- The account menu continues to avoid rendering or exposing bearer tokens.
