# ADR-0348: Versioned refund-execution localization at the final release boundary

- Status: Accepted
- Date: 2026-10-05
- Deciders: Principal UI Architecture and Principal Low-Latency Blockchain Architecture
- Scope: Final refund release preparation, two-wallet authorization, submission, and proof-pending/finalized states

## Context

The refund-execution workspace is the final wallet authorization surface before
an approved refund can move. It loads a finalized request, prepares a
server-owned immutable release artifact, requires the final approver and refund
account owner, submits unchanged bytes, and distinguishes provider acknowledgement
from finalized movement evidence. It is currently English-only.

Localization at this boundary must improve operator comprehension without
changing the release artifact, account topology, wallet roles, protocol status,
or completion claim. The UI must never present a submitted transaction as a
completed refund or as tax/legal compliance.

## Decision

Use a typed immutable in-bundle catalog for `en-US`, `de-DE`, `ar-EG`, and
`ja-JP`. The catalog covers release preparation, immutable artifact review,
final-approver/refund-funder roles, recovery, proof-pending status, and
finalized movement evidence. Reuse the existing localized WalletPicker.

The catalog is presentation-only. Claim IDs, release IDs, amounts, mints,
destinations, account keys, signatures, transaction bytes, status values,
evidence digests, and request payloads remain unchanged. Server-owned errors
and status values remain authoritative.

## Alternatives considered

| Alternative | Signing clarity | Authority/value isolation | Determinism | Decision |
| --- | --- | --- | --- | --- |
| Keep final release in English | Fails at the irreversible release boundary | Pass | Pass | Rejected |
| Runtime machine translation | Unstable signing and recovery language | Partial | Fail | Rejected |
| Reuse refund-claim catalog | Partial; final-approver semantics differ | Partial | Pass | Rejected |
| **Typed refund-execution catalog** | **Pass** | **Pass** | **Pass** | **Selected** |

## Invariants and failure behavior

- Locale changes do not prepare, sign, submit, reload, or reconcile an
  execution artifact.
- Final approver and refund-funder signatures remain distinct and ordered.
- Provider acknowledgement remains proof-pending; finalized transaction-bound
  event and balance evidence remain the completion boundary.
- Wallet recovery data stays advanced, in memory, and unchanged on submission.
- Unknown locales fall back deterministically to English.
- No localized copy implies tax filing, legal compliance, or completed movement.

## Latency and resource budget

- Catalog lookup is O(1), in-bundle, allocation-bounded, and independent of
  wallet/RPC/API state.
- The catalog remains below 64 KiB before compression and adds no network
  request to the signing path.

## Verification

- Failing-first catalog and component tests cover all supported locales,
  fallback, and presentation/value isolation.
- Existing two-signature, unchanged-byte, proof-pending, and finalized-state
  tests remain unchanged in their protocol assertions.
- Browser tests cover German, Arabic RTL, and axe checks through NGINX.

## References

- [ADR-0314: Non-custodial wallet source contract](0314-non-custodial-wallet-source-contract.md)
- [ADR-0347: Versioned refund-claim localization](0347-versioned-refund-claim-localization.md)
