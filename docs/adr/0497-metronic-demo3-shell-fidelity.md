# ADR-0497: Metronic Demo 3 shell fidelity pass

## Status

Accepted — 2026-10-11

## Context

ADR-0490 established the licensed Metronic 9.5.0 Demo 3 layout contract, but
the first adapter kept product branding in the rail and placed several product
status surfaces outside the Demo 3 hierarchy. At desktop widths this made the
rail appear blank when compact, hid the product identity from the header, and
made the page read like a legacy dashboard rather than the licensed reference.

The shell must remain a React 19/Vite SPA and must preserve the existing
authorization, accessibility, and repository-owned Material semantic color
contracts. This is a composition and spacing correction, not a change to
authentication or money-moving behavior.

## Decision

Align the adapter to the package's actual Demo 3 composition:

- keep the product name in the fixed header while letting the compact rail own
  the product mark, so the header does not render a duplicate logo;
- keep the left rail icon-only at desktop widths, with visible tooltips/titles,
  a bordered active icon state, and no hidden interaction target;
- keep the current workspace group as a horizontal secondary navbar;
- keep the current workspace inside one rounded, bordered content canvas;
- keep system status, display settings, identity, and sign-in as compact header
  controls, with the account menu owning authenticated identity details;
- use the licensed package's layout relationships and typography rhythm while
  retaining repository-owned semantic tokens and domain content.

Use the package's branded authentication layout for `/auth/signin` and
`/auth/signup`: a focused form card paired with a calm product-context panel
on desktop, stacked responsively on mobile. The panel explains the security
boundary without adding another sign-in action or implying legal compliance.

The existing navigation collapse control remains available as an explicit
accessibility and large-screen preference, but it is visible and positioned in
the header rather than being an invisible rail overlay.

## Alternatives considered

- Keep the current shell and only change colors: rejected because the problem
  is hierarchy and density, not palette alone.
- Copy Metronic's full demo runtime and component dependencies: rejected because
  it would introduce unrelated demo routes, providers, and dependencies into
  the production SPA.
- Replace the icon rail with a full labeled sidebar: rejected because Demo 3's
  distinguishing contract is the compact rail plus horizontal navbar.
- Remove product context from the header entirely: rejected because a compact
  rail without a persistent product identity is not usable at a glance.

## Invariants

- No auth, tenant, role, capability, or protected-workspace behavior changes.
- No raw component color literals are introduced outside scheme declarations.
- Every rail control remains keyboard reachable and has an accessible name.
- The mobile layout expands navigation labels and remains responsive.
- The content canvas, navbar, and header maintain stable landmarks for existing
  browser and accessibility tests.

## Verification

- Product-shell tests assert the header brand, visible rail control, active
  icon state, secondary navbar, and rounded content canvas.
- TypeScript, frontend unit tests, stylesheet contract tests, and Playwright
  desktop/mobile journeys remain green.
- Browser screenshots at the local Docker edge are reviewed against the
  licensed Demo 3 shell contract.
