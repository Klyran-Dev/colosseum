# ADR-0250: Deterministic FX provider agreement

## Status

Accepted for implementation.

## Context

ADR-0249 restricts quote candidates to policy-approved provider identities, but
provider availability and disagreement still need an explicit authority rule.
Selecting the first response, averaging rates, or silently dropping a
malformed response can produce a tax amount that cannot be replayed or
explained. A production system needs bounded multi-provider admission without
putting provider network calls in the tax-engine hot path.

## Decision

Add `minimum_provider_agreement` to `FxProviderPolicy` and a pure candidate-set
selection operation. The operation accepts at most the policy provider bound,
validates every candidate against the same policy and time/reference inputs,
requires distinct approved provider identities, and requires at least the
configured number of providers.

For agreement, candidates must have identical source and target currencies,
exact numerator and denominator, rounding mode, and atomic quantum. Provider
quote IDs, observation windows, and evidence digests may differ because they
are source provenance. If economic terms disagree, selection fails closed;
the engine does not average, choose a median, or silently discard a provider.

When multiple agreeing candidates exist, the selected candidate is the first
provider in the immutable `allowed_providers` priority order. Input order is
never an authority boundary. Issuance exposes a candidate-set method that
selects first and then runs the existing server-owned signing path.

The provider adapter remains responsible for bounded concurrent acquisition,
deadlines, cancellation, and provider-specific transport. This ADR defines
only the pure, deterministic admission boundary after candidates arrive.

## Alternatives considered

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| First successful provider | Lowest latency and simplest implementation | Provider order and transient outages change tax results; no disagreement evidence | Rejected |
| Average or median provider rates | Tolerates small market differences | Rounding/ordering ambiguity, can hide an incorrect provider, and requires new legal semantics | Rejected |
| Silently drop invalid or disagreeing candidates | Higher availability | Masks provider faults and weakens auditability | Rejected |
| Require exact economic agreement and select policy priority | Deterministic, replayable, bounded, and conservative | Lower availability when providers diverge; requires explicit policy versions | **Selected** |

## Invariants and failure policy

1. The candidate set is bounded and contains no duplicate provider identity.
2. Every candidate is validated; invalid, stale, unapproved, or malformed
   candidates fail the operation rather than being ignored.
3. Agreement requires exact economic equality for currencies, rate, rounding
   mode, and quantum.
4. Fewer than `minimum_provider_agreement` valid distinct providers fails
   closed.
5. Selection is independent of input order and follows policy provider order.
6. Provider acquisition never occurs in the pure selector or tax engine.

## Verification

- Failing-first tests cover input-order independence, insufficient agreement,
  duplicate identities, exact-rate disagreement, and successful priority
  selection.
- Existing single-provider policies remain valid with an agreement count of
  one.
- Provider transport/load/failover, regional SLO, legal ownership, and
  production evidence remain separate release gates.
