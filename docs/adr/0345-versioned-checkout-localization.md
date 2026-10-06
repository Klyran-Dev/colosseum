# ADR-0345: Versioned aggregate-checkout localization

- Status: Accepted
- Date: 2026-10-05
- Deciders: Principal UI Architecture and Principal Low-Latency Blockchain Architecture
- Scope: Aggregate quote payment review, recipient-plan selection, wallet authorization, and settlement status

## Context

The aggregate payment-review workspace is a wallet-signing boundary and still
contains English-only operator copy. The shell, quote, document quote, and
document checkout have bounded locale contracts, but aggregate checkout can
still present mixed-language instructions after a tenant selects German or
Arabic. Mixed language at the amount, route, recipient, and settlement-status
decision points increases signing ambiguity.

## Decision

Add a typed immutable catalog for `en-US`, `de-DE`, `ar-EG`, and `ja-JP`.
The catalog covers route state, payment-sharing mode, recipient-plan state,
safeguards, quote expiry, review summaries, technical disclosures, wallet
approval, and unknown/submitting settlement recovery states.

The catalog is presentation-only. Amounts, currencies, route IDs, merchant
accounts, mints, recipient accounts, basis points, rule identifiers, digests,
intent IDs, signatures, server status values, and request payloads remain
unchanged. It does not translate server-owned errors or infer legal/tax
compliance from a wallet action.

## Alternatives considered

| Alternative | Signing clarity | Authority/value isolation | Determinism | Decision |
| --- | --- | --- | --- | --- |
| Keep aggregate checkout in English | Fail for localized operators | Pass | Pass | Rejected |
| Runtime machine translation | Fail | Partial | Fail | Rejected |
| Reuse document-checkout catalog | Partial | Partial | Pass | Rejected; semantics differ for aggregate splits |
| **Typed aggregate-checkout catalog** | **Pass** | **Pass** | **Pass** | **Selected** |

## Invariants and failure behavior

- Locale changes cannot alter calculation, route, recipient plan, wallet
  account, nonce, amounts, signing bytes, or settlement state.
- Unknown locales fall back deterministically to English.
- Loading, unavailable, expired, incompatible-plan, signing, submitting,
  unknown-outcome, retry, and finalized-state distinctions remain explicit.
- Technical values remain in controlled disclosures and are never translated
  or normalized as if they were financial inputs.
- Wallet approval is never presented as finalized network settlement, and
  withholding is never presented as filing, reporting, legal compliance, or
  tax advice.

## Latency and resource budget

- Catalog lookup is O(1), in-bundle, allocation-bounded, and independent of
  network, wallet discovery, and database state.
- The catalog remains under 64 KiB before compression.
- Locale changes do not initiate quote loading, route loading, plan loading,
  compilation, signing, submission, or reconciliation.

## Verification

- Failing-first tests cover every locale, required keys, fallback, and
  protocol-value isolation.
- Existing aggregate checkout wire/state-machine tests remain unchanged.
- Component tests cover German primary copy without translating values.
- Docker browser coverage verifies German and Arabic RTL presentation plus
  WCAG axe checks and the existing expiry/wallet-rejection paths.
- Manual wallet-provider, legal/tax, screen-reader, and operator review
  remain release gates.

## References

- [ADR-0272: Payment-review progressive disclosure](0272-payment-review-progressive-disclosure.md)
- [ADR-0315: Tax-responsibility language boundary](0315-executable-tax-responsibility-language.md)
- [ADR-0342: Versioned quote workspace localization](0342-versioned-quote-workspace-localization.md)
- [ADR-0344: Versioned document-checkout localization](0344-versioned-document-checkout-localization.md)
