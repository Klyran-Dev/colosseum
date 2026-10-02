# ADR-0247: Atomic FX provider policy snapshot reload

## Status

Accepted for implementation.

## Context

ADR-0246 makes FX provider policy explicit, but a production process must also
load and rotate approved policy versions without a request-path database read,
remote configuration dependency, partial replacement, or process restart. A
policy digest is already part of the issued evidence digest, so lookup must be
by an immutable digest rather than by mutable names or caller-selected fields.

## Decision

Add a bounded `FxProviderPolicyRegistry` to the `fx-provider` context. It loads
a versioned JSON document containing at most 32 fully validated policies, keys
them by their non-zero policy digest, and replaces the immutable snapshot in a
single atomic operation. Parsing and validation happen before the replacement;
an invalid, empty, oversized, duplicate, or semantically invalid document
leaves the last known-good snapshot active.

The registry is a warm in-process read model. It is not an authority for
policy publication, legal approval, or HSM/KMS custody. A governed deployment
adapter owns file delivery, permissions, provenance, activation approval, and
reload signaling. The issuance path receives an already selected immutable
policy and never calls PostgreSQL, a broker, a filesystem, or a remote config
service.

## Alternatives considered

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Read PostgreSQL for every issuance | Centralized storage | Hot-path latency, contention, partial failure, and mutable replay | Rejected |
| Query remote configuration at request time | Centralized rollout | External dependency and nondeterministic tail latency | Rejected |
| Process restart for every policy change | Simple lifecycle | Slow rotation and larger blast radius | Rejected |
| Mutable global policy object | Minimal code | Readers can observe partial updates and stale digest bindings | Rejected |
| **Bounded immutable digest-keyed snapshot with atomic replacement** | Lock-free reads, deterministic replay, fast rotation, fail-closed reload | Requires governed delivery and retained activation evidence | **Selected** |

## Invariants and failure policy

1. Every policy is validated before it enters a snapshot.
2. Policy lookup is by the exact requested digest; names and array position
   are not authority boundaries.
3. Duplicate digests, duplicate currency pairs, unknown fields, invalid bounds,
   and malformed JSON reject the entire replacement.
4. Empty or invalid replacement documents never erase the last good snapshot.
5. Snapshot size, document size, pair count, text fields, and all policy time
   and deviation bounds are finite and explicit.
6. A missing digest fails closed before provider issuance.

## Verification

- Failing-first tests cover valid replacement, digest lookup, duplicate and
  malformed entries, invalid policy bounds, oversized documents, and retention
  of the last good snapshot after failed reload.
- Existing exact policy, signing, and tax-engine replay tests remain required.
- Deployment-level file provenance, permissions, legal approval, rotation
  timing, and production reload evidence remain release gates.
