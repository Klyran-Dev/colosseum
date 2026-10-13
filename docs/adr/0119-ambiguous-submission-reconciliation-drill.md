# ADR-0119: Ambiguous submission and signature reconciliation drill

- Status: Accepted
- Date: 2026-09-27
- Deciders: Principal Low-Latency Architecture and SRE
- Scope: local RPC fixture, settlement integration evidence, unknown-intent recovery

## Context

ADR-0118 prevents a second submission after a provider outcome is ambiguous, but
the existing local drill only proves the successful concurrent path. A Tax Engine
release needs executable evidence for the more dangerous case: the provider
accepts and records a transaction, the API receives an ambiguous error, and a
later status query finds the exact native signature.

The evidence must not depend on sleeping past a timeout or on manually editing
PostgreSQL rows. Those approaches are timing-sensitive and do not prove the
provider accepted the exact signed bytes.

## Decision

Extend the local-only RPC fixture with a bounded, explicit admin control that
switches the next `sendTransaction` outcome to:

1. validate and record the signed transaction and its native signature;
2. increment the send counter;
3. return a JSON-RPC error representing a lost provider response; and
4. expose the recorded signature as `confirmed` from `getSignatureStatuses`.

The integration drill will then prove, in order:

1. the API returns `SETTLEMENT_SUBMISSION_UNKNOWN` and never reports success;
2. the durable intent is `unknown` and retains the exact native signature;
3. reconciliation returns `confirmed` for that signature;
4. the RPC send counter increases by exactly one, not two; and
5. the reconciliation request does not carry or recreate signed transaction
   bytes.

The control is available only on the synthetic fixture bound to the local
Compose profile. It is not an API production capability, is not reachable by
the settlement service as a normal business route, and cannot alter the
production coordinator's retry or lifecycle policy.

## Alternatives considered

### Sleep beyond the coordinator timeout

Rejected. It is nondeterministic at scheduler and network boundaries, makes the
test slow, and does not prove the provider accepted the transaction before the
response was lost.

### Mutate PostgreSQL directly to `unknown`

Rejected. It bypasses the API's failure path, append-only lifecycle event, and
signature-preservation behavior.

### Add a production-facing fault-injection endpoint

Rejected. It expands the production attack surface and confuses operational
failure injection with an intentionally constrained local test adapter.

### Local fixture with explicit accepted-but-ambiguous mode

Selected. It is deterministic, bounded, validates the exact signed bytes before
recording acceptance, and exercises the real API/coordinator/storage boundary.

## Invariants and release limits

- The fixture records at most one bounded last accepted signature.
- An ambiguous submission always increments the send counter exactly once.
- The API persists `unknown` with the signature and a bounded error code.
- Reconciliation may advance from `unknown` only from provider status evidence.
- Reconciliation never submits a transaction and never needs signed bytes.
- The drill is local evidence only; validator, crash-process, multi-replica,
  provider-failover, and production fault-injection evidence remain separate
  gates.

## Verification

The drill is executed by `scripts/verify_local_stack.sh` against the pinned
Docker images and must assert the HTTP responses, durable lifecycle, signature
identity, and exact RPC send count. The test is not a production-readiness
claim until the corresponding live-provider and process-crash scenarios pass.
