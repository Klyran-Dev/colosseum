# ADR-0304: Static contrast contract for Material color roles

## Status

Accepted — implemented as a repository and CI gate.

## Context

Material 3 Expressive provides semantic roles, but a role-based stylesheet can
still regress if a future palette edit pairs a foreground and surface with
insufficient contrast. Browser axe checks cover rendered journeys and can miss
unvisited role combinations, while a visual review is not deterministic or
cheap enough to be the only merge gate.

## Decision

Add a dependency-free Node contract that parses the repository-owned dark and
light Material token blocks, computes WCAG relative luminance and contrast, and
requires a minimum 4.5:1 ratio for the text role pairs used by backgrounds,
surfaces, containers, primary/secondary/tertiary actions, warning, and error
states. The gate runs as `make material-palette-contract`, is part of `make
check`, and runs in the web CI contract before tests and build.

The contract checks only literal token declarations. Component rules continue
to consume semantic variables and remain covered by the existing raw-color
contract and rendered axe journeys. The custom `warning` roles are retained as
explicit product semantics and receive the same contrast requirement.

## Alternatives considered

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Manual visual review only | Human judgment and expressive flexibility | Non-repeatable; misses unused role pairs and regressions | Rejected |
| Browser axe only | Tests rendered output and interaction states | Does not guarantee every token pairing is exercised | Rejected |
| Third-party color dependency | More color utilities | Extra supply-chain surface for a small deterministic calculation | Rejected |
| **Dependency-free token contrast contract plus browser axe** | Deterministic, fast, complete for declared role pairs, complementary to runtime review | Must update the role matrix when new semantic roles are introduced | **Selected** |

## Invariants and failure policy

- Dark and light token blocks must both exist and define every required role.
- Required colors are six-digit hex declarations; malformed values fail closed.
- Every declared text pairing must meet at least 4.5:1 contrast.
- The contract never authorizes actions, changes financial values, or replaces
  manual accessibility and assistive-technology review.
- Adding a new semantic foreground/surface role requires adding its pair to the
  contract in the same change.

## Verification

- `make material-palette-contract` passes for the current dark and light
  palettes.
- `make check` and the pinned web CI job execute the contract.
- `make web-browser-check` and axe tests remain required for rendered behavior.
