# Operations, observability, and SRE

The normative operating policy is [the SLO and error-budget
policy](../operations/slo-error-budget-policy.md), with incident response in
the [severity runbook](../runbooks/incident-severity-and-error-budget.md) and
provider ownership/exit paths in the [dependency
register](../operations/dependency-register.md). This document defines the
telemetry and evidence shape; it does not claim that a deployment has met the
targets.

## Source-of-truth model

The Solana ledger, verified program logs, token balances, and settlement receipt accounts are authoritative. PostgreSQL contains rebuildable projections. Redis/cache entries are disposable. No business decision may depend on an unverified WebSocket notification.

## Telemetry

Emit OpenTelemetry traces and metrics with the API-owned UUIDv7 `x-request-id`
correlation ID carried through quote, compile, wallet handoff, submit,
signature lookup, index, and reconciliation. The API overwrites incoming
values and returns its generated value on every response; this identifier is
diagnostic metadata, never an authorization or idempotency credential.

The first executable metric boundary is the Rust `observability` crate and the
API `/metrics` endpoint. It uses fixed operation names and bounded latency
histogram buckets; unknown request-derived labels are rejected. This is a
foundation contract. The Docker `observability` profile adds Prometheus,
Grafana, and an OpenTelemetry Collector; local dashboards and collection are
operator aids, not evidence that production scrape authorization, alert
routing, retention, or SLO evidence are complete.

Prometheus alert rules are versioned in
`ops/observability/alert-rules.yml`. The pinned-image `promtool` contract
validates both the Prometheus configuration and rule syntax in CI and through
`make observability-config`. Rules cover exporter availability, bounded
aggregate error ratio, quote/compile p95 budgets, and compiled-rule-cache
pressure. Production must bind these validated rules to an approved
Alertmanager/on-call receiver; local rule validation and dashboards never
prove notification delivery.

Required metrics:

- `settlement_quote_latency_ms` and `settlement_compile_latency_ms`;
- `settlement_submit_latency_ms` by RPC provider and region;
- confirmation latency by commitment level;
- transaction success/failure by program error code;
- compute units requested/consumed and priority fee;
- stale policy and policy mismatch rejects;
- duplicate nonce rejects;
- indexer slot lag, event gap count, replay duration;
- vault balance delta versus receipt-derived balance;
- API rate-limit rejects and wallet-signature abandonment.
- compiled tax-rule cache hits, misses, evictions, entries, and compile
  duration by service replica.
- route-verifier account-proof latency, provider attempt result, pending route
  age, activation count, verification failure count, and finalized slot lag;
  provider and route identifiers must use bounded allow-lists or remain in
  structured logs rather than creating unbounded metric labels.
- refund-proof worker cycle count, duration, and error count through the fixed
  `refund_proof` operation; `stp_indexer_refund_proofs_pending` is a durable
  configured-stream backlog gauge. Claim, signature, merchant, address, and
  provider identities remain structured evidence/log fields, never metric
  labels. ADR-0159 defines the alert and reconciliation boundary.

Replay-specific telemetry is mandatory: claimed gap count, chunk duration,
remaining suffix slots, lease-expiry recovery count, dead-letter count, and
claim-token loss. The replay executor processes one bounded chunk per indexer
tick and never advances the forward checkpoint; alerting must distinguish
historical projection repair from forward chain ingestion.

ADR-0123 now exposes the first bounded replay telemetry slice from the
indexer's configured stream: queued, leased, and dead-lettered gap gauges plus
the leased slot span. The stream key is deliberately not a metric label. The
versioned Prometheus rules alert on a sustained queued backlog and any
dead-lettered work. Lease-expiry recovery and claim-token-loss counters,
scrape authorization, receiver delivery, and on-call response remain open
production gates.

ADR-0124 adds a bounded finalized signature-pagination adapter for targeted
recovery. It is a discovery signal only: it validates canonical signatures,
finalized status, page limits, duplicate entries, and cursor progress, but it
cannot advance the checkpoint or mutate projections. A separately governed
worker must fetch and validate full finalized transaction evidence before any
repair or reconciliation write.

