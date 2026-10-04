# ADR-0300: Close the semantic color-token boundary across the SPA

## Status

Accepted — implemented and verified in the stylesheet and web gates.

## Context

ADR-0298 established Material 3 Expressive roles and a final precedence layer,
but the stylesheet still contains legacy literal colors in selectors that are
also rendered inside the production shell. A final override can hide some of
those values while leaving the palette contract dependent on selector order.
That is fragile under future workspace additions, makes light-theme review
harder, and permits an unrelated color literal to communicate state.

The production UI needs one auditable semantic color boundary. Theme changes
must update roles, not individual components; alpha treatments must derive from
roles; and financial, access, warning, and error states must remain meaningful
without relying on decorative color variation.

## Decision

Migrate all non-token stylesheet color declarations to repository-owned semantic
roles. The token declaration blocks remain the only place where scheme-specific
color values may be defined. Component rules may use role variables, role-
derived color-mix() values, and shared elevation/action tokens, but may not
contain raw hex, rgb, or rgba literals.

The contract applies to the complete apps/web/src/styles.css, not only the
last precedence layer. The verifier fails if a color literal appears outside
the declared token blocks. New components must therefore choose an existing
role or add a documented role to the token model and ADR before adding CSS.

## Alternatives considered

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Keep the final override layer only | Smallest diff | Legacy rules remain an unreviewed palette and can regress when selectors change | Rejected |
| Replace literals with component-local variables | Local readability | Duplicates theme semantics and permits inconsistent state meaning | Rejected |
| Use a third-party Material theme package | Broad component coverage | Adds dependency and ownership surface; does not govern domain failure states | Rejected |
| **Use one repository semantic token layer and enforce it statically** | Auditable, theme-safe, low runtime cost, compatible with the existing SPA | Requires an initial stylesheet migration and token review | **Selected** |

## Invariants

- No component rule contains a raw color literal.
- Theme-specific values exist only in the documented Material role blocks.
- Alpha and elevation treatments derive from semantic roles or shared tokens.
- A color role communicates one meaning across every workspace.
- The migration changes presentation only; it cannot affect authorization,
  wallet artifacts, calculations, or settlement state.

## Verification

- The web color contract scans the complete stylesheet after excluding only the
  token declaration blocks.
- Web component tests and the real-browser theme journey remain required.
- make web-check, make web-color-contract, make material-palette-contract, and
  make web-browser-check must pass before this slice is reviewable.
