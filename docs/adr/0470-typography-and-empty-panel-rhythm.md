# ADR-0470: Establish a readable type stack and compact empty-panel rhythm

## Status

Accepted

## Context

The product shell had a tokenized Material type scale, but the effective font
stack depended on whether `Inter` happened to be installed. The main heading
and panel headings also had no shared wrapping rule, and status pills could
compete with long localized titles in a non-wrapping heading row. Finally,
empty result panels inherited the generic empty-state minimum height, leaving
large unused areas beside active operational forms.

Together these details made the UI read as a collection of decorative cards
rather than a focused operations workspace.

## Decision

Define an explicit cross-platform sans stack with macOS display fallbacks,
enable optical sizing and tabular numerals for financial/protocol values, and
use balanced heading wrapping with pretty body-copy wrapping. Panel headings
may wrap when their status chip needs space. Use the existing large semantic
corner token for panels and reduce the minimum height of empty siblings within
multi-panel workspaces to 12rem; standalone error and empty routes retain
their existing centered presentation.

## Alternatives considered

- Download a webfont at runtime: rejected because it adds network dependence,
  privacy/cache variance, and a new supply-chain/licensing surface.
- Keep the current stack and tune individual headings: rejected because the
  result would remain platform-dependent and would not establish a reusable
  hierarchy.
- Remove empty-state minimum height globally: rejected because standalone
  empty and error workspaces need a stable, deliberate landing surface.
- Keep large rounded panels and empty siblings: rejected because the repeated
  shape and unused vertical space amplify the prototype/card-grid feeling.

## Consequences

- Typography is more consistent across the macOS development shell and Linux
  Docker browser gate.
- Long headings and localized status labels remain readable without overlap.
- Result panels become scannable while a neighboring workflow is active.
- The change remains CSS-only and consumes existing semantic tokens.

## Verification

- Material shell contract test for the type stack, wrapping, panel geometry,
  and compact empty rhythm.
- Full frontend typecheck and unit suite.
- Docker browser reflow, touch, localization, and WCAG coverage.
