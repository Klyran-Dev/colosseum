# ADR-0265: Responsive labeled navigation disclosure

## Status

Accepted. This decision refines ADR-0263 for narrow screens without changing
workspace ownership, authorization, or routing semantics.

## Context

The product shell currently uses a compact icon rail at intermediate widths and
an icon-only horizontal strip on small screens. That pattern is efficient for
experienced users of a developer console, but it hides task names and makes the
product harder to navigate for finance, tax, operations, and support users.
It also weakens keyboard and screen-reader discoverability because a user must
remember the meaning of single-letter icons or rely on hover text.

The shell must remain usable on a 320px viewport, preserve the full task list,
avoid competing with the active workspace, and keep navigation state explicit.

## Decision

Use a responsive labeled navigation disclosure:

- Wide screens keep the persistent labeled sidebar.
- Intermediate desktop widths may use a compact icon rail, but each control
  retains an accessible name and tooltip.
- Narrow screens show the product identity and an explicit “Open navigation”
  control. The task navigation is hidden until opened, then displays its full
  group labels, task names, descriptions, and risk markers.
- Selecting a workspace closes the narrow-screen navigation and preserves the
  existing focus handoff to the committed workspace boundary.
- The disclosure uses native button semantics, `aria-expanded`,
  `aria-controls`, and a stable navigation landmark. It does not rely on hover,
  color, or an icon-only interpretation.

This is a presentation concern only. Server authorization remains the source of
truth, and hiding or showing the navigation never grants access or changes a
workspace's capability decision.

## Alternatives considered

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Keep an icon-only horizontal strip | Minimal vertical space and quick for experts | Poor discoverability, weak task recognition, and inaccessible without remembered icons | Rejected |
| Use a fixed bottom navigation bar | Familiar on mobile and always available | Only supports a few destinations; forces high-risk work into an ambiguous shortcut surface | Rejected |
| Render a full-screen route menu | Maximum room for descriptions | Adds a disruptive transition and obscures the current context | Rejected |
| **Use a labeled disclosure within the existing shell** | Preserves context, supports the complete task catalog, has native semantics, and keeps one navigation model across widths | Requires responsive styling and a small amount of disclosure state | **Selected** |

## Acceptance criteria

1. At narrow widths, a user can discover every primary task without hover,
   memory of icons, or horizontal scrolling.
2. The control exposes its open/closed state to assistive technology and has a
   deterministic accessible name.
3. Selecting a task closes the disclosure and preserves the existing focus
   handoff to workspace content.
4. Desktop navigation remains visible and keyboard-operable.
5. No capability, authorization, API, or wallet behavior is changed by the
   responsive presentation.

## Verification

- Testing Library covers the disclosure state, accessible attributes, and
  close-on-navigation behavior.
- Existing focus, RTL, reduced-motion, and axe-core checks remain green.
- The pinned real-browser journey remains the evidence source for responsive
  overflow and browser-level layout behavior.

