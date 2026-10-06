# ADR-0347: Versioned refund-claim localization at the wallet boundary

- Status: Accepted
- Date: 2026-10-05
- Deciders: Principal UI Architecture and Principal Low-Latency Blockchain Architecture
- Scope: Refund-request preparation, wallet authorization, evidence review, and finalized reconciliation

## Context

The refund-request workspace is a high-consequence wallet surface. It loads a
server-owned approved correction, presents immutable refund evidence, collects
two distinct wallet signatures, submits unchanged transaction bytes, and
requires finalized event evidence before a refund is treated as complete. The
shell, quote, checkout, Treasury, and document surfaces have bounded locale
contracts, but this workflow remains English-only.

Mixed-language instructions at the proposer/refund-funder handoff can create
signing ambiguity. The UI must explain custody, proof, and recovery in plain
language without translating protocol values or implying that a wallet
signature is finalized movement, legal compliance, filing, or tax advice.

## Decision

Use a typed immutable in-bundle catalog for `en-US`, `de-DE`, `ar-EG`, and
`ja-JP`. The catalog covers refund preparation, immutable artifact review,
wallet-role actions, recovery disclosure, finalized evidence, reconciliation,
and non-custodial/legal boundaries. Reuse the existing WalletPicker catalog
through its locale prop.

The catalog is presentation-only. It cannot modify claim IDs, correction IDs,
amounts, mints, destinations, account keys, signatures, transaction bytes,
status values, evidence digests, request bodies, or reconciliation decisions.
Server-owned errors and status values remain authoritative and are not
machine-translated in this slice.

## Alternatives considered

| Alternative | Signing clarity | Authority/value isolation | Determinism | Decision |
| --- | --- | --- | --- | --- |
| Keep refund claims in English | Fails for localized operators | Pass | Pass | Rejected |
| Runtime machine translation | Unstable custody and recovery wording | Partial | Fail | Rejected |
| Reuse checkout catalog | Partial; refund roles and proof differ | Partial | Pass | Rejected |
| **Typed refund-claim catalog** | **Pass** | **Pass** | **Pass** | **Selected** |

## Invariants and failure behavior

- Locale changes never trigger preparation, signing, submission, or
  reconciliation requests.
- Wallet roles, account addresses, immutable artifact bytes, financial values,
  claim identifiers, and server status values remain unchanged.
- The proposer and refund-funder signature steps remain distinct and ordered.
- Provider acknowledgement remains transport evidence only; finalized typed
  event and balance evidence remain the completion boundary.
- Recovery data remains advanced, explicitly temporary, and unchanged when
  submitted for verification.
- Unknown locales fall back deterministically to English.

## Latency and resource budget

- Catalog lookup is O(1), in-bundle, allocation-bounded, and independent of
  wallet, RPC, API, and database state.
- The catalog remains below 64 KiB before compression.
- Locale changes add no network request and do not alter the signing or
  settlement critical path.

## Verification

- Failing-first tests cover every supported locale, required keys, fallback,
  and protocol-value isolation.
- Component tests cover German primary copy with the service unavailable and
  signing disabled.
- Browser tests cover German and Arabic RTL refund-request presentation and
  axe checks while preserving the existing finalized-evidence journey.
- Existing wallet signing, unchanged-byte submission, and reconciliation tests
  remain protocol assertions and are not weakened.

## References

- [ADR-0314: Non-custodial wallet source contract](0314-non-custodial-wallet-source-contract.md)
- [ADR-0315: Tax-responsibility language boundary](0315-executable-tax-responsibility-language.md)
- [ADR-0330: Server-owned document evidence status](0330-server-owned-document-evidence-status.md)
- [ADR-0346: Versioned Treasury and vault-release localization](0346-versioned-vault-release-localization.md)
