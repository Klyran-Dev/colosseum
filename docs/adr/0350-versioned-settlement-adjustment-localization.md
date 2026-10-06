# ADR-0350: Versioned settlement-adjustment localization at the correction boundary

- Status: Accepted
- Date: 2026-10-05
- Decision owners: Product, Finance Operations, Wallet, Security, Accessibility

## Context

Settlement adjustments are an operator-controlled correction boundary. They
cover refunds, reversals, chargebacks, remittances, and corrections; require
exact signed financial deltas and evidence; and may later create a
wallet-bound, record-only on-chain evidence instruction. The existing
workspace presented these high-consequence choices only in English and mixed
plain-language review with protocol-heavy evidence labels.

## Decision

Use a typed immutable catalog in
`apps/web/src/settlementAdjustmentMessages.ts` for `en-US`, `de-DE`, `ar-EG`,
and `ja-JP`. Translate the operator workflow, approval decisions, recovery,
wallet evidence, ambiguous-provider state, and finalized evidence disclosure.
Pass the active locale to `WalletPicker` and preserve exact protocol values.

The catalog is presentation-only. It cannot change adjustment kinds, signed
financial strings, UUIDv7 identifiers, byte arrays, evidence digests, account
keys, chain intent IDs, status values, request payloads, or transition guards.
Server-owned status values remain unchanged; only their visible labels are
localized.

## Alternatives considered

| Option | Financial review clarity | Authority/value isolation | Determinism | Decision |
| --- | --- | --- | --- | --- |
| Keep adjustments in English | Fails for localized finance operators | Pass | Pass | Rejected |
| Runtime machine translation | Unstable correction and wallet wording | Partial | Fail | Rejected |
| Reuse refund or checkout catalogs | Mismatched correction lifecycle and evidence semantics | Partial | Pass | Rejected |
| **Typed settlement-adjustment catalog** | **Pass** | **Pass** | **Pass** | **Selected** |

## Invariants and budgets

1. Locale changes cannot invoke the API, wallet, signer, persistence, or
   lifecycle state machine.
2. Financial facts, evidence, account identities, signed bytes, and status
   values remain byte-for-byte and value-for-value unchanged.
3. Approval and record-only chain evidence never imply payment, filing, legal
   compliance, or tax advice.
4. Provider acknowledgement remains transport evidence; finalized typed event
   and raw evidence matching remains the completion boundary.
5. Recovery/signing material remains memory-only and unchanged.
6. Unknown locales fall back deterministically to `en-US`.
7. Catalog lookup is O(1), static, bounded below 64 KiB, and adds no work to
   the settlement or wallet-signing hot path.

## Verification

- Failing-first catalog and component tests cover supported locales, fallback,
  empty financial fields, and technical-term exclusion from primary copy.
- Existing adjustment parser, API, exact-string delta, signed-message, and
  finalized evidence tests remain unchanged.
- The NGINX browser journey covers German, Arabic RTL, and axe checks.
- `make ui-language-contract`, `make web-check`, and
  `make web-browser-check` are required before review.
