# ADR-0484: Editorial operational visual direction

- Status: Accepted
- Date: 2026-10-10
- Owners: Web platform

## Problem

The product shell is functionally structured but visually reads as a uniform
control plane: pale tonal surfaces, repeated equal-weight cards, and a quiet
navigation rail make the primary task and the operational hierarchy harder to
scan. The supplied visual reference demonstrates a more confident pattern:
one strong navigation surface, a bright canvas, generous spacing, expressive
type, and an asymmetric set of purposeful cards.

The reference is inspiration only. Its brand, copy, imagery, labels, metrics,
and terminology are not part of this product and must not be reproduced.

## Decision

Adopt an original editorial-operational shell treatment:

- a high-contrast semantic navigation rail that anchors orientation;
- a bright, low-noise work canvas in the light scheme and a restrained
  equivalent in dark mode;
- stronger page-heading hierarchy and more generous whitespace around the
  operator's current task;
- asymmetric surface hierarchy: the active workflow is primary, evidence and
  access context are secondary, and supporting notices are compact;
- on mobile, the navigation drawer restores full workspace labels and uses a
  readable list alignment; icon-only navigation remains a deliberate compact
  desktop mode, not a fallback for touch users;
- role-derived accent surfaces for real state, never decorative color or fake
  metrics;
- no illustrations, sample activity, placeholder charts, or invented data in
  production workspaces.

The existing domain components, server-owned values, authorization gates,
failure states, and localization contracts remain unchanged. This is a visual
system change, not a change to protocol or workflow authority.

## Alternatives considered

| Option | Benefits | Costs | Decision |
| --- | --- | --- | --- |
| Keep the current uniform tonal shell | Lowest change risk | Retains weak hierarchy and large undifferentiated whitespace | Rejected |
| Copy the reference's dashboard/cards and terminology | Fast visual similarity | Violates product identity, zero-mock requirements, and domain clarity | Rejected |
| Add decorative gradients and imagery to existing panels | Expressive | Adds noise and can imply data that does not exist | Rejected |
| **Use the reference's hierarchy principles with semantic operational surfaces** | **Improves scanability while preserving real workflows and state truth** | **Requires a coordinated shell and surface pass** | **Selected** |

## Invariants and acceptance criteria

- Every visible number, status, label, and evidence value remains sourced from
  the real domain state or an explicit empty/loading/error state.
- Light and dark themes remain contrast-checked and use repository-owned
  semantic roles; component CSS contains no new raw color literals.
- The rail, top bar, page heading, workflow panels, and empty/protected states
  form one consistent hierarchy at desktop, tablet, and 320px widths.
- The change does not alter route contracts, authorization behavior, wallet
  signing behavior, or protocol terminology.
- Browser accessibility, interaction-fidelity, reflow, localization, and
  reduced-motion gates remain green.

## Verification

- Add a stylesheet contract test for the editorial rail/canvas/surface roles.
- Add a responsive stylesheet contract for readable mobile drawer labels.
- Capture the real simulator and protected-workspace states at desktop and
  mobile widths.
- Run frontend tests, typecheck, `git diff --check`, and the Docker browser
  gate.

