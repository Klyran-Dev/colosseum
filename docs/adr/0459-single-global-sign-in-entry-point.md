# ADR-0459: Use one global signed-out sign-in entry point

- Status: Accepted
- Date: 2026-10-10
- Owners: Product engineering, accessibility, and operations

## Context

The signed-out protected workspace needs to explain why the current surface is
unavailable, but the rendered shell was showing two competing actions: the
global sign-in control in the header and a second “Open sign-in” button inside
the workspace gate. Both actions opened the same authentication path. This
made the most important next step look duplicated and increased visual noise,
especially at narrow widths.

## Decision

Keep the header sign-in control as the single actionable signed-out entry
point. The protected workspace gate remains visible as an access explanation:
it states the protected state, preserves the access-status pill, and tells the
operator to use the header control. The gate does not open a second auth
surface.

### Alternatives considered

1. **Keep both actions.** Rejected because duplicate controls compete for
   attention and create two apparent owners for the same transition.
2. **Remove the header control.** Rejected because the global entry point is
   discoverable from every workspace and remains usable when the protected
   content is gated.
3. **Keep the header control and make the gate explanatory only.** Selected
   because it preserves discoverability while reducing action density and
   keeping the access reason beside the blocked workspace.

## Acceptance criteria

- A signed-out protected workspace contains no second sign-in button.
- The header sign-in control remains keyboard accessible and opens the
  configured email-OTP or OIDC flow.
- The gate explains where to continue in each supported locale.
- Desktop, narrow/reflow, RTL, and accessibility checks remain green.
- The protected gate remains fail-closed; removing the duplicate button does
  not expose protected workspace content.
