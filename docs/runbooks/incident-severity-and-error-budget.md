# Incident severity and error-budget runbook

This runbook is a public response contract. Bind the generic roles to named
teams and approved contact mechanisms in deployment records. Never put secrets,
private keys, PII, or raw tax evidence in an incident channel.

## Severity

| Severity | Trigger | Immediate authority | Recovery target |
| --- | --- | --- | --- |
| P0 — financial integrity | Unauthorized outflow, conservation mismatch, forged/ambiguous authority, or evidence proving an unsafe settlement | Incident commander pauses the affected protocol route/release authority and starts security/finance response | Contain immediately; recovery evidence and executive/legal escalation are required |
| P1 — settlement safety | Finality disagreement, signer boundary failure, widespread unknown intents, policy issuer compromise suspicion, or unreconciled vault mismatch | Stop affected settlement/release operations; preserve unknown states and signatures | Containment ≤15 min; service recovery target follows the SLO policy |
| P2 — material degradation | SLO burn, provider outage with safe failover, indexer backlog, replay dead-letter growth, or repeated quote/compile failures without financial loss | SRE on-call owns mitigation; feature release may be frozen | Mitigation ≤30 min; corrective action tracked |
| P3 — limited/non-production | Isolated UI defect, documentation/observability defect, or non-production failure with no financial exposure | Owning team triages during business-hours support | Next planned maintenance unless it becomes a higher severity |

When severity is uncertain, use the higher severity until evidence lowers it.
The incident commander may pause activity; only the separately governed
authority may unpause a protocol or release path.

## First fifteen minutes

1. Create an incident ID and assign incident commander, technical lead,
   communications lead, and scribe.
2. Record environment, region, program ID, route/policy digests, affected
   intent IDs, signatures, slots, and the last known finalized state.
3. Stop the smallest affected authority boundary: quote publication,
   settlement submission, vault release, or indexer projection. Do not delete
   rows, retry blindly, or mutate signed bytes.
4. Preserve logs, RPC responses, database evidence, alert snapshots, and exact
   commands. Mark unknown chain outcomes for signature-based reconciliation.
5. Compare Solana finalized evidence with PostgreSQL projections. Redis,
   WebSocket notifications, wallet toasts, and dashboards are not authority.
6. Page the security, finance operations, SRE, and tax/legal owners when the
   trigger involves money movement, policy provenance, or personal data.

## Error-budget response

- At 50% remaining: open a reliability work item and stop making unsupported
  latency or availability claims.
- At 25% remaining: require owner approval for unrelated changes and prioritize
  overload, backpressure, dependency, and reconciliation work.
- At 0% remaining: freeze unrelated releases. Security, correctness, and
  recovery work may proceed with incident-commander approval.
- A P0/P1 financial-control event supersedes the availability budget and blocks
  release even when uptime is within target.

## Recovery and closure

1. Define the containment boundary and the evidence that proves it is closed.
2. Reconcile every possibly accepted signature against finalized chain state;
   do not infer state from a timeout or provider response alone.
3. Restore projections from retained evidence or PITR using the bounded runbook;
   record before/after checkpoint and digest values.
4. Require a second reviewer for unpause, key rotation, policy rollback, or
   release-claim action.
5. Close only when the SLO/error-budget impact, customer communication,
   evidence links, corrective actions, and owner/due dates are recorded.
6. Update the relevant ADR, dependency register, alert, test, and runbook if
   the incident exposed a design gap.

## Evidence checklist

- incident ID, severity transitions, timeline, and accountable roles;
- alert/query revision, affected SLO, budget calculation, and sample window;
- immutable database event IDs, chain signatures, slots, receipt PDAs, and
  finalized balance observations;
- exact deployment/config/program hashes and change references;
- sanitized customer/support communication;
- root cause, contributing factors, corrective/preventive actions, and review
  date.
