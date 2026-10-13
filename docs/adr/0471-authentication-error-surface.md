# ADR-0471: Keep authentication errors inside the form surface

## Status

Accepted

## Context

The shell's email-OTP and local credential controls are anchored popovers. The
generic inline-error rule positioned errors absolutely relative to the session
control. When an OTP request failed, the error detached from the form and
overlapped the workspace content, making the failure hard to associate with
the action that caused it.

## Decision

Override the generic floating error placement for errors inside the email-OTP
and local credential form surfaces. Render those errors in normal flow inside
the anchored surface, with the surface's existing grid providing spacing and
width.

## Alternatives considered

- Keep the error as a top-bar overlay: rejected because it can obscure the
  workspace and loses field/action context.
- Render a second error in the workspace gate: rejected because it duplicates
  authentication state and creates competing announcements.
- Replace the popover with a modal dialog: rejected because sign-in remains a
  short shell action and does not need modal focus trapping.

## Consequences

- Authentication failures remain visually attached to the form.
- The popover grows to contain the message instead of covering workspace text.
- The existing `role="alert"` announcement remains intact.
- The rule applies equally to local development credentials and email OTP.

## Verification

- Material shell contract test for static in-surface error placement.
- Real-browser interaction, reflow, and accessibility suites.
