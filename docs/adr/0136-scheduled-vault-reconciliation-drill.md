# ADR-0136: Prove scheduled vault reconciliation through the RPC boundary

- Status: Accepted
- Date: 2026-09-27
- Owners: Chain Data, Finance Operations, SRE

## Context

The indexer service already contains a bounded, sequential vault-reconciliation
cycle and fixed-cardinality mismatch/error gauges. Existing integration checks
validated the tenant-scoped read model using a row inserted directly into
PostgreSQL. That proves storage and authorization, but it does not prove that
the scheduler reads a finalized token account, validates its owner/mint/layout,
computes the checked-u64 expectation, appends a run, or updates alert inputs.

A production-grade system needs an executable local contract for that boundary.
The local fixture must remain synthetic and must never be treated as a Solana
ledger or production RPC. Live validator, multi-provider, and staging evidence
remain separate gates.

## Decision

Extend the synthetic RPC fixture with a bounded, opt-in `getMultipleAccounts`
response for one configured legacy SPL token account. The fixture constructs the
canonical 165-byte account layout, binds the configured mint and amount, emits a
configurable finalized slot, and exposes a fixture-only amount mutation endpoint
for deterministic mismatch evidence. The fixture rejects unconfigured or
malformed requests and is not included in production images or APIs.

Add `scripts/vault_reconciliation_drill.sh`, which starts isolated PostgreSQL,
the RPC fixture, and the real indexer service; inserts one synthetic
server-owned target; waits for the scheduled cycle; and proves:

1. the RPC account is read at finalized commitment;
2. the first run is `matched` with exact expected/observed atomic values;
3. changing the fixture amount creates an append-only `mismatch` run;
4. the fixed-cardinality Prometheus mismatch gauge becomes `1` while the
   error gauge remains `0`; and
5. a PostgreSQL query shows at least two immutable run rows for the same target.

The drill has bounded startup, polling, target cardinality, and cleanup. It
does not mutate production-like chain state, advance a checkpoint, or allow a
browser/operator to set opening balances.

## Alternatives considered

| Alternative | Why rejected |
| --- | --- |
| Keep a database-only seeded run | Does not exercise the real scheduled service or RPC account validation. |
| Use a mock `FinalizedVaultBalanceSource` only | Fast unit coverage, but misses Docker process configuration, JSON-RPC serialization, and PostgreSQL persistence. |
| Start a full validator for every reconciliation test | Stronger chain evidence but too slow and unavailable on runtimes without Agave's required `io_uring`; retained for the separate validator gate. |
| Call a public/testnet RPC | Non-deterministic, external data, privacy risk, and unsuitable for repeatable CI. |
| Add an unbounded generic fixture account API | Increases attack surface and makes fixture responses less auditable; one bounded configured account is sufficient. |
| **Synthetic bounded RPC account plus real scheduled indexer drill** | Exercises the service boundary deterministically without weakening the live-validator requirement. |

## Consequences

- Local and CI evidence now distinguishes storage/read-model tests from actual
  scheduled RPC reconciliation.
- The fixture gains test-only behavior and must remain isolated by Compose
  profile, synthetic values, and no production image dependency.
- The mismatch drill proves alert input, not Alertmanager delivery or human
  response; those remain deployment evidence.
- Token-2022 extension semantics continue to fail closed and are not widened by
  this decision.
