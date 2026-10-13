# ADR-0442 — Signed-out workspace gate and compact authentication control

- Status: Accepted
- Date: 2026-10-09
- Decision owners: Principal UI Architecture, Security

## Context

The shared product shell rendered the email-OTP form open in the top bar on every
signed-out route. On small screens this pushed the workspace below the fold and
made authentication look like the primary application surface. Protected
workspaces also showed blank, disabled forms without a single clear next action.
The account avatar used a question mark, which was ambiguous as an account
control and duplicated the signed-out state already shown elsewhere.

## Decision

1. Keep authentication collapsed by default. A compact, keyboard-accessible
   summary opens an anchored form only after the operator requests sign-in.
2. Add a shared signed-out workspace gate before workspace children. It states
   that tenant access is checked before protected actions and provides one
   `Open sign-in` action. Workspace content remains present for context and
   assistive-technology continuity; the existing action guards remain the
   authority for submission.
3. Use an account glyph for the account menu and identity state. Status text,
   not icon shape, communicates whether access is missing, pending, denied, or
   confirmed.
4. Localize the gate copy with the typed shell session catalog for English,
   German, Arabic, and Japanese. No raw color literals or workspace-specific
   authentication styling are introduced.

## Alternatives considered

- **Hide all workspace content until sign-in:** Rejected. It removes useful
  workspace context, makes deep links feel broken, and conflates authentication
  with authorization.
- **Keep the inline form open:** Rejected. It consumes shell space, especially
  on mobile, and gives no hierarchy to the actual work surface.
- **Use a modal dialog for sign-in:** Rejected for this slice. It adds focus,
  escape, scroll-lock, and restoration complexity to a flow that already has a
  bounded disclosure control.
- **Keep the question-mark avatar:** Rejected. It is not a recognizable account
  affordance and can be mistaken for help or an unresolved error.

## Invariants and failure behavior

- No access token is created, stored, or inferred by the gate.
- The server/API authorization decision remains authoritative; the gate is only
  presentation and navigation.
- The auth form remains closed after a successful sign-in and can be reopened
  explicitly after sign-out.
- Auth service errors remain visible inside the bounded auth surface and do not
  displace the workspace permanently.
- The gate and auth control use the same typed localized catalog and preserve
  focusable controls at narrow widths.

## Evidence

- `ProductShell.test.tsx` proves the signed-out gate is the primary next action
  and opens the sign-in control.
- `AuthSessionControl.test.tsx` proves email sign-in is closed by default and
  reports the disclosure transition.
- Frontend tests: 439 passed, 10 skipped.
- TypeScript typecheck and Vite production build passed.
- Docker edge image rebuilt and the served UI was inspected at the narrow
  browser viewport; the sign-in form is compact until explicitly opened.

## Limits and follow-up

This ADR does not redesign each workspace form. The next UI review pass should
replace broad disabled-form areas with explicit loading, preview, and
authorization states where the individual workspace contract permits it.
