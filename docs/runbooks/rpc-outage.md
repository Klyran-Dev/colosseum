# Runbook: RPC outage or provider disagreement

## Trigger and owner

Use for timeout/error-rate alerts, finalized-slot lag, inconsistent account or
transaction responses, provider censorship suspicion, or repeated unknown
submission status. The chain-operations owner coordinates the incident.

## Contain

1. Keep commitment requirements at `finalized`; never downgrade a response to
   make the system appear healthy.
2. Allow the bounded provider failover policy to operate. Do not add an
   unreviewed endpoint, increase retries without a budget, or submit rebuilt
   transaction bytes.
3. Mark ambiguous settlement intents `unknown` and reconcile by signature and
   finalized chain state. Do not create a second submission from a timeout.
4. If no provider can prove the required finality, stop settlement submission
   while keeping read-only incident evidence collection bounded.

## Recovery

- Compare provider slot, account, transaction, and program-log responses at
  the same commitment. Persist disagreement as evidence and page operations.
- Restore only approved, authenticated endpoints and verify TLS/configuration.
- Backfill indexer ranges from finalized slots, replay bounded gaps, and use
  the [indexer-gap runbook](indexer-gap.md) for projection repair.
- Resume after provider health, finalized lag, and reconciliation alerts clear
  for the approved observation window.

## Stop conditions

Never infer settlement success from a wallet callback, WebSocket notification,
or a single provider's processed/confirmed response.

