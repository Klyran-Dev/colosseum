# ADR-0446 — Progressive disclosure for long tax workflows

- Status: Accepted
- Date: 2026-10-09
- Decision owners: Principal UI Architecture, Tax Operations

## Context

The authenticated Tax Rules workspace put the complete rule authoring form and
the line-and-invoice simulator on screen at the same time. Both are valid
workflows, but their combined control count creates a long, high-density page
before an operator has chosen either task. This makes the primary review and
lookup actions harder to find and increases the chance of editing the wrong
surface.

## Decision

Keep the rule authoring and line-and-invoice simulation surfaces in the Tax
Rules workspace, but render each as a closed native disclosure by default.
Opening a disclosure is an explicit operator choice. A successful document
simulation opens its own disclosure so the returned result is immediately
visible. Existing rule lookup, review, and evidence content remain in the
workspace's normal reading order.

## Alternatives considered

- **Render both forms expanded:** Rejected. It preserves discoverability at the
  cost of excessive vertical density and competing primary actions.
- **Move simulation to a separate route:** Rejected. It breaks the useful
  relationship between a loaded rule and its controlled simulation.
- **Use custom accordions or modal forms:** Rejected. Native disclosures give
  keyboard and assistive-technology semantics without introducing another
  focus and dismissal model.

## Invariants and failure behavior

- Closing a disclosure never discards entered facts or a returned result.
- The disclosure is the only visibility boundary; it does not bypass service,
  authentication, authorization, or validation checks.
- A failed simulation does not fabricate a result and leaves the operator able
  to reopen the form and correct the inputs.
- Successful simulation results remain inside the same disclosure and open it
  for review.

## Evidence

- `governance-ui.test.tsx` proves the authoring disclosure starts closed and
  remains usable after an explicit expansion.
- `taxDocumentSimulationPanel.test.tsx` proves the simulation disclosure
  starts closed and opens for the controlled workflow.
- The Docker-served Tax Rules workspace is covered by the real-browser UI and
  accessibility gates.
