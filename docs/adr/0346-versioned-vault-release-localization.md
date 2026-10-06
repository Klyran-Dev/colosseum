# ADR-0346: Versioned Treasury and vault-release localization

- Status: Accepted
- Date: 2026-10-05
- Deciders: Principal UI Architecture and Principal Low-Latency Blockchain Architecture
- Scope: Treasury release proposal, governed lifecycle transitions, evidence review, and authority-boundary messaging

## Context

The Treasury/vault-release workspace is a high-consequence operator surface. It
creates governed release requests and records approval, ledger, completion,
cancellation, and uncertainty evidence, but it does not sign or move funds.
The shell and adjacent checkout workspaces already provide bounded language
catalogs. Leaving Treasury in English creates mixed-language instructions at a
money-movement decision boundary and makes the distinction between a recorded
request and finalized ledger evidence harder to understand.

## Decision

Use a typed, immutable in-bundle catalog for `en-US`, `de-DE`, `ar-EG`, and
`ja-JP`. The catalog covers proposal, verified-route, lifecycle, transition,
evidence, unavailable-service, and authority-boundary presentation copy.

The catalog is presentation-only. It cannot translate or modify route IDs,
destination accounts, mints, token programs, nonce values, amounts, timestamps,
digests, transaction references, slots, request payloads, server-owned claim
statuses, or transition values. Server/client operational errors remain
authoritative service messages; localization must never hide an error code or
change its recovery meaning.

Treasury release remains a two-person governed workflow. The browser records
requests and evidence only. A separately approved operator prepares, signs,
submits, and reconciles the ledger transaction; confirmed ledger evidence—not
the page or its localized copy—decides whether funds moved.

## Alternatives considered

| Alternative | Operator clarity | Authority/value isolation | Determinism | Decision |
| --- | --- | --- | --- | --- |
| Keep Treasury in English | Fails for localized operators at a money boundary | Pass | Pass | Rejected |
| Runtime machine translation | Unstable terminology and recovery wording | Partial | Fail | Rejected |
| Reuse checkout catalog | Partial; proposal and evidence semantics differ | Partial | Pass | Rejected |
| **Typed Treasury-specific catalog** | **Pass** | **Pass** | **Pass** | **Selected** |

## Invariants and failure behavior

- Unknown locales fall back deterministically to English.
- Locale changes do not initiate route loading, claim loading, mutation,
  signing, submission, or reconciliation.
- Protocol values, financial values, identities, evidence references, and
  server-owned statuses remain byte-for-byte unchanged.
- `proposed`, `approved`, `onchain_created`, `executed`, `cancelled`, and
  `unknown` remain distinct state-machine values even when their visible labels
  are localized.
- Required evidence, two-person approval, and fail-closed unavailable states
  remain explicit in every supported locale.
- The interface never claims tax filing, reporting, legal compliance, or tax
  advice, and never presents a recorded request as a completed transfer.

## Latency and resource budget

- Catalog selection is O(1), in-bundle, allocation-bounded, and independent of
  network, database, wallet, and service state.
- The catalog must remain below 64 KiB before compression.
- Locale switching must not add a network request or a measurable change to the
  release proposal/transition critical path.

## Verification

- Failing-first tests cover all supported locales, required keys, and fallback.
- Component tests verify German copy while preserving disabled/offline behavior.
- Browser tests verify German and Arabic RTL Treasury navigation, headings,
  action labels, and axe accessibility checks.
- Existing release payload, route-selection, claim-reload, and authority-boundary
  tests remain unchanged in their protocol assertions.
- Manual operator, screen-reader, legal/tax, and supported-terminology review
  remain release gates.

## References

- [ADR-0324: Server-owned vault-release route selection](0324-server-owned-vault-release-route-selection.md)
- [ADR-0331: Canonical authorization policy catalog](0331-canonical-authorization-policy-catalog.md)
- [ADR-0341: Versioned shell chrome and session localization](0341-versioned-shell-chrome-and-session-localization.md)
- [ADR-0345: Versioned aggregate-checkout localization](0345-versioned-checkout-localization.md)
