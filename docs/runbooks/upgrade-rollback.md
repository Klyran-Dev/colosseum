# Runbook: Solana program upgrade or rollback

## Trigger and owner

Use for a failed deployment, unexpected program behavior, audit finding, or
upgrade-authority alert. The release commander owns the change; the program
security owner and upgrade multisig signers approve any on-chain action.

## Contain

1. Freeze the release pipeline and record the deployed program ID, cluster,
   slot, binary hash, IDL/schema version, upgrade authority, and change ID.
2. If settlement safety is uncertain, use the governed pause control and stop
   submission retries. Do not use a database flag as a substitute for the
   on-chain pause authority.
3. Preserve validator/RPC responses, finalized signatures, logs, and program
   data. Do not redeploy repeatedly to chase a transient provider error.

## Decide and execute

- Verify the candidate or rollback binary hash against the reviewed artifact,
  source revision, toolchain, SBOM, and audit approval.
- Confirm the upgrade authority and multisig threshold independently. A single
  API, operator, or wallet must not authorize the upgrade.
- Prefer a forward, reviewed corrective upgrade. A rollback is allowed only
  when the prior binary and state compatibility are proven for the deployed
  account schema and migration boundary.
- Execute at the approved commitment, capture multisig and finalized evidence,
  then run the applicable localnet/staging conformance suite before unpausing.

## Stop conditions

Stop if the binary hash, account layout, authority, or finalized result differs
from the approved change. Never claim rollback success from a submitted or
processed transaction alone; require finalized evidence and post-change
reconciliation.

