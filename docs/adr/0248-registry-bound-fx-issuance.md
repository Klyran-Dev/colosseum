# ADR-0248: Registry-bound FX evidence issuance

## Status

Accepted for implementation.

## Context

ADR-0247 introduced an immutable digest-keyed policy snapshot, but a caller
could still construct `FxEvidenceIssuer` directly with a policy that was not
present in the active registry. That creates an authority bypass: the policy
digest would appear in signed evidence even though it was not an approved
deployment snapshot.

## Decision

Expose a registry-bound issuance method that resolves the exact requested
policy digest from the active snapshot and constructs the issuer only after a
successful lookup. The method returns a bounded `PolicyNotFound` error for a
missing or zero digest. Direct construction remains available to the pure
crate for isolated tests and explicitly controlled adapters, but production
composition must use the registry-bound method.

The resolved policy is cloned into the issuer as an immutable request-scoped
value. No lock, filesystem, database, broker, or provider call occurs after
the registry lookup. A policy replacement affects only subsequent issuance;
an in-flight issuer retains the exact policy that was selected for its
request, preserving evidence replay and avoiding partial reads.

## Alternatives considered

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Keep the public constructor as the production path | No API change | Any adapter can bypass the active policy registry | Rejected |
| Pass a boolean or caller assertion that a policy is approved | Small signature | Unverifiable authority and easy misuse | Rejected |
| Resolve policy from PostgreSQL or remote configuration per request | Centralized | Hot-path latency, mutable replay, and new failure dependencies | Rejected |
| **Resolve exact digest through the immutable registry before constructing the issuer** | Fail-closed authority, lock-free lookup, deterministic request snapshot | Requires production adapters to select the registry API | **Selected** |

## Invariants and failure policy

1. A non-zero policy digest must resolve in the active registry before
   production issuance.
2. Missing, zero, or invalid policy digests fail before provider or signer
   work.
3. The issuer owns an immutable clone of the selected policy for the full
   request and cannot observe a later snapshot replacement.
4. Policy replacement never mutates or invalidates an issuer already in use.
5. The registry remains a policy read model; publication, legal approval,
   signer custody, and activation evidence remain governed outside the crate.

## Verification

- Failing-first tests prove exact digest selection, missing/zero digest
  rejection, and request isolation across snapshot replacement.
- Existing policy validation, signing, freshness, deviation, and replay tests
  remain required.
- Runtime composition must use the registry-bound method; direct construction
  is reserved for test fixtures and explicitly reviewed adapters.
