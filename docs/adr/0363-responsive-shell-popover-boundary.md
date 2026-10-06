# ADR-0363: Responsive shell popover boundary

## Status

Accepted.

## Context

Operators use the SPA at desktop and narrow browser widths. The local
development sign-in control is a native disclosure, and the shell also has
system-status, display-settings, and account disclosures. The local
credential form was laid out as a flex child beside its trigger. When opened,
it increased the top-bar's intrinsic width and could clip the breadcrumb and
workspace at the viewport edge. Multiple open disclosures could also overlap
on narrow screens.

## Decision

Keep shell disclosures native and keyboard-operable, but make expanded content
anchored popovers. The local credential form is positioned below its trigger,
uses a bounded viewport-relative width, and stacks its note below the field.
The top-bar breadcrumb may shrink with ellipsis, while the action group may
wrap at intermediate widths. Top-bar disclosures are mutually exclusive so a
large panel cannot cover another open panel on a narrow viewport.

The popover remains presentation-only. It does not change authentication
authority, persist credentials, or alter the settlement and tax contracts.

## Scope and non-goals

In scope: ProductShell top-bar geometry, local sign-in presentation, shell
disclosure coordination, responsive overflow regression coverage.

Out of scope: authentication protocol changes, wallet behavior, authorization
decisions, financial calculations, route selection, and server-side session
state.

## Invariants and failure policy

- The document and product shell must not grow wider than the viewport because
  a shell disclosure is open.
- The breadcrumb remains visible and readable at the supported desktop width.
- An expanded local sign-in form begins at or below its trigger and is bounded
  by the viewport.
- Opening one top-bar disclosure closes other top-bar disclosures; closing a
  disclosure never mutates credential or authorization state.
- Native `details`/`summary` semantics and visible focus styles remain intact.
- If layout CSS is unavailable, the controls remain usable in their native
  document order; no JavaScript state is required for authentication.

## Threat model

The browser is an untrusted presentation surface. This change must not expose
the in-memory local credential in a new DOM location, persist it, log it, or
send it anywhere. Popover coordination only removes `open` attributes from
sibling disclosures in the same top-bar; it cannot grant access or submit a
transaction.

## Budgets and acceptance criteria

At a 1440 x 900 dark-mode browser viewport with local sign-in enabled, opening
the local sign-in and display-settings controls must keep document scrollWidth
at or below clientWidth, retain at least 80 CSS pixels of breadcrumb width, and
keep the local form below its trigger. The browser regression runs against the
real Vite SPA. Unit coverage verifies disclosure exclusivity. No production
request, wallet signing, or tax-engine path is changed.

## Alternatives considered

See `docs/architecture/alternative-analysis.md`.

1. Keep expanded forms inline in the flex row: rejected because intrinsic
   width can clip the workspace and creates an unstable desktop layout.
2. Replace disclosures with modal dialogs: rejected because the controls are
   lightweight contextual settings and modal focus management would add more
   interaction cost than the problem requires.
3. Allow multiple popovers and rely on z-index: rejected because overlapping
   panels remain ambiguous on narrow screens and can hide actionable content.
4. **Bounded anchored popovers with one open shell disclosure:** selected
   because it preserves native semantics, keeps the workspace geometry stable,
   and gives small screens a deterministic interaction model.

## Consequences

The shell is more resilient at desktop and narrow widths, and the local
credential panel is easier to scan. Popover content can cover nearby page
content while open, which is expected for contextual controls; only one
top-bar panel is open at a time and its bounded size prevents unbounded
viewport growth. The decision adds a small browser geometry test and a local
presentation regression test.
