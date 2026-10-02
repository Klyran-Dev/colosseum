# ADR-0254: Deterministic FX acquisition outcome order

- Status: Implemented
- Date: 2026-10-02
- Scope: Ordering of bounded asynchronous FX-provider acquisition results

## Context

ADR-0252 deliberately completes provider calls concurrently. Tokio task
completion order is therefore a function of network latency and scheduler
timing, not policy priority. Returning candidates and failure classifications
in that order makes an otherwise equivalent acquisition look different across
replays. That weakens audit comparison, telemetry correlation, and any future
canonical digest built from the acquisition result.

The selector already makes the economic decision independent of input order,
but order-independent selection is not sufficient for a replayable boundary.
The acquisition result must have one canonical representation before it is
passed to policy admission or recorded as bounded operational evidence.

## Decision

Normalize both successful candidates and failure classifications after all
bounded provider tasks complete. Sort each collection by the immutable
provider priority declared by `FxProviderPolicy::allowed_providers`.

The normalization is performed in the coordinator, after aggregate timeout
handling and before returning to the caller. It performs no network, clock,
signing, or unbounded work. Provider identities are validated against the
policy before any task is spawned, so an unknown identity cannot enter the
normalized result.

This ordering is observational and canonical; it does not change the policy's
minimum-agreement or exact-economic-terms rules. The selector remains the sole
authority for issuance eligibility.

## Invariants and failure policy

- Equivalent provider responses produce equivalent result sequences regardless
  of response completion order.
- Candidate and failure collections contain at most the validated source
  count, which is capped by ADR-0252.
- Policy priority is the only ordering input; response timing never affects
  evidence order or selection.
- No provider or signer call occurs during normalization.
- A failed or timed-out provider remains a classified failure and cannot count
  toward agreement.

## Alternatives considered

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Preserve task completion order | No post-processing | Nondeterministic replay and telemetry order | Rejected |
| Sort by provider identity lexicographically | Easy and stable | Ignores governed policy priority and may conflict with selection order | Rejected |
| Sort at every downstream consumer | Local flexibility | Duplicated security-sensitive logic and inconsistent evidence | Rejected |
| **Normalize once by policy priority in the acquisition coordinator** | Stable, bounded, aligned with admission policy, and easy to test | Requires the policy at the return boundary | **Selected** |

## Verification

The FX-provider test suite proves that deliberately reversed response delays
still return candidates and failures in policy order. Existing concurrency,
timeout, cancellation, identity, agreement, and signing tests remain required.
The production provider adapter and regional SLO evidence remain separate
release gates.

