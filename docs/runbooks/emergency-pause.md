# Emergency protocol pause runbook

This runbook governs a production incident in which Solana settlement must be
contained. The on-chain `ProtocolConfig.paused` account is authoritative. The
API, indexer, Redis, and PostgreSQL projections cannot authorize or clear a
pause.

## Trigger conditions

Page the protocol-security owner immediately for either:

- `STPProtocolPauseActive`; or
- `STPProtocolPauseStateUnknown`, which is fail-closed and must be treated as
  unsafe until finalized chain evidence is verified.

Also consider a governed pause for a confirmed policy-issuer compromise,
unexpected program behavior, amount-conservation defect, or unexplained vault
outflow. Use the smallest safe scope available, but do not create an API-side
simulation of a pause.

## Containment

1. Appoint an incident commander and record the incident/change identifier.
2. Stop settlement retries, quote-to-submit promotion, and vault-release
   execution. Preserve unknown transaction status as unknown; do not replay it
   by assumption.
3. Verify the finalized `ProtocolPauseChanged` event, signature, slot,
   authority, event digest, and the current `protocol-config` account through
   independent RPC providers.
4. If a pause is required, submit `set_protocol_paused(true)` through the
   approved multisig/timelock or break-glass governance path. The signer must
   remain outside API/web custody. Record the proposal, approvals, signer
   identity, transaction signature, and finalized slot.
5. Confirm that new settlement attempts fail with the on-chain paused error and
   that source, tax-vault, destination, and receipt balances remain unchanged.
6. Keep the pause active while policy, program, RPC, indexer, and vault evidence
   are collected. Attach all evidence to the incident record and preserve the
   immutable chain signatures.

## Recovery and unpause

Unpause requires all of the following in the incident record:

- incident commander approval;
- protocol-security approval;
- tax/legal owner approval when a tax policy or withholding defect is in scope;
- verified program binary, upgrade authority, policy issuer, and RPC health;
- finalized vault reconciliation with no unexplained mismatch;
- a successful localnet and staging rehearsal against the release candidate;
- approved rollback/release reference and explicit customer-support plan.

Submit `set_protocol_paused(false)` only through the same governed signer
boundary. Verify the finalized unpause event independently, then resume traffic
in bounded stages while watching settlement conservation, replay, RPC
disagreement, latency, and vault alerts. Do not close the incident until the
post-incident evidence bundle is complete.

## Operator safety

Never paste private keys, seed phrases, HSM material, or raw sensitive tax
evidence into tickets or logs. Never use a caller-supplied merchant identifier
to choose an authority. Never suppress the unknown-state alert to make a
dashboard appear healthy.
