# ADR-0483: Direct sign-in action from protected workspaces

- Status: Superseded by ADR-0485
- Date: 2026-10-10
- Owners: Web platform

## Problem

Signed-out users who open a protected workspace see a gate that explains that
sign-in is required, but the only action is an instruction to locate the
header sign-in control. On narrow screens and dense operational pages this
creates avoidable search and leaves the primary next step visually detached
from the blocked workspace.

## Decision

Add an `Open sign-in` action to the protected-workspace gate. The action only
expands the existing global `AuthSessionControl`; it does not create a second
authentication form, issue credentials, or change authorization behavior. The
header remains the single owner of sign-in state and validation. The gate
continues to explain why the workspace is protected and retains the global
control as the keyboard-accessible source of truth.

## Alternatives considered

| Option | Benefits | Costs | Decision |
| --- | --- | --- | --- |
| Instruction-only gate | No extra control | Requires visual search; poor mobile and keyboard flow | Rejected |
| Duplicate inline OTP form | Short path to entry | Duplicates auth state, validation, error handling, and security surface | Rejected |
| Gate action that expands the global control | One obvious next step; preserves one auth owner and existing flow | Requires controlled expansion state and localized action copy | Selected |

## Invariants and acceptance criteria

- The gate action is available only when a protected workspace is signed out.
- Activating it expands the existing global sign-in control without duplicating
  inputs or submit handlers.
- The action is keyboard operable, localized with the shell, and remains usable
  at mobile widths.
- When the existing email/local control opens, focus moves to its first
  actionable field; the OTP field receives focus when the challenge is shown.
- Existing OIDC, email OTP, local-development, and unconfigured auth modes keep
  their current behavior.
- The protected workspace remains fail-closed until authentication and server
  access checks succeed.

## Verification

- Add a component test that activates the gate action and asserts that the
  global email sign-in disclosure is open.
- Run the full frontend unit suite, typecheck, and Docker browser/accessibility
  gate.

