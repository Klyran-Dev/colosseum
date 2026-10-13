# ADR-0514: Use a compact Metronic Demo 3 operational callout

## Status

Accepted — 2026-10-11

## Context

The first Demo 3 shell pass corrected the rail, header, contextual navbar, and
status-card structure, but the signed-out workspace still rendered as a large
dark authentication stage. The result repeated access messaging, created a
large dead zone, and made a protected page feel like a diagnostic screen
instead of an operational SaaS workspace.

## Decision

Keep the global email sign-in control as the only authentication action and
make the protected workspace state a compact, two-column operational callout:

- keep the page title and description close to the status deck;
- use four restrained tonal status cards as context, not as a second dashboard;
- put the access state and sign-in guidance in the callout's leading content;
- keep the three-step explanation as a compact vertical process list;
- remove the oversized stage treatment, decorative emphasis, and duplicate
  sign-in affordance;
- reflow the callout and process list to one column at narrow widths.

## Alternatives considered

| Alternative | Result | Reason |
| --- | --- | --- |
| Keep the large protected-workspace stage | Rejected | It dominates the page and repeats the access state already shown above. |
| Add another sign-in button inside the workspace | Rejected | Increases cognitive load and violates the single global authentication action. |
| Hide all protected-workspace context | Rejected | Users still need to understand why content is unavailable and what happens next. |
| **Compact Demo 3 operational callout** | **Selected** | Preserves clarity and security messaging while restoring premium workspace density. |

## Invariants

- The browser never becomes an authorization boundary.
- Protected content remains absent until server authorization confirms access.
- Signed-out protected pages expose one global sign-in action.
- Access, deployment, network, and API labels remain derived from runtime-owned
  state; the callout does not imply readiness or legal compliance.
- The layout remains keyboard accessible and responsive without horizontal
  overflow.

## Verification

- Component test asserts the compact-callout presentation contract and single
  global sign-in action.
- Shell, style-contract, stylesheet-budget, typecheck, and browser checks are
  run against the Docker-served SPA.
