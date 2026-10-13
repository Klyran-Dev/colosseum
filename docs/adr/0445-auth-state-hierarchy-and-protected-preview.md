# ADR-0445 — Authentication state hierarchy and protected workspace preview

- Status: Accepted
- Date: 2026-10-09
- Decision owners: Principal UI Architecture, Security

## Context

The signed-out protected workspace rendered the shell sign-in gate and the
workspace's own disabled form state at the same time. The sidebar identity,
top-bar control, access card, shell gate, and panel notice repeated the same
message, creating visual competition and leaving a large form surface that a
signed-out operator could not use.

## Decision

Use one primary authentication state per protected workspace:

1. When signed out, show the shell gate as the single workspace action and do
   not mount protected workspace children. Keep the page title, description,
   navigation, and shell account affordance for orientation.
2. Hide the secondary “Your access” context card while signed out. The shell
   gate already communicates the prerequisite and owns the next action.
3. When authenticated but the server capability snapshot is pending or
   unavailable, keep the shell-level fail-closed access state and do not mount
   protected children.
4. When authenticated and the snapshot is server-allowed, mount the workspace.
   Workspace-level capability denial remains visible because it explains a
   real role or scope decision rather than repeating sign-in guidance.

## Alternatives considered

- **Keep every repeated lock state:** Rejected. It increases cognitive load and
  makes the protected form look partially usable.
- **Hide the complete page including title and navigation:** Rejected. It
  breaks deep-link orientation and makes authentication look like a routing
  failure.
- **Use a translucent overlay over the form:** Rejected. It is weaker for
  keyboard and assistive-technology users and preserves unusable controls.
- **Render a static form preview:** Rejected. It can imply that the operator
  may prepare a request before identity and capability context exists.

## Invariants and failure behavior

- A signed-out protected route has exactly one primary sign-in action in the
  workspace body.
- No protected child effect, request, or form control is mounted while signed
  out or while server authorization is uncertain.
- Authenticated capability denial remains explicit and does not imply that
  signing in again will grant access.
- The API remains the authorization authority; this is a presentation and
  lifecycle boundary only.

## Evidence

- `ProductShell.test.tsx` proves signed-out protected children are not mounted
  and server authorization pending also prevents mounting.
- The Docker-served Tax Rules route was inspected at desktop and narrow
  viewports after the shell hierarchy change.
- Frontend unit tests, typecheck, build, color, language, and CSP contracts
  remain required gates for this change.
