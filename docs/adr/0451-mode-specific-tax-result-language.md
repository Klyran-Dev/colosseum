# ADR-0451: Mode-specific tax result language

- Status: Accepted
- Date: 2026-10-09
- Owners: Product engineering, tax domain, and security

## Context

The quote and public simulator workspaces share a result renderer, but they do
not have the same authority model. A quote is based on an approved rule and a
signed-in, service-checked workflow. The simulator is a read-only calculation
from caller-supplied amounts and rate. Reusing quote copy in the simulator
claimed that an approved rule had been applied and suggested preparing a quote,
which could mislead an operator about both authority and available actions.

## Decision

Keep one result renderer and give each mode an explicit, localized presentation
projection for its explanation, rule label, empty state, and disclosure title.
Quote mode may describe approved rules and saved review. Simulator mode must
describe a caller-supplied scenario, must not imply live-rule authority, and
must not advertise the protected quote workflow as an available action.

These strings are presentation-only. Calculation requests, status values,
attestations, rule digests, and server authorization remain unchanged.

## Alternatives considered

1. Keep the shared quote copy. Rejected because it misstates the simulator's
   authority and creates a misleading next action.
2. Duplicate the entire result component for the simulator. Rejected because
   it would create two financial presentation paths with drift risk.
3. **Keep the renderer shared and inject typed mode-specific copy.** Selected
   because it preserves structural and accessibility consistency while making
   authority semantics explicit and testable.

## Acceptance criteria

- The simulator never describes its result as based on an approved tax rule.
- The simulator empty state advertises only the simulator action.
- Quote mode retains approved-rule and saved-review language.
- English, German, Arabic, and Japanese use mode-specific copy.
- Unit, accessibility, and Docker browser gates remain green.
