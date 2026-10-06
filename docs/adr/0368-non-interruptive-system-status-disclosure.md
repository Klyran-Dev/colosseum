# ADR-0368: Non-interruptive system-status disclosure

- Status: Accepted
- Date: 2026-10-05
- Deciders: Principal UI Architecture, Accessibility, and Operations
- Scope: Material 3 Expressive application-shell status messaging

## Context

The shell can surface several independent conditions at once: an unconfigured
deployment or network label, an unavailable API, an expired access snapshot,
and a workspace-specific financial failure. Treating every condition as an
assertive `role="alert"` creates competing announcements and makes the primary
task harder to scan. A collapsed system-status disclosure already provides the
summary and count; its expanded details are context, not an immediate action
that should interrupt the operator.

The financial workspace must continue to use an assertive alert for an
actionable failure such as a rejected quote, failed compilation, or unknown
settlement outcome. System context must remain visible and accessible, but it
must not compete with that recovery message.

## Decision

Keep the warning content inside the expandable system-status panel as ordinary
disclosure text, without a live-region role. The summary's accessible name and
attention count are the primary announcement for system context. Preserve
`role="alert"` for actionable workspace failures and other urgent,
user-directed recovery states. The shell's separate polite workspace-navigation
live region remains the only generic status live region.

This is presentation-only. It does not change service health, authorization,
settlement, retry, or release authority. The warning remains in the DOM and is
available after the operator expands the disclosure. Locale catalogs and the
Material semantic warning roles remain unchanged.

## Alternatives considered

| Option | Interruption control | Discoverability | Accessibility semantics | Decision |
| --- | --- | --- | --- | --- |
| Assertive `role="alert"` for every warning | Fail; competing announcements | Pass | Misrepresents context as urgent action | Rejected |
| Remove the warning from the DOM | Pass | Fail; hides operational context | Fail | Rejected |
| Polite `role="status"` inside the disclosure | Partial; duplicates the navigation status region | Pass | Semantically valid but noisy in this shell | Rejected |
| **Ordinary disclosure text with no live-region role** | **Pass** | **Pass** | **Matches contextual content** | **Selected** |
| Custom live-region queue | Partial | Partial | Higher lifecycle and localization risk | Deferred |

## Invariants

- System configuration warnings never suppress or replace a workspace failure.
- The warning remains visible when the system-status disclosure is expanded.
- Only reviewed, localized copy is rendered; no raw exception or provider text
  is introduced.
- The change cannot authorize, retry, sign, submit, reconcile, or declare a
  financial operation complete.
- The shell continues to expose keyboard-operable disclosure and a
  color-independent warning state.

## Verification and rollout

- A failing-first shell test proves the system warning has no live-region role
  and that the shell retains only its dedicated workspace status region.
- The full frontend test, typecheck, build, Material palette, forced-colors,
  UI-language, and real-browser accessibility gates must remain green.
- Manual screen-reader and operator review remain release evidence gates.
