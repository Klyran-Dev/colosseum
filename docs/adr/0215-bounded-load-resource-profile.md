# ADR-0215: Bounded load-test resource-profile evidence

- Status: Accepted
- Date: 2026-09-30
- Deciders: Principal Low-Latency Architecture, API / Tax Engine, SRE
- Bounded contexts: Performance evidence, Docker local development, CI release evidence

## Context

The bounded aggregate and document quote harnesses already report workload
shape, status counts, p50/p95/p99 latency, pool waiting, and durable
idempotency/calculation/outbox cardinality. Latency without the contemporaneous
resource profile is insufficient to distinguish an implementation regression
from a different container or host budget, and it cannot support a credible
low-latency review.

The profile must remain bounded, public-safe, and reproducible. It must not
collect request bodies, credentials, tenant identifiers beyond the existing
synthetic benchmark metadata, or arbitrary host process data.

## Decision

Extend `scripts/benchmark_quote_service.sh` with an opt-in Docker resource
sampler that runs only during the measured unique-request and same-key
contention phases:

- sample the benchmark API replica, independent API replica, PostgreSQL, and
  NATS containers once per second with Docker's JSON stats format;
- retain timestamped per-container records as a CI artifact when
  `BENCHMARK_RESOURCE_PROFILE_OUTPUT` is set, while always reporting the
  sampling mode, interval, container set, record count, and sampling-epoch
  count in stdout;
- stop and reap the sampler before teardown, including on failure, so it cannot
  outlive the isolated Compose project;
- fail the benchmark if resource profiling is explicitly enabled but no valid
  container record is captured; and
- keep the profile diagnostic only. It does not establish a production SLO,
  regional capacity limit, or a target-cluster/RPC budget.

The CI aggregate and document jobs set the output path under their existing
retained evidence directories. Local runs may enable the same path explicitly;
the default remains temporary cleanup after the run.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| Report latency without resource data | Rejected | Cannot explain host/container budget changes or support low-latency review. |
| Sample arbitrary host processes | Rejected | Leaks unrelated host data and is not reproducible across runners. |
| Add an unpinned monitoring stack to every benchmark | Rejected | Adds startup cost and dependency drift to a bounded service test. |
| **Bounded Docker JSON stats for the four benchmark containers** | **Selected** | Uses the existing isolated runtime, has fixed scope, is public-safe, and is easy to retain with latency evidence. |

## Invariants and budgets

- Sampling is bounded to the benchmark lifetime and one sample per second.
- Only explicitly named benchmark containers are sampled.
- The sampler is cancelled and reaped on success, failure, interrupt, and
  teardown.
- Raw records are timestamped JSON lines containing Docker runtime metrics
  only; no application payloads or secrets are written.
- A successful CI evidence artifact includes both the benchmark summary and
  the resource-profile file.

## Verification and release impact

The benchmark shell contract, `sh -n`, CI configuration, and existing Docker
load jobs verify the wiring. A retained profile improves diagnostic evidence,
but production acceptance still requires regional/resource-profile runs,
representative data volume, an approved error budget, and live-provider or
cluster measurements.

## Local evidence (2026-09-30)

The aggregate workload passed with profile retention enabled: 200/200
successful requests at concurrency 16 across two API replicas, 32/32
same-key contention responses, exact 200/200/200 idempotency/calculation/
outbox counts, and p50/p95/p99 of 4.634/17.965/37.060 ms. The run retained
four timestamped per-container Docker JSON-stat records covering the API,
independent API replica, PostgreSQL, and NATS in one sampling epoch. A
subsequent sampler-correctness rerun preserved the same invariants with
p50/p95/p99 of 4.078/26.630/31.121 ms and the same four records/one epoch.
This is local synthetic diagnostic evidence;
it is not regional capacity, a production resource budget, or SLO acceptance.

A follow-up document workload used the corrected profile contract and passed
200/200 requests, 32/32 contention responses, exact durable counts, and
p50/p95/p99 of 4.166/26.137/35.064 ms. It also retained four timestamped
per-container records in one sampling epoch. Both runs used the pinned local
Compose images and a synthetic tenant.
