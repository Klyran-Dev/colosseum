# Runbook: policy signer or rule compromise

## Trigger and owner

Use when a policy issuer key, rule release, tax attestation, identity claim,
or rule provenance is suspected to be compromised. The incident commander
owns coordination; the policy-security owner owns issuer containment; the tax
owner decides whether a replacement rule is legally reviewable.

## Contain

1. Open an incident record and preserve the first alert, request IDs, policy
   digest, calculation IDs, and affected settlement signatures.
2. Disable the affected issuer or policy route through the governed control
   plane. If scope is uncertain, activate the smallest safe protocol pause
   that prevents new settlement; do not silently change historical records.
3. Stop retries and release workflows that depend on the affected policy.
   Existing `unknown` settlement intents remain unknown until finalized chain
   evidence is reconciled.
4. Revoke/rotate the issuer through the approved multisig/HSM ceremony. Never
   paste key material into a shell, ticket, log, or chat.

## Investigate and recover

- Compare the published rule document, canonical rule digest, attestation
  digest, issuer sequence, expiry, registry state, and finalized program logs.
- Mark affected calculations and settlements for review; do not mutate their
  evidence or overwrite a rule version. Corrections are new governed versions.
- Require independent author/reviewer/approver and legal/tax review before
  publishing a replacement. Re-run calculation replay and settlement binding
  checks from immutable evidence.
- Resume only after the incident commander, policy-security owner, and tax
  owner record approval, and monitoring shows no stale/revoked policy use.

## Stop conditions

Stop and escalate if signer identity, issuer sequence, finality, or affected
scope cannot be established. Automated withholding is evidence generation, not
a determination that filing or legal compliance occurred.

