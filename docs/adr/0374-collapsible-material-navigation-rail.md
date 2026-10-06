# ADR-0374: Collapsible Material 3 navigation rail

- Status: Accepted
- Date: 2026-10-05
- Owners: Product UI, accessibility, platform

## Context

The product shell contains many legitimate work areas, but a persistent full
navigation label, account context, and operational controls compete with the
active financial workflow. The shell must remain understandable for new
operators while giving experienced operators more canvas for payment, tax, and
evidence review. Access decisions must remain available to assistive
technology, and the compact state must not hide the current workspace or
change authorization behavior.

## Decision

Add an explicit, user-controlled compact navigation rail for desktop widths.
The expanded rail remains the default for discoverability. Compact mode:

- keeps the active workspace, group disclosure, icon, and accessible name;
- hides repeated prose, account context, legal copy, and counts visually while
  retaining semantic labels and keyboard tooltips;
- uses the existing Material 3 Expressive semantic roles and shape tokens only;
- is a presentation preference, not a security boundary, and does not change
  route availability or server-owned RBAC/PBAC/ABAC decisions;
- is not applied to the mobile navigation disclosure, where labels remain
  visible for touch and screen-reader orientation;
- supports reduced motion and forced-colors modes through the existing shell
  contracts.

The preference is held in component state for the current session. Durable
preference storage is deferred until cross-device account settings have an
explicit privacy and tenancy decision.

## Alternatives considered

1. Keep the permanent expanded sidebar. Rejected: it preserves discoverability
   but leaves too little room for dense evidence and settlement workflows.
2. Remove navigation labels and ship an icon-only rail. Rejected: it harms
   first-use orientation and makes the product dependent on tooltips.
3. Use a responsive icon-only breakpoint. Rejected: viewport width is not a
   reliable proxy for user preference and creates a discontinuity during window
   resizing.
4. Use a user-controlled compact rail with semantic labels preserved.
   Selected: it reduces visual density without hiding information from
   assistive technology or changing authorization semantics.

## Invariants and verification

- Every navigation control remains keyboard reachable and has an accessible
  localized name.
- Compact mode exposes an accessible expand control and the active workspace
  remains discoverable from the current page heading and breadcrumb.
- No raw color, gradient, opacity blend, or workspace-specific hue is added.
- Expanded and compact states use the same navigation data and capability
  evaluation.
- Unit/UI, accessibility, typecheck, build, and real-browser checks remain
  required. Manual RTL, reduced-motion, and assistive-technology review remain
  release gates.

## Consequences

The desktop workspace gains usable horizontal space and a calmer visual
hierarchy. Operators must expand the rail to read navigation labels, so the
default remains expanded and the button carries a clear localized label. The
session-only preference avoids persisting a potentially sensitive workspace
layout choice before account-level settings are designed.
