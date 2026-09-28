# Runbook: vault release incident

## Trigger and owner

Use for an unexpected vault balance delta, release claim anomaly, destination
substitution, duplicate release, or unavailable release evidence. The treasury
incident commander owns containment; vault governance and finance/tax owners
approve recovery.

## Contain

1. Pause new settlement or the affected merchant/asset scope through the
   governed on-chain control when the scope is known; use global pause when it
   is not.
2. Disable release execution and signer automation. Do not transfer funds
   manually to “repair” a projection or bypass the release policy.
3. Capture finalized vault and destination account data, token program/mint,
   release claim IDs/nonces, signatures, slots, route versions, and balance
   snapshots. Preserve all database evidence.

## Reconcile

- Compare finalized token balances with the immutable receipt tax totals,
  approved release ledger, claim nonce state, and expected vault authority.
- For an ADR-0127 target, inspect the immutable opening-balance reference and
  latest reconciliation run. A mismatch or reconciliation error is a
  fail-closed stop signal; it is not permission to edit evidence or reclassify
  an unexplained transfer.
- Treat provider disagreement, missing raw evidence, Token-2022 extension
  ambiguity, or unknown finality as a mismatch requiring escalation.
- A correction must be a new governed claim or compensating workflow with its
  own evidence; historical receipts and reconciliation runs are immutable.

## Resume criteria

Require independent vault-governance and finance/tax approval, verified signer
health, a finalized balance reconciliation, and an alert test before release
automation or protocol settlement resumes. The API and indexer can never be a
sole vault authority.
