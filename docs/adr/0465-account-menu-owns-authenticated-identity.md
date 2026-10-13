# ADR-0465: Let the account menu own authenticated identity presentation

- Status: Accepted
- Date: 2026-10-10
- Owners: Product engineering, accessibility, and operations

## Context

The authenticated top bar rendered a passive `Signed in` control beside the
account avatar. The avatar menu already owned the account label, access facts,
protection details, access review, and labelled sign-out action. Keeping both
presentations consumed horizontal space and made the identity/session model
look like two competing controls. The persistent sidebar identity state still
provides the workspace-level access posture.

## Decision

When an authenticated session is valid, `AuthSessionControl` renders no
additional visible top-bar control. The account menu is the single interactive
identity/session presentation and remains labelled for assistive technology.
Authentication errors remain visible through the existing error surface when
the session control has an error. Signed-out and unconfigured states keep their
existing sign-in affordances.

## Alternatives considered

1. **Keep the passive signed-in status and account menu.** Rejected because it
   duplicates identity state in the narrowest, highest-frequency shell area.
2. **Remove the account menu.** Rejected because it would hide access facts,
   protection details, and the labelled sign-out action.
3. **Move all session context to the sidebar.** Rejected because the header
   account menu remains the discoverable owner of identity actions and details;
   the sidebar can retain its existing passive access posture independently.

## Invariants and acceptance criteria

- An authenticated shell has one visible account/session control: the account
  menu trigger.
- The account menu keeps a labelled sign-out button and never exposes bearer
  tokens.
- Signed-out email OTP, OIDC, local, and unconfigured flows remain available.
- Session errors remain exposed as an alert when present.
- The change removes no authorization or access-state evidence.

## Verification

- Auth session component tests cover the authenticated no-control state and
  bearer-token non-disclosure.
- Product-shell UI tests continue to cover the account menu and sign-out path.
- The Docker browser gate verifies the rendered shell at desktop and narrow
  widths.
