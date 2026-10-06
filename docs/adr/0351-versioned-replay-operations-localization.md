# ADR-0351: Versioned replay-operations localization at the recovery boundary

- Status: Accepted
- Date: 2026-10-05
- Decision owners: Indexing, Site Reliability, Security, Accessibility

## Context

Replay recovery is an operator-only control plane for finalized chain evidence.
It exposes dead-lettered gaps, bounded requeue actions, targeted evidence
recovery, and route-verification run evidence. The workspace accepted locale
and time-zone inputs but rendered the recovery decision, provider uncertainty,
and route-evidence boundary in English. That increased the risk of an operator
misreading a retry as a payment action or treating an empty projection as a
chain-health assertion.

## Decision

Use the typed immutable catalog in
`apps/web/src/replayOpsMessages.ts` for `en-US`, `de-DE`, `ar-EG`, and
`ja-JP`. Localize the recovery control, route-verification disclosure,
targeted recovery form, queue states, lifecycle labels, and bounded recovery
actions. Pass presentation labels through the existing locale and time-zone
props while keeping all server-owned values and protocol identifiers intact.

The catalog is presentation-only. It cannot change stream keys, status values,
slot ranges, signatures, addresses, page budgets, operator notes, request
payloads, evidence IDs, route IDs, approval IDs, or recovery action names.
Raw reason/error codes and chain identifiers remain evidence values; they are
not translated or used as authority.

## Alternatives considered

| Option | Recovery decision clarity | Authority/value isolation | Determinism | Decision |
| --- | --- | --- | --- | --- |
| Keep recovery controls in English | Fails for localized operators | Pass | Pass | Rejected |
| Runtime machine translation | Unstable incident and retry semantics | Partial | Fail | Rejected |
| Reuse refund or settlement catalogs | Wrong lifecycle and evidence vocabulary | Partial | Pass | Rejected |
| **Typed immutable replay-operations catalog** | **Pass** | **Pass** | **Pass** | **Selected** |

## Invariants and budgets

1. Locale changes cannot call the API, alter recovery state, or alter a retry
   or recovery payload.
2. A recovery action never claims payment movement, finalized settlement, route
   activation, or chain health; the distinction remains visible in every
   supported locale.
3. Status labels are presentation mappings only; server status values and
   action discriminators remain unchanged.
4. Time-zone formatting continues to use the existing validated formatter and
   never changes persisted timestamps or slot values.
5. Unknown locales fall back deterministically to `en-US`.
6. Catalog lookup is O(1), static, bounded below 64 KiB, and adds no work to
   API, indexer, or recovery hot paths.

## Verification

- Failing-first catalog and component tests cover supported locales, fallback,
  operator-safe language, and the existing read-only evidence boundary.
- Existing replay parser, authorization, note, bounded action, and route-run
  tests remain unchanged.
- The NGINX browser journey covers German, Arabic RTL, and axe checks without
  changing the route or authorization semantics.
- `make ui-language-contract`, `make web-check`, and
  `make web-browser-check` are required before review.
