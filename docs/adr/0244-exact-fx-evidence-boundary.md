# ADR-0244: Exact foreign-exchange evidence boundary

## Status

Accepted for implementation.

## Context

Global tax rules are often authored in a tax or reporting currency while a
transaction arrives in another currency or token denomination. The current
tax engine is deterministic once facts are supplied, but it has no typed
boundary for the exchange-rate evidence or the exact conversion semantics.
An untyped conversion would make replay, threshold evaluation, cumulative
periods, and tax explanations ambiguous. A live FX provider in the hot path
would also violate the pure-domain and low-latency boundaries.

## Decision

Add an optional `FxConversionEvidence` envelope to aggregate and document tax
facts. It contains the source and target currency codes, a positive exact
rational rate, provider and quote identifiers, observation timestamp, target
atomic quantum, declared rounding mode, and a non-zero evidence digest.

The facts `currency` remains the target calculation/settlement currency. Raw
gross, net, line, and cumulative-period amounts are expressed in the source
currency when the envelope is present. The pure Rust engine converts each
amount with checked rational arithmetic before rule evaluation. The target
quantum and rounding mode are applied exactly once during normalization.

The original facts digest includes the complete FX envelope, while the
calculation result exposes the same envelope for explanation and replay. The
normalized internal facts clear the envelope so amounts cannot be converted
twice. Missing, malformed, zero-digest, stale/ambiguous, or target-mismatched
evidence fails closed before tax calculation. Provider lookup, freshness
policy, and legal acceptance remain application-owned inputs; the domain does
not perform network calls or consult a clock.

Quote and document-quote endpoints reject caller-supplied FX envelopes with a
stable `CLIENT_FX_CONVERSION_FORBIDDEN` error. A future provider adapter may
attach verified evidence server-side after authenticating the merchant,
validating the source facts, and applying the configured freshness and legal
acceptance policy; browser or API payloads cannot make FX evidence authoritative.

## Alternatives considered

See `docs/architecture/alternative-analysis.md`.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Convert in the browser or API with decimal floating point | Simple integration | Client authority, binary rounding, replay drift, and unverifiable provider state | Rejected |
| Call an FX provider inside the tax engine | Fresh rates | Network latency, nondeterminism, provider outage, and impure domain | Rejected |
| Require all callers to pre-normalize without evidence | Small domain model | Cannot prove rate, rounding, or source currency during replay | Rejected |
| **Typed exact FX evidence plus pure normalization** | Deterministic, auditable, cache-friendly, low-latency, and replayable | Requires upstream provider verification and explicit quote policy | **Selected** |

## Invariants and failure policy

1. The target currency is the facts currency and the rate is positive.
2. Source and target currency codes are bounded and cannot be equal when an FX
   envelope is present.
3. Provider, quote identity, observation time, digest, and conversion policy
   are part of the immutable facts digest.
4. Every converted amount uses checked rational arithmetic and remains within
   the Solana-compatible atomic bound.
5. Cumulative-period snapshots are converted with the same evidence and
   rounding policy as the transaction amount.
6. A calculation never performs conversion more than once.
7. FX evidence is not proof that a tax jurisdiction accepted a rate or that a
   filing obligation was satisfied; those remain separate governed concerns.

## Verification

- Rust unit, property, and replay tests cover exact rates, ties, quantum
  rounding, cumulative snapshots, overflow, malformed evidence, and
  double-conversion prevention.
- TypeScript response tests cover the bounded optional result envelope.
- API boundary tests prove simulation and persisted quote entry points reject
  client-supplied FX evidence before calculation or storage.
- The existing golden tax vectors remain byte-compatible when FX evidence is
  absent.
- Docker, security, and public-documentation checks remain required.
