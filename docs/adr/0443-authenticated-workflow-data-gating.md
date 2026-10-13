# ADR-0443 — Authenticated workflow data gating

- Status: Accepted
- Date: 2026-10-09
- Decision owners: Principal UI Architecture, Security, Payments

## Context

The document-payment, aggregate checkout, document-quote, governance,
treasury, balance-check, refund, adjustment, replay, and evidence-access workspaces correctly
refused unauthenticated network requests, but their workflow inputs remained
editable while routes, quotes, authority, or evidence were unavailable. The
resulting surface looked broken: a user could enter data, see “no active
routes,” and encounter disabled actions without an explanation of the
dependency. This was a presentation failure even though the server-side
authorization boundary was intact.

## Decision

For authenticated, evidence-backed or authority-bearing workflows:

1. Keep the workflow visible for orientation, but render its inputs and actions
   disabled until the required authenticated context and service availability
   are present.
2. Put an explicit, localized workflow-lock notice before the first gated form.
   It names the missing prerequisite and states that no protected request,
   authority transition, evidence read, recovery action, or payment request
   can be sent until account and service checks complete.
3. Reuse the same interaction-disabled predicate for fieldsets, individual
   controls, and submit actions. This keeps browser semantics, keyboard
   behavior, and React test behavior aligned.
4. Do not mount protected workspace children while a bearer token is present
   but the server capability snapshot is pending, unavailable, expired, or the
   API is offline. The shell shows a bounded access-check state instead. A
   bearer token alone is not sufficient evidence to render protected controls.
5. Do not fetch routes or evidence without a bearer token. The UI state mirrors
   the existing service boundary; it does not become an authorization source.

## Alternatives considered

- **Leave fields editable and only disable submit:** Rejected. It suggests the
  user can make meaningful progress despite unavailable server-owned data.
- **Hide the entire protected workspace until sign-in:** Rejected. It removes
  useful orientation and makes deep links appear empty.
- **Automatically start sign-in from the workspace:** Rejected. Identity flows
  must be user initiated and remain visible as a deliberate authorization step.
- **Use one global disabled overlay:** Rejected. It is weak for keyboard and
  assistive technology users and hides which workflow data is unavailable.

## Invariants and failure behavior

- No authenticated token means no route/evidence request, governance mutation,
  protected evidence read, refund/adjustment mutation, recovery action,
  treasury transition, or payment submit.
- A token without a current server-allowed capability snapshot does not mount
  protected workspace controls; authorization uncertainty fails closed at the
  shell boundary.
- A disabled control cannot accept keyboard input or submit through the browser.
- Service-offline state remains fail-closed and does not imply that cached route
  data is authoritative.
- The lock notice is explanatory, not a promise that signing in will grant a
  particular role or payment authority.

## Evidence

- `ProductShell.test.tsx` proves a protected child is not mounted while server
  authorization is still being evaluated.
- `App.ui.test.tsx`, `documentCheckout-ui.test.tsx`, `checkout-ui.test.tsx`,
  `documentQuotePanel.test.tsx`, `governance-ui.test.tsx`,
  `vaultRelease-ui.test.tsx`, `refundClaim-ui.test.tsx`,
  `refundCancellation-ui.test.tsx`, `refundExecute-ui.test.tsx`,
  `settlementAdjustment-ui.test.tsx`, `vaultReconciliation-ui.test.tsx`,
  `replayOps-ui.test.tsx`, and `evidenceAccess-ui.test.tsx` prove the signed-out notice is present and
  protected controls are disabled.
- Accessibility tests continue to pass for the affected workspaces.
- TypeScript, Vite build, color contract, language contract, and Docker edge
  rebuild passed.
- The Docker-served document-payment route was inspected at a narrow viewport;
  the lock state is visible before the disabled form controls.
