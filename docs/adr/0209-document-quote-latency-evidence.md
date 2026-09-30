# ADR-0209: Bounded document-quote latency evidence

- Status: Accepted
- Date: 2026-09-30
- Owners: API / Tax Engine / SRE
- Scope: Authenticated document-quote HTTP latency and bounded concurrency

## Context

The aggregate quote benchmark does not represent the Tier-1 document path.
Document quotes carry a versioned multi-line facts envelope, line-scope tax
calculation, jurisdiction evidence, a separate PostgreSQL calculation table,
and a document-specific outbox event. Treating aggregate quote measurements as
document performance evidence would hide payload, serialization, rule-graph,
and persistence differences.

The document path therefore needs a repeatable workload with explicit line
cardinality, concurrency, timeout, tenant, published document rule, provider
evidence, two API replicas, same-key contention, and durable record-count
invariants. It must remain local synthetic evidence and must not be presented
as a production SLO.

## Decision

Extend `scripts/benchmark_quote_service.sh` with an explicit
`BENCHMARK_MODE=document` mode and expose it as
`make benchmark-document-quote-service`. The mode reuses the bounded Docker
topology, ephemeral EdDSA credential, request timing, percentile calculation,
and contention harness while changing only the authoritative document contract:

1. use the published synthetic document rule and a bounded two-line document;
2. call `POST /v1/tax/document-quote` on alternating API replicas;
3. bind provider-signed jurisdiction evidence to the exact document facts;
4. verify one `tax_document_calculations` row and one
   `tax.document_calculation.created` outbox event per unique request; and
5. verify one idempotency record, calculation, and outbox event for the
   cross-replica same-key contention phase.

The existing aggregate mode remains unchanged and is still the default. CI
runs the document mode as a separate bounded job and retains its evidence
artifact. The benchmark is not a settlement or chain-finality benchmark; live
RPC, validator, regional, resource-profile, and production SLO evidence remain
separate gates.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| Reuse aggregate quote results as document evidence | Rejected | Different facts shape, rule scope, storage table, and outbox aggregate make the measurement non-representative. |
| Copy the aggregate benchmark into a second script | Rejected | Duplicates concurrency, timeout, credential, percentile, and cleanup logic and risks drift in safety invariants. |
| Add a mixed aggregate/document workload | Rejected as the primary gate | Useful for capacity planning later, but obscures per-context regressions and makes persistence attribution ambiguous. |
| **Explicit mode in the existing bounded Docker harness** | **Selected** | Preserves DRY/reproducibility while isolating document workload, endpoint, evidence, and durable invariants. |
| External load tool or host-installed benchmark binary | Rejected | Violates the clean-checkout Docker contract and adds toolchain/version drift. |

## Budgets and invariants

- Default workload: 200 unique document quotes, concurrency 16, 32 same-key
  contention requests, two API replicas, and a bounded 10-second request
  timeout.
- Each document has two lines and uses only synthetic provider evidence and a
  synthetic published rule.
- Any non-200 response, incomplete sample, invalid latency, duplicate durable
  calculation, missing outbox event, or contention divergence fails the run.
- Percentiles are emitted in milliseconds with request count, concurrency,
  pool size, status counts, and the exact endpoint/rule contract.
- The benchmark never receives a user private key, signs a Solana transaction,
  or uses a non-loopback endpoint by default.

## Consequences

Document quote regressions become independently visible in local and CI
evidence without weakening the aggregate benchmark or claiming production
performance. The workload consumes additional CI time and PostgreSQL storage
but cleans up its isolated volumes on exit. Production acceptance still
requires regional load, representative data volume, resource limits, error
budget, and live chain/provider measurements.

## Local evidence (2026-09-30)

`make benchmark-document-quote-service` passed on the shared Docker Desktop
host with the declared workload: 200/200 successful requests at concurrency
16 across two API replicas, 32/32 successful same-key contention responses,
and no observed database-pool waiters. The measured p50/p95/p99 were
5.467/34.901/45.221 ms. Durable invariants were exact: 200 idempotency
records, 200 document calculations, 200 outbox events, and one of each for
the contention key. This is local synthetic evidence only; it is not a CI
retained result, regional measurement, or production SLO acceptance.
