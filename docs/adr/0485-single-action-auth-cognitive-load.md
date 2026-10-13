# ADR-0485: Keep one actionable sign-in entry point on signed-out protected pages

- Status: Accepted; supersedes ADR-0483
- Date: 2026-10-10
- Owners: Web platform, accessibility, and product engineering

## Problem

Signed-out protected workspaces currently repeat the same transition in the
header auth control and in the workspace gate. The duplicate `Open sign-in`
button increases action density and makes the operator choose between two
controls that open the same auth state. The rail identity block and gate status
remain useful context, but they are not additional actions.

## Decision

Keep the header authentication control as the single actionable sign-in entry
point. A signed-out protected gate remains next to the blocked workspace and
explains the required transition, but renders only the status and a concise
instruction to use the header control. It does not open a second auth surface.

The authenticated session control, email-OTP state, OIDC flow, and protected
workspace fail-closed behavior remain unchanged. This is a cognitive-load and
shell hierarchy change, not an authorization change.

## Alternatives considered

| Option | Benefits | Costs | Decision |
| --- | --- | --- | --- |
| Keep header and gate actions | Short path from either location | Duplicate controls compete and repeat the same transition | Rejected |
| Remove the header action on protected pages | Fewer visible actions | Makes the global entry point route-dependent and harder to discover | Rejected |
| Inline a second auth form in the gate | Immediate field proximity | Duplicates auth state, validation, focus, and error surfaces | Rejected |
| **Keep the header action and make the gate explanatory** | **One clear action with local reason and global discoverability** | **Requires concise instructional copy and a visible header control** | **Selected** |

## Invariants and acceptance criteria

- A signed-out protected workspace renders exactly one enabled sign-in button.
- The gate still states why access is unavailable and remains fail-closed.
- The header control remains keyboard accessible and opens the configured
  email-OTP, local, or OIDC flow.
- The rail identity block is status context only, never a second auth action.
- English, German, Arabic RTL, narrow/mobile, and accessibility flows retain
  readable, localized guidance.

## Verification

- Component tests assert the protected gate has no second sign-in button and
  that the header control remains present.
- The real-browser gate checks mobile/reflow, accessibility, localization,
  interaction fidelity, and protected workspace behavior.
