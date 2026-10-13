# ADR-0450: One signed-out authentication affordance

- Status: Accepted
- Date: 2026-10-09
- Owners: Product engineering and security

## Context

The shell previously rendered both the email sign-in control and an account
menu while signed out. The account menu could only explain that sign-in was
needed, so it duplicated the actionable sign-in affordance and added a second
popover to the primary navigation row. This is especially noisy on narrow
screens and makes the next action ambiguous.

## Decision

When no access token is present, render the configured sign-in control as the
only authentication affordance in the top bar. Render the account menu only
when a session token exists; the signed-in account menu remains the single
place for verified subject context, access review, and sign-out.

This is a presentation rule only. It does not change authentication, session
storage, authorization, or the protected-workspace gate. A signed-out operator
can still reach sign-in from the workspace gate and the email sign-in control.

## Alternatives considered

1. Keep both controls and rely on different labels. Rejected because two
   controls still represent the same next step and compete in the mobile row.
2. Remove the email sign-in control and make the account icon start sign-in.
   Rejected because it hides the available authentication method and weakens
   discoverability for email-OTP users.
3. Keep the account menu as a read-only signed-out status panel. Rejected
   because the shell already exposes session status in the sidebar and system
   status disclosure.

## Acceptance criteria

- Signed-out shell has exactly one actionable sign-in disclosure in the top bar.
- Signed-in shell retains the account menu and safe sign-out action.
- Protected workspace sign-in and public simulator behavior are unchanged.
- Desktop, mobile, keyboard, localization, and WCAG browser gates remain green.

