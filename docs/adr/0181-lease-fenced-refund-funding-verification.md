# ADR-0181: Lease-fenced refund-funding topology verification

- Status: Accepted — implementation complete; release evidence pending
- Date: 2026-09-29
- Owners: Settlement infrastructure and finance/tax SRE
- Scope: Refund Movement bounded context, route-verifier, PostgreSQL

## Context

ADR-0174 makes finalized Solana account topology the only authority that can
activate a reviewed refund-funding configuration. The verifier is a bounded
worker and may run as more than one replica for availability. The previous
`approved` to `pending_verification` transition prevented a configuration
from being activated without proof, but it did not assign ownership of the
provider work. Two replicas could therefore read the same pending row, issue
duplicate finalized RPC requests, and race to persist equivalent evidence.

Duplicate reads are safe for correctness only when the activation write is
fenced. They are not acceptable as the normal concurrency contract: they add
RPC load, make provider disagreement harder to diagnose, and allow a slow
stale worker to compete with a newer retry.

## Decision

Use a short PostgreSQL lease on each refund-funding configuration while the
verifier performs external RPC work:

1. A verifier transaction creates a UUIDv7 `verification_lease_token` and
   `verification_lease_claimed_at` timestamp. It may claim an `approved` row
   while moving it to `pending_verification`, or reclaim a pending row only
   after the bounded 60-second lease TTL.
2. The transaction commits before any RPC call. No database row lock is held
   across network I/O.
3. A verifier that sees an unexpired lease skips the row. The token is never
   supplied by a browser or operator.
4. Activation requires the same token, configuration identity, version,
   configuration digest, and `pending_verification` status. The successful
   activation clears the lease and writes immutable finalized evidence and the
   outbox record in one transaction.
5. Provider failure clears only the matching token, leaving the configuration
   pending for the next bounded retry. A stale worker cannot clear or activate
   a lease that has been reclaimed by another worker.
6. The lease is coordination state only. It cannot authorize token movement,
   signing, settlement, or legal/tax compliance claims.

The lease columns are nullable and pair-constrained. UUIDv7 gives sortable,
auditable worker-claim identities; it is not used as a secret or authority.

## Alternatives considered

| Alternative | Benefit | Cost/risk | Decision |
| --- | --- | --- | --- |
| PostgreSQL UUIDv7 lease with token-fenced CAS | Uses the authoritative durable store, survives replica loss, bounded and observable | Requires migration and stale-lease policy | **Selected** |
| Hold `FOR UPDATE` across RPC | Simple ownership semantics | Blocks writers for network latency and can exhaust the pool; violates hot-path lock discipline | Rejected |
| PostgreSQL advisory lock | No schema change | Ownership is less inspectable, recovery semantics are session-bound, and evidence cannot carry the claim identity | Rejected |
| Redis distributed lock | Low-latency shared lock | Redis is non-authoritative and introduces lease-clock/availability ambiguity into activation | Rejected |
| Queue partitioning | Serializes work per key | Adds broker dependency and does not itself fence a stale completion | Rejected as the authority |

## Invariants and failure policy

- At most one unexpired token can own a configuration at a time.
- A verification proof can activate only the exact configuration and lease that
  produced it.
- A timeout, provider disagreement, malformed account response, or process
  crash never activates a configuration.
- A crash leaves a recoverable pending row; the TTL bounds recovery delay.
- Lease release is compare-and-set and cannot erase a newer owner's token.
- Activation evidence remains immutable after activation, suspension, or
  retirement.

## Required evidence

- Failing-first storage tests for claim, unexpired skip, stale reclaim,
  token-mismatched release, and token-mismatched activation.
- Concurrent Docker verification with two verifier workers proving one RPC
  owner and one fenced skip.
- Provider failure and process-crash retry evidence.
- Migration verification for UUIDv7 lease columns, pair constraint, and
  partial eligibility index.
- Metrics and alert/runbook coverage for verification backlog, lease expiry,
  provider disagreement, and activation failures.
- Linux validator evidence remains a separate ADR-0174 release gate.

The reproducible local evidence is `make refund-funding-verifier-drill`. It
starts two isolated route-verifier containers against PostgreSQL and a
synthetic RPC fixture that returns the complete five-account legacy-SPL
topology. The drill asserts that exactly one verifier owns the lease, exactly
one finalized account read occurs, exactly one activation event is committed,
and the other verifier observes a fenced skip. It then ages a claimed lease,
restarts a verifier, and asserts stale reclaim plus single activation. The
fixture is synthetic and local-only; it is not an authority and cannot be
used by staging or production configuration.

The same command is retained as a public-safe artifact by the Ubuntu CI
`refund-funding-verifier` job. `REFUND_FUNDING_VERIFIER_EVIDENCE_DIR` writes
only exit status and fixed-cardinality counts to `result.env`; it contains no
RPC credentials, private keys, customer data, or tax evidence. CI retention
improves regression detection but does not replace production RPC,
multi-region, or Linux validator evidence required by ADR-0174.
