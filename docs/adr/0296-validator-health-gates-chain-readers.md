# ADR-0296: Gate the indexer on validator health

## Status

Accepted

## Context

The local `indexer` and `route-verifier` use `http://validator:8899` by
default. Compose previously started them after PostgreSQL and migrations only,
so a validator that failed during startup left both services alive and
retrying an unavailable RPC endpoint indefinitely. This produced replay-gap
noise and made a chain-reader failure look like an indexer failure.

The pinned Agave 4.3.0 validator requires an io_uring-capable Linux runtime.
Some Docker Desktop environments cannot provide that capability, so the
validator exits before its RPC health endpoint is available. That condition
must be visible as a startup failure, not converted into a long-lived degraded
indexer process.

## Decision

The indexer performs a bounded RPC readiness probe before it binds its ready
state. The probe uses the configured provider pool and accepts a valid
`NonFinalized` response as proof that the endpoint is reachable but the
configured start slot is ahead of the current chain. It retries for a fixed
startup budget and exits if no provider becomes reachable. This keeps Compose
profiles independent: synthetic RPC-fixture drills can continue to override
`INDEXER_RPC_ENDPOINTS` and run with `--no-deps`.

PostgreSQL and migration dependencies remain required as before. The route
verifier keeps its existing bounded retry behavior because its verification
passes already leave routes inactive when the provider is unavailable.

We do not add an automatic fallback to a fixture, downgrade Agave, or disable
validator restart diagnostics. The validator version and Linux io_uring
capability are release evidence requirements; silently changing either would
invalidate conformance evidence.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| Retry RPC from a healthy-looking indexer | Rejected | Keeps a broken chain reader in service and floods replay-gap state. |
| Automatically use the RPC fixture | Rejected | Synthetic data must never masquerade as chain truth. |
| Downgrade the local validator | Rejected | Breaks the pinned toolchain and changes conformance semantics. |
| Gate the indexer on validator health | Selected | Fails fast, preserves authority boundaries, and gives the service a precise startup gate. |

## Consequences

- A local validator startup failure prevents the indexer from becoming ready
  and makes the validator error the primary diagnostic after a bounded probe
  window.
- Docker Desktop users whose Linux VM lacks the required io_uring behavior must
  run the pinned validator on a supported Linux/amd64 runtime, or use the
  repository's explicit synthetic RPC-fixture drills for non-validator work.
- Validator-backed conformance remains a separate release gate and is not
  replaced by a Docker image build or a fixture test.

## Verification

- `docker compose --profile indexer --profile solana config --quiet`
- `make integration-check`
- `make solana-conformance` on a Linux/amd64 io_uring-capable runner