ADR-0125 adds the bounded signature-to-evidence recovery endpoint. It scans at
most the configured page budget, verifies the requested signature against a
finalized `getTransaction` response, decodes the canonical event boundary,
and uses `persist_replayed_page` without touching the forward checkpoint.
Production invocation requires the exact indexer-operator role/scope; each
attempt appends immutable requested/recovered/failed audit evidence. Alert on
requested rows without a terminal outcome and reconcile them before declaring
repair complete. Live provider/failover evidence and fleet recovery SLOs
remain open.

ADR-0126 exposes `stp_indexer_targeted_recovery_stale_requested` as a
fixed-cardinality aggregate. It counts requested recovery audits older than
five minutes without a terminal row and has no target, signature, operator, or
stream labels. A non-zero value is control-plane failure evidence only; follow
the targeted recovery runbook and inspect the immutable audit ledger before
appending a terminal reconciliation outcome.

ADR-0127 adds `stp_indexer_vault_reconciliation_mismatch` and
`stp_indexer_vault_reconciliation_error`, both fixed-cardinality gauges with
only the service label. They count active targets whose latest append-only run
is a mismatch or could not prove a finalized observation. A non-zero value is
not a tax or legal-compliance conclusion; it is a stop-and-investigate signal.
Follow the vault-release incident runbook, preserve target/run evidence, and
do not edit projections or release claims to clear the alert.

Replay-gap operator actions are a separate control-plane audit stream. The
authenticated list, acknowledge, and requeue endpoints emit bounded
`indexer_replay_operator` operation metrics and preserve the operator subject,
bounded note, action, and gap identity in the append-only PostgreSQL ledger.
Acknowledgement is not repair completion. Alerting and runbooks must page on
retry exhaustion/dead-letter growth and require an incident reference before
requeue; no operator action may advance a chain checkpoint.

Projection rebuilds are a separate recovery operation. Use the pinned Docker
drill (`make projection-rebuild-drill`) to verify that retained raw finalized
event evidence recreates `indexed_receipts` with a byte-identical projection
digest after normalized rows are removed. The drill is synthetic and local-
only. Production recovery must use the authenticated operator workflow, a
bounded slot range and batch size, an incident/change reference, and
before/after checkpoint and digest evidence. An evidence conflict stops the
batch and does not advance indexing state.

## Logging rules

- Never log private keys, seed phrases, raw wallet signatures when not needed, PII, or full policy evidence.
- Log public correlation identifiers, signature, slot, receipt PDA, policy digest, merchant ID, asset mint, and error code.
- Use structured JSON logs and a redaction test in CI.
- Preserve immutable raw chain evidence for the configured retention period.

## SLOs and alerts

Initial SLOs:

- 99.95% availability for read APIs;
- 99.9% successful submission acknowledgement excluding user rejection;
- 99.99% of finalized settlements represented in projections within 60 seconds;
- zero unexplained conservation mismatches;
- zero unauthorized vault-release events.

Page on:

- conservation mismatch or unexpected vault outflow;
- upgrade authority or policy signer change outside a change window;
- indexer gap beyond one slot checkpoint interval;
- RPC provider disagreement or elevated dropped transaction rate;
- finalization latency burn-rate breach;
- policy issuer key compromise or expired key set.

Every on-chain protocol pause and unpause is a security-significant event.
Alert immediately, attach the governance signer and incident/change
reference, stop settlement retries while paused, and require an approved
recovery record before clearing the pause.

The indexer records finalized `ProtocolPauseChanged` evidence and exports
fixed-cardinality active and unknown-state signals. `STPProtocolPauseActive`
pages immediately; `STPProtocolPauseStateUnknown` is also critical because a
missing projection is not proof of an unpaused chain. Follow
[`docs/runbooks/emergency-pause.md`](../runbooks/emergency-pause.md); never
clear the state through an API or by editing PostgreSQL.

## Deployment model

- Build the React SPA once, publish content-addressed assets, and serve through NGINX.
- Run Rust API and indexer as separate deployables with independent scaling and permissions.
- Use blue/green or canary releases for off-chain services.
- Deploy Solana programs through devnet -> testnet/staging -> mainnet, with verified binaries and an approved upgrade authority.
- Every deployment records git commit, toolchain digest, program binary hash, IDL/schema version, and configuration version.
