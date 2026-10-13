# ADR-0455: Show chain evidence only after submission

- Status: Accepted
- Date: 2026-10-09
- Owners: Product engineering, operations, and observability

## Context

The document payment review rendered the chain-evidence verification card as
soon as a payment artifact was compiled. A prepared artifact has no chain
signature and cannot have finalized account evidence yet. In local and
degraded environments this produced a red “temporarily unavailable” state
beside an otherwise valid approval action, making an expected pre-submission
absence look like a settlement failure.

## Decision

Render the chain-evidence verification card only when the server has returned a
submitted settlement intent. The prepared artifact remains the authority for
wallet review and approval; chain evidence begins only after submission and is
then refreshed through the existing intent-bound verification endpoint.

## Alternatives considered

1. Render the card for prepared artifacts and label it pending. Rejected
   because the current endpoint is intent/stream based and a prepared artifact
   has no chain observation to query; it also adds a competing status block
   before the primary approval action.
2. Render a disabled placeholder card before submission. Rejected because it
   consumes attention without providing actionable evidence.
3. **Render the card after a submitted intent exists.** Selected because it
   preserves the authoritative evidence boundary and keeps pre-signing review
   focused on the exact wallet authorization.

## Acceptance criteria

- Prepared-but-unsubmitted document payment shows no chain-evidence status card.
- A submitted intent still renders the existing pending, verified, or error
  states and retry action.
- The prepared payment remains reviewable and approvable without indexer data.
- Browser accessibility and Docker workflow gates remain green.
