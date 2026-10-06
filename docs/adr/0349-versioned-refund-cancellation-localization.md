# ADR-0349: Versioned refund-cancellation localization at the governance boundary

- Status: Accepted
- Date: 2026-10-05
- Decision owners: Product, Wallet, Security, Accessibility

## Context

Refund cancellation is a governed money-movement boundary. It does not move
funds, but it changes whether a confirmed refund claim may proceed and requires
an approval-authority wallet signature followed by finalized chain evidence.
The existing workspace exposed cancellation, recovery, signing, provider
acknowledgement, and reconciliation language only in English. That makes a
high-consequence operator decision less clear for supported locales and was
inconsistent with the localized refund-claim and refund-execution surfaces.

## Decision

Use a typed, immutable in-bundle catalog in
`apps/web/src/refundCancellationMessages.ts` for `en-US`, `de-DE`, `ar-EG`,
and `ja-JP`. The catalog owns visible operator language for preparation,
artifact review, governance-wallet signing, recovery, provider uncertainty,
and finalized cancellation evidence. `WalletPicker` receives the same locale
so wallet discovery and account-selection copy remain consistent.

The catalog is presentation-only. It must not translate or transform claim
references, UUIDv7 identifiers, public keys, mint addresses, account
addresses, nonces, signatures, transaction bytes, evidence digests, API
payloads, or server-owned status values. Status values remain the exact API
contract; only their displayed labels may use a bounded locale map.

## Alternatives considered

| Option | Governance clarity | Authority/value isolation | Determinism | Decision |
| --- | --- | --- | --- | --- |
| Keep cancellation in English | Fails for localized operators at an approval boundary | Pass | Pass | Rejected |
| Runtime machine translation | Unstable authorization and recovery wording | Partial | Fail | Rejected |
| Reuse refund-claim catalog | Partial; cancellation has different authority and evidence semantics | Partial | Pass | Rejected |
| **Typed refund-cancellation catalog** | **Pass** | **Pass** | **Pass** | **Selected** |

## Invariants

1. Changing locale has no network, wallet, signing, persistence, or state-machine
   side effect.
2. The browser cannot edit server-owned cancellation facts or authority
   accounts through localization.
3. Provider acknowledgement never becomes finalized cancellation evidence.
4. Finalized event evidence and exact transaction/account binding remain the
   completion boundary.
5. Recovery data remains memory-only and is submitted byte-for-byte unchanged.
6. Unsupported locale values deterministically fall back to `en-US`.
7. Copy does not imply tax filing, legal compliance, or completed movement.

## Performance and accessibility budget

The catalog is a bounded static module with O(1) lookup and no request-path
I/O. It adds no work to the wallet-signing or settlement path and remains
below 64 KiB. The route must retain WCAG 2.2 AA semantics, keyboard order,
focus behavior, RTL direction, reduced-motion behavior, and axe coverage.

## Verification

- Failing-first catalog and German component tests cover all supported locales,
  deterministic fallback, no technical jargon in primary copy, and advanced
  recovery disclosure.
- Existing refund-cancellation parser, signing, submission, and reconciliation
  tests remain unchanged and continue to cover protocol values and bytes.
- The NGINX-served browser journey covers German, Arabic RTL, and axe checks.
- `make ui-language-contract`, `make web-check`, and `make web-browser-check`
  are required before review.

## Consequences

Each new cancellation phrase requires catalog review and tests in every
supported locale. This small maintenance cost is intentional: a governed
wallet decision must not depend on runtime translation or silently diverge
from the rest of the product language system.
