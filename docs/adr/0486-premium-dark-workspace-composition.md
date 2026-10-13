# ADR-0486: Compose the dark workspace as a premium operational stage

- Status: Accepted
- Date: 2026-10-10
- Owners: Web platform and product design

## Problem

The dark shell has the correct navigation and authorization behavior, but its
protected and empty states still read as a flat legacy admin surface. A large
unpopulated canvas, equal-weight chrome, and a full-width footer band weaken
the operator's sense of place and make the real next step less prominent.

## Decision

Use a composed operational stage for the dark and light shells:

- layer the main canvas, top bar, and workflow surfaces with distinct semantic
  roles rather than one uninterrupted background;
- give page headings a larger editorial scale and a narrower readable measure;
- render protected-workspace gates as a two-zone card: reason and state on the
  left, the next-step guidance on the right;
- use a restrained semantic accent rail on the gate to make the access boundary
  scannable without adding a decorative illustration;
- reduce the footer to quiet provenance metadata instead of a second content
  band;
- preserve real server/auth state and empty-state truth; no metrics, activity,
  charts, or placeholder workflow content are introduced.

## Alternatives considered

| Option | Benefits | Costs | Decision |
| --- | --- | --- | --- |
| Keep the flat dark canvas | Lowest implementation risk | Retains the legacy control-plane feel and dead-space problem | Rejected |
| Add gradients, illustrations, or sample activity | High visual impact | Violates semantic color and zero-mock requirements | Rejected |
| Center the entire gate as a modal-like empty state | Strong focus | Separates the explanation from the workspace context and scales poorly | Rejected |
| **Use semantic stage layers and a two-zone workflow gate** | **Premium composition while preserving real state and responsive flow** | **Requires coordinated shell and gate geometry** | **Selected** |

## Invariants and acceptance criteria

- Every visible status and value remains sourced from real state or an explicit
  empty/loading/error condition.
- The visual pass adds no raw colors outside scheme token declarations and no
  gradients or fake data.
- The protected gate remains fail-closed and keeps one sign-in action in the
  header.
- Desktop, mobile, RTL, reduced-motion, forced-colors, and WCAG checks remain
  green.

## Verification

- Add a stylesheet contract for stage layers, title hierarchy, and the gate's
  two-zone composition.
- Capture the real dark protected workspace at desktop and narrow widths.
- Run frontend tests, typecheck, `git diff --check`, and the Docker browser
  gate.
