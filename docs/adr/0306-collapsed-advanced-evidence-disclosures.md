# ADR-0306: Collapsed advanced evidence disclosures

## Status

Accepted — implemented in the aggregate and document quote workspaces.

## Context

The quote forms require provider-signed location evidence. That evidence is a
real, required input and must remain available to an authorized operator, but
raw signed JSON is not the primary task. Showing it expanded on first render
made the workspace visually dense, pushed the amount and rule fields below the
fold, and made implementation vocabulary compete with the operator's decision.

The product shell is intended for real financial operations. It must explain
what is needed, preserve a clear path through the task, and keep technical
evidence inspectable without pretending that the evidence is optional.

## Decision

Render the `Location verification` evidence section collapsed by default in
both quote workspaces, using the native accessible `<details>/<summary>`
disclosure. The operator opens it when supplying or reviewing the provider
record. Submission validation remains unchanged: production quotes still
require a non-empty bounded provider-signed evidence set, and the service
remains the authority for evidence validation.

The disclosure is presentation state only. It does not change authorization,
tenant scope, quote inputs, attestation state, retry identity, or settlement
behavior. Real-browser tests explicitly open the disclosure before entering
the required record, and component tests assert that the initial presentation
is collapsed.

## Alternatives considered

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Keep evidence expanded | Lowest interaction cost for a technical operator | Cluttered primary flow; raw JSON dominates the first view | Rejected |
| Remove evidence from the form | Cleaner first view | Breaks the explicit provider-evidence contract and encourages unsafe guessing | Rejected |
| Move evidence into a custom modal | More visual separation | Adds focus, dismissal, validation, mobile, and screen-reader failure modes | Rejected |
| **Native collapsed disclosure** | Keeps the required input discoverable, accessible, and local to the task with minimal runtime cost | Adds one deliberate interaction for the advanced path | **Selected** |

## Invariants

- The location evidence remains a required submission input for production
  quote paths.
- The service, not the browser, validates provider source, account, dates,
  location, and bounded evidence semantics.
- The disclosure is keyboard accessible and exposes its open state to assistive
  technology through native browser semantics.
- No bearer credential, financial amount, rule digest, or authority decision is
  persisted or changed by opening the disclosure.
- The primary view retains plain-language task fields and does not display raw
  signed JSON until the operator requests the advanced section.

## Verification

- `documentQuotePanel.test.tsx` asserts the initial closed state.
- `ui-journeys.spec.ts` opens the disclosure before entering evidence in the
  real browser and continues to assert failure-path and accessibility behavior.
- `make web-browser-check` remains the browser-level gate for the packaged
  NGINX surface.
