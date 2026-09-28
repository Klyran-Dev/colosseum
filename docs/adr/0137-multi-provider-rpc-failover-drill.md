# ADR-0137: Prove bounded multi-provider RPC failover at the service boundary

- Status: Accepted
- Date: 2026-09-27
- Owners: Settlement Platform, Chain Data, SRE

## Context

The settlement coordinator and finalized indexer use bounded adaptive provider
health, deadlines, and failover. Unit tests prove ordering and retry
invariants, but a Tier-1 release needs process-level evidence that the
configured endpoint list is actually wired into a deployed service. The drill
must demonstrate a degraded provider attempt and a successful bounded fallback
without claiming that a synthetic fixture represents a real RPC vendor or
regional network.

## Decision

Extend the isolated RPC fixture with:

- a bounded `RPC_FIXTURE_FAIL_METHODS` configuration that returns a deterministic
  JSON-RPC error for selected methods;
- a fixture-only count for finalized vault-account reads; and
- a fixture-only admin read endpoint for that count.

Extend `scripts/vault_reconciliation_drill.sh` with an opt-in
`VAULT_RECONCILIATION_PROVIDER_MODE=failover` mode. It starts a failing fixture
and a healthy fixture on the same isolated Compose network, configures the real
indexer with the failing endpoint first and healthy endpoint second, and proves:

1. the real scheduler produces a matched finalized vault reconciliation;
2. both providers receive a bounded account-read attempt;
3. the final observed result comes from the healthy provider; and
4. the existing mismatch/append-only assertions remain intact.

The fixture failure is method-scoped and local-only. The drill does not modify
signed transaction bytes, create a quorum, or assert production provider SLOs.

## Alternatives considered

| Alternative | Why rejected |
| --- | --- |
| Unit tests only | Cannot prove endpoint configuration, Docker networking, or the deployed service's transport boundary. |
| Retry one provider indefinitely | Violates bounded retry, latency, and overload mandates. |
| Randomly select providers per request | Makes evidence nondeterministic and weakens adaptive health semantics. |
| Public/testnet RPC outage injection | Not repeatable, not controlled, and may expose sensitive traffic. |
| **Two isolated fixture providers with deterministic first-provider failure** | Reproducible process-level evidence while preserving a separate live-provider gate. |

## Consequences

- CI can detect provider-list wiring and failover regressions without external
  dependencies.
- The fixture gains test-only failure behavior and must remain profile-isolated,
  bounded, synthetic, and excluded from production images and APIs.
- Live provider authentication, regional capacity, failover SLOs, and
  provider-contract evidence remain open release gates.
