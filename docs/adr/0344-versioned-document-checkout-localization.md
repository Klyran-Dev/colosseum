# ADR-0344: Versioned document-checkout localization

- Status: Accepted
- Date: 2026-10-05
- Deciders: Principal UI Architecture and Principal Low-Latency Blockchain Architecture
- Scope: Invoice payment review, wallet authorization, and settlement-status presentation

## Context

The invoice quote workspace is now localized under ADR-0343, but document
checkout still presents English-only copy across evidence verification, route
selection, amount review, wallet authorization, and post-submit recovery. This
is the highest-consequence remaining workspace because ambiguous language at a
wallet boundary can cause an operator to sign the wrong action or misread an
unknown settlement outcome.

## Decision

Add a typed, immutable, versioned document-checkout catalog for `en-US`,
`de-DE`, `ar-EG`, and `ja-JP`. It covers evidence and route states, wallet
step, payment scope, line-subset guidance, review summaries, signing and
submission states, status refresh/reconciliation actions, expiry, and the
explicit distinction between wallet authorization and finalized settlement.

The reusable wallet picker receives its own small typed catalog rather than
borrowing parent-panel strings. This keeps wallet discovery, account
selection, connection errors, and signing affordances coherent wherever the
control is reused, while allowing the document-checkout catalog to own its
payment-specific semantics.

The catalog is presentation-only. Wallet addresses, route IDs, mints, account
keys, amounts, digests, intent IDs, transaction signatures, status values, and
server-owned evidence remain authoritative values and are never translated or
rewritten by the browser. The signing bytes, request payloads, route registry,
attestation status, and retry/recovery state machine remain unchanged.

## Alternatives considered

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Keep checkout in English | No implementation cost | Inconsistent high-consequence flow and increased signing ambiguity | Rejected |
| Runtime machine translation | Broad apparent coverage | Nondeterministic financial/legal language, privacy/network cost, no reviewable contract | Rejected |
| Reuse document-quote catalog | Fewer strings | Conflates evidence, wallet, signing, and settlement semantics | Rejected |
| **Typed bounded document-checkout catalog plus reusable wallet catalog** | Deterministic, reviewable, local, authority-preserving, and coherent across wallet surfaces | Requires deliberate legal and wallet-copy review | **Selected** |

## Invariants and failure behavior

- Locale selection cannot change route selection, wallet account, scope, nonce,
  amounts, signing bytes, request payloads, authorization, or evidence.
- Unknown locales fall back deterministically to English.
- Evidence required, expired, unavailable, unknown-outcome, retry, and finalized
  status states remain explicit and fail closed in every locale.
- The UI never states that wallet approval proves settlement; only finalized
  chain evidence can establish movement.
- The UI never turns withholding authorization into filing, reporting, legal
  compliance, or tax advice.
- Protocol values remain available in controlled technical disclosures and are
  not localized as if they were user-entered financial data.

## Latency and resource budget

- Static in-bundle lookup is O(1); no network, dynamic import, timer, or wallet
  dependency is introduced.
- The document-checkout catalog must remain under 64 KiB before compression.
- Locale changes cannot initiate route loading, evidence verification, compile,
  wallet discovery, signing, submission, refresh, or reconciliation.

## Verification

- Failing-first catalog tests cover every supported locale, required keys,
  deterministic fallback, and protocol-value isolation.
- Existing evidence, route, compile, signing, recovery, and reconciliation
  tests retain their wire and state-machine assertions.
- Component tests cover translated wallet/evidence/review copy without sample
  financial or wallet values.
- Docker browser coverage verifies German/Arabic checkout presentation, RTL,
  axe, and unchanged route/evidence failure behavior through NGINX.
- Manual wallet-provider, tax/legal, screen-reader, and operator review remain
  release gates.

## References

- [ADR-016: Explainable, accessible tax UI/UX](0016-tax-ui-ux.md)
- [ADR-0272: Payment-review progressive disclosure](0272-payment-review-progressive-disclosure.md)
- [ADR-0330: Server-owned document evidence status](0330-server-owned-document-evidence-status.md)
- [ADR-0343: Versioned document-quote localization](0343-versioned-document-quote-localization.md)
