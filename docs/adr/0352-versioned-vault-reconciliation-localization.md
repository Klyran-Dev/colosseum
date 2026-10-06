# ADR-0352: Versioned vault-reconciliation localization at the release-stop boundary

- Status: Accepted
- Date: 2026-10-05
- Decision owners: Treasury, Reconciliation, Security, Accessibility

## Context

Vault reconciliation is a read-only release-control surface. It compares
server-owned tax-account balances with finalized chain observations and must
make stale, mismatched, unproven, and empty states unmistakable. The workspace
already accepted locale and time-zone inputs for numeric and timestamp
formatting, but its safety-critical explanatory copy and statuses remained in
English.

## Decision

Use the typed immutable catalog in
`apps/web/src/vaultReconciliationMessages.ts` for `en-US`, `de-DE`, `ar-EG`,
and `ja-JP`. Localize the read-only balance workspace, source selection,
evidence detail, release-stop warnings, history states, and bounded API
messages. Continue to use the validated locale/time-zone formatter for amounts
and timestamps.

The catalog is presentation-only. It cannot change atomic balances, slots,
account or mint addresses, route IDs, run IDs, error codes, server statuses,
request URLs, or release authority. Raw chain/account identifiers and error
codes remain evidence values and are never translated.

## Alternatives considered

| Option | Release-stop clarity | Authority/value isolation | Determinism | Decision |
| --- | --- | --- | --- | --- |
| Keep reconciliation in English | Fails for localized treasury operators | Pass | Pass | Rejected |
| Runtime machine translation | Unstable incident and release-stop semantics | Partial | Fail | Rejected |
| Reuse replay or settlement catalogs | Wrong balance and reconciliation vocabulary | Partial | Pass | Rejected |
| **Typed immutable vault-reconciliation catalog** | **Pass** | **Pass** | **Pass** | **Selected** |

## Invariants and budgets

1. Localization cannot call the API, alter selection state, or infer a matched
   status from an empty, stale, mismatched, or unproven response.
2. A reconciliation result never authorizes release, clears an alert, or
   implies custody or legal compliance.
3. Exact financial strings, slots, identities, statuses, and error codes remain
   unchanged; only labels and explanatory copy are localized.
4. Amount and timestamp formatting remains bounded and uses the validated
   locale/time-zone path already used by the workspace.
5. Unknown locales fall back deterministically to `en-US`.
6. Catalog lookup is O(1), static, bounded below 64 KiB, and adds no work to
   reconciliation or release hot paths.

## Verification

- Failing-first catalog and component tests cover supported locales, fallback,
  read-only language, empty-state handling, and release-stop messaging.
- Existing exact parser, status, amount, URL, and reconciliation tests remain
  unchanged.
- The accessibility suite covers the workspace with the explicit locale.
- A German/Arabic RTL NGINX browser journey and axe audit are required before
  review.
