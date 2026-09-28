# SLO and error-budget policy

Status: operational contract baseline, not evidence of production attainment.
Last reviewed: 2026-09-27.

This policy applies to the quote, tax, settlement, indexer, reconciliation,
governance, and operator journeys. It is deliberately provider-neutral. A
deployment must bind the roles and endpoints in the dependency register before
the SLOs are considered active.

## Measurement rules

- Use a rolling 30-day window for release decisions and a rolling 7-day window
  for incident response.
- Measure by environment, region, cluster, and approved RPC provider. Keep
  those dimensions bounded; never add tenant IDs, wallet addresses, raw
  signatures, rule digests, or arbitrary URLs as metric labels.
- Exclude only scheduled maintenance that was approved before the window and
  explicitly communicated. Do not exclude provider outages, deploys, queue
  saturation, database failures, or operator mistakes.
- A request is successful only when it returns a semantically valid response
  within its deadline. HTTP 2xx with malformed, stale, unauthorized, or
  unbound financial evidence is a failure.
- A chain transaction is successful only when the expected finalized receipt,
  balances, and durable evidence agree. Wallet signing or an RPC submission
  acknowledgement is not finality.
- Report p50, p95, and p99 latency with sample count, workload, commitment,
  transaction format, resource limits, and provider. A target without those
  fields is not a benchmark.

## User-journey and financial-control objectives

| Objective | SLI | Initial target | Error-budget interpretation |
| --- | --- | --- | --- |
| Quote availability | Valid, authenticated quote responses before deadline / eligible requests | 99.95% monthly | 21m 54s monthly availability budget |
| Warm quote latency | Valid quote latency under the declared warm-cache workload | p50 ≤25 ms, p95 ≤60 ms, p99 ≤100 ms | Latency failures consume quote budget |
| Compilation latency | Valid server-owned compiled artifact under bounded account shape | p50 ≤10 ms, p95 ≤25 ms, p99 ≤40 ms | Never bypass artifact validation to recover budget |
| Submit acknowledgement | Durable intent transition to an accepted, rejected, or explicit unknown state | p50 ≤150 ms, p95 ≤300 ms, p99 ≤500 ms | Unknown is a safe outcome, not a success |
| Finalized evidence freshness | Eligible accepted settlement with finalized receipt and projection | 99% within 15 s, excluding declared chain outage | Delay pages reconciliation; it never authorizes release |
| Tax replay integrity | Replays matching stored facts, rule digest, engine version, trace, and attestation | 100% | Any mismatch is a P0/P1 financial-control incident |
| Amount conservation | Finalized source debit equals tax plus net distributions | 100% | Zero tolerance; stop the affected route and preserve evidence |
| Unauthorized outflow | Finalized vault outflow without a valid governed claim | 0 events | Zero tolerance; pause release authority and page immediately |
| Indexer projection correctness | Rebuild digest equal to retained raw finalized evidence | 100% of drill runs | A mismatch blocks release and reporting use |
| API recovery | API service restored after an approved failure | RTO ≤30 min | Requires retained drill evidence |
| Durable evidence recovery | PostgreSQL evidence loss bounded by backup/PITR policy | RPO ≤5 min, RTO ≤60 min | Deployment-specific backup evidence is required |

The quote and compilation targets are engineering budgets for a declared
workload, not a promise about every geography or RPC. Chain-confirmation and
finalization objectives are reported separately from API latency.

## Error-budget policy

1. At less than 50% remaining budget, review the top contributors and freeze
   unmeasured performance claims.
2. At less than 25% remaining budget, require an owner-approved reliability
   plan for new feature work and prioritize capacity, backpressure, and
   correctness fixes.
3. At zero budget, pause unrelated releases. Only security, correctness,
   incident recovery, and work that demonstrably restores the budget may ship.
4. A financial-integrity violation, evidence mismatch, unauthorized outflow,
   or unsafe authority change overrides availability-budget status and is an
   immediate release stop.
5. Resume normal release flow only after the incident has a contained state,
   preserved evidence, an owner, a corrective action, and a verified recovery
   window.

Alert burn-rate windows should be 5m/1h and 1h/6h for fast and slow burns.
Alert rules must use the fixed-cardinality metrics in
[`operations.md`](../architecture/operations.md); receiver delivery and
on-call acknowledgement require separate deployment evidence.

## Ownership and review

| Concern | Accountable role | Required backup |
| --- | --- | --- |
| Quote/tax service | Tax Platform owner | Settlement Platform owner |
| On-chain program and signer boundary | Settlement Security owner | Security incident commander |
| Indexing/reconciliation | Chain Data owner | Finance Operations owner |
| PostgreSQL evidence and recovery | Data Platform owner | SRE on-call |
| UI/accessibility and wallet handoff | Product UI owner | Support lead |
| Incident command and communications | SRE incident commander | Engineering duty manager |
| Legal/tax claims | Tax/Legal owner | Product owner |

The generic roles above must be replaced with named accountable teams and
escalation routes in each deployment record. No code or runbook may invent a
pager, phone number, signer, or legal owner.

## Required retained evidence

Each monthly report and release candidate must retain the query or dashboard
revision, time window, environment, region, provider, sample count, percentile
results, budget calculation, alert delivery result, and linked incident/change
IDs. Synthetic Docker measurements are labeled local evidence and cannot close
regional production gates.
