# ADR-0364: Collapse the secondary rule simulation by default

- Status: Accepted
- Date: 2026-10-05
- Deciders: Principal UI Architecture and Principal Low-Latency Blockchain Architecture
- Scope: React rule-governance workspace information density and progressive disclosure

## Context

The Tax Rules workspace has two legitimate operator jobs: author a governed
draft and run a what-if calculation against a saved rule. They are not the
same workflow. Rendering both forms expanded on first load makes the empty
workspace look like two competing tasks, pushes the review record below the
fold, and increases the chance that a simulation is mistaken for publication
or settlement.

The simulation remains useful and must stay discoverable, keyboard-operable,
localized, and backed by the server-owned persisted rule. The change must not
alter authorization, tax inputs, persistence, or calculation behavior.

## Decision

Keep the governed-draft authoring section open because it is the primary task
of the workspace. Render the saved-rule simulation inside a native
`<details>` disclosure that is closed by default. The operator explicitly
opens “Try the saved rule” when a saved rule has been loaded and a controlled
what-if result is needed.

The disclosure remains a normal DOM control rather than a modal or custom
popover. Opening it does not issue a request; the existing submit action still
requires a loaded rule, service availability, and an authenticated session.
Values are preserved while the disclosure is toggled, and the simulation
result remains inside the same review record.

## Alternatives considered

| Alternative | Benefit | Risk | Decision |
| --- | --- | --- | --- |
| Keep both forms expanded | Immediate access to every control | Competing primary tasks and unnecessary first-load density | Rejected |
| Hide simulation entirely until a rule loads | Lowest empty-state density | Makes an important review tool undiscoverable and changes the workflow after loading | Rejected |
| Use a modal dialog | Focused task surface | Adds focus restoration, layering, and evidence-context complexity | Rejected |
| Use custom popover behavior | Compact presentation | More keyboard, mobile, and dismissal failure modes | Rejected |
| **Native collapsed disclosure** | **Discoverable, accessible, bounded, no new overlay state** | **Requires one deliberate operator action** | **Selected** |

## Invariants and failure policy

- The disclosure state is presentation-only and cannot change authorization,
  rule identity, facts, tax results, or publication state.
- No simulation request is made merely because the workspace is opened or the
  disclosure is expanded.
- The submit action remains disabled until the existing service, session, and
  loaded-rule guards pass.
- Keyboard and screen-reader users receive the same summary control and the
  same fields after expansion.
- The empty workspace exposes one primary authoring path and one clearly named
  secondary review path; no technical JSON or protocol identifiers are added
  to the primary surface.

## Verification

- `governance-ui.test.tsx` asserts the simulation starts closed and opens via
  its summary control.
- Existing governance submission and controlled-simulation tests continue to
  verify the unchanged server boundaries.
- `make web-browser-check` and the WCAG axe journey remain release gates.

