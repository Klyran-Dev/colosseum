# ADR-0297: Production account and session menu

## Status

Accepted — implemented and verified in the product shell and real browser gate.

## Context

The product shell exposes account/session state in the top bar, but its avatar
control is currently interactive-looking without an action. That is a false
affordance and makes it harder for an operator to understand which account,
identity, and access decision are in force. The shell must also provide a
consistent, keyboard-accessible sign-out path without exposing tokens or
turning browser-rendered claims into authorization.

## Decision

Replace the inert avatar button with a native accessible disclosure menu. The
menu is a presentation of server-owned session state and contains:

- account confirmation status and the short, non-sensitive account reference;
- signed-in subject status and access-check status;
- a plain-language link to the Access & approvals workspace;
- a sign-out action that invokes the existing authenticated session boundary.

Anonymous, access-checking, access-error, and server-allowed states remain
explicit. The menu never renders bearer tokens, wallet keys, raw claims,
authority digests, or technical account topology. It cannot grant access; the
API capability snapshot remains authoritative and all protected actions still
fail closed when that snapshot is unavailable.

The native `<details>/<summary>` pattern is selected for the first boundary:
it provides keyboard and assistive-technology semantics without a global
overlay manager, portal lifecycle, or focus-trap dependency. A future menu may
add a dedicated account route only if profile, organization switching, or
support workflows require durable navigation state.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| Leave the avatar inert | Rejected | Creates a false affordance and fails the no-fake-control mandate. |
| Modal account dialog | Rejected for this slice | Adds focus-trap and dismissal complexity for a small, non-editable summary. |
| Dedicated account route | Deferred | Useful for profile and organization administration, but excessive for session status and sign-out. |
| Native disclosure menu | Selected | Small, accessible, progressive, keyboard-friendly, and easy to test without a client-side authority boundary. |

## Invariants and failure policy

- No credential, token, wallet key, or raw identity-provider claim is rendered.
- Sign-out is available even when the capability snapshot is unavailable.
- The access-review link is informational; it does not bypass authorization.
- A missing or failed access snapshot is shown as unavailable, never as allowed.
- The control has a meaningful accessible name and visible keyboard focus.

## Verification

- Product-shell component tests cover anonymous, server-allowed, and access-error states.
- The component and real-browser suites verify the disclosure name, sign-out
  button, access-review link, anonymous state, and post-sign-out state.
- `make check` and the real-browser UI journey gate remain required.
