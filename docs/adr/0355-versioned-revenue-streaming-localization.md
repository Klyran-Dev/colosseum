# ADR-0355: Versioned revenue-streaming operator localization

- Status: Accepted
- Date: 2026-10-05
- Deciders: Principal UI Architecture and Principal Low-Latency Blockchain Architecture
- Scope: React partner-payout/revenue-streaming workspace

## Context

Partner payout plans are a money-movement-adjacent workflow with four-eyes
governance, finalized-balance evidence, bounded release preparation, and
server-owned route references. The workspace still exposed English-only copy
even when the product shell selected German, Arabic RTL, or Japanese. This
creates an operator-review and accessibility gap in a high-consequence surface.

## Decision

Use a typed, immutable, versioned presentation catalog for `en-US`, `de-DE`,
`ar-EG`, and `ja-JP`. Pass the existing shell locale into the panel and keep
all protocol values unchanged: plan IDs, UUIDv7 idempotency keys, status
discriminators, route/reference IDs, Solana addresses, atomic amounts, slots,
digests, request bodies, and authorization capability names.

Localize lifecycle labels, balance summaries, governance actions, setup
guidance, loading/error/empty states, claim preparation language, and the
non-custodial/finalized-evidence boundary. Technical identifiers remain in
progressive disclosure and are never translated or reformatted as authority
inputs.

## Alternatives considered

| Alternative | Operator clarity | Protocol safety | Determinism | Decision |
| --- | --- | --- | --- | --- |
| Keep partner payouts in English | Fails for localized operators | Pass | Pass | Rejected |
| Runtime machine translation | Unstable governance and money-movement wording | Partial | Fail | Rejected |
| Translate statuses, IDs, or request values | Visually consistent | Fail; mutates authority inputs | Fail | Rejected |
| Separate localized payout application | Tailored UX | Duplicates authorization and evidence boundaries | Partial | Rejected |
| **Typed presentation-only catalogs** | **Pass** | **Pass** | **Pass** | **Selected** |

## Invariants and verification

- Locale changes cannot change an API request body, idempotency key, plan
  status, amount, slot, address, digest, or capability decision.
- No prepared release is described as a completed transfer; finalized Solana
  evidence remains authoritative.
- Denied, loading, timeout, empty, stale, and error states remain explicit in
  every supported locale.
- The catalog completeness test, panel test, `make ui-language-contract`,
  `make web-check`, and German/Arabic RTL browser journey with axe checks are
  release gates.

Manual linguistic, screen-reader, tax-owner, and payout-operator review remain
external release evidence.
