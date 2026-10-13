# ADR-0469: Anchor mobile shell popovers to the top bar

## Status

Accepted

## Context

The shell exposes email sign-in, display settings, and system-status details as
anchored popovers. On mobile, the responsive shell intentionally removes the
sticky positioning from the top bar and makes the individual controls static so
their popovers can share the top-bar width. The popover contents remained
absolutely positioned, however, without a positioned containing block. As a
result, opening email sign-in placed the form below the viewport; the field was
present in the accessibility tree but not visually available until focus caused
the page to scroll.

This is a broken production interaction: a user can activate the sign-in
control and still be unable to see the form they need to complete the flow.

## Decision

At mobile widths, keep the controls themselves in the top-bar flow but make the
product top bar a positioned stacking context. Its existing popovers then use
the visible top bar as their containing block and remain above the workspace.
The change is layout-only and continues to use the existing semantic surface
and elevation tokens.

## Alternatives considered

- Make each popover a fixed-position dialog: rejected because it would require
  viewport-specific offsets, add focus-management complexity, and detach the
  control from its owning shell surface.
- Render popovers inline in the mobile top-bar grid: rejected because opening a
  form would move the page header and workspace, making the shell jump and
  changing the established desktop interaction model.
- Keep the current static mobile top bar: rejected because the form is
  visually unreachable without an incidental focus scroll.

## Consequences

- Mobile shell popovers open below the top-bar controls and remain visible.
- The workspace geometry does not change when a popover opens.
- The top bar receives a bounded stacking order so its menus appear above the
  workspace content.
- Browser coverage must verify both visibility and narrow-screen reflow for
  shell popovers.

## Verification

- Contract test: `materialExpressiveShellContract.test.ts` ADR-0469 rule.
- Real-browser mobile interaction and WCAG/reflow suites.
