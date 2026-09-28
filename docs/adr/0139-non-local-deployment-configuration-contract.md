# ADR-0139: Fail-closed non-local deployment configuration contract

- Status: Accepted
- Date: 2026-09-27
- Deciders: Principal Low-Latency Architecture, Security, SRE
- Scope: Staging and production deployment manifests, preflight, and startup safety

## Context

ADR-0130 makes each Rust service reject ambiguous environment modes, but a
deployment manifest can still omit a database, identity, RPC, evidence-key, or
jurisdiction-provider input and rely on a local Compose default or an operator
convention. That is unsafe for a product handling tax evidence and real-money
settlement. The checked-in Compose file remains intentionally local and must not
be turned into a production template.

## Decision

Add `scripts/verify_deployment_config.sh` as a repository-side, fail-closed
preflight for `staging` and `production`. The contract requires:

- the deployment label to match every service environment label;
- simulation and insecure-local-RPC flags to be exactly `false`;
- non-local, TLS-verified PostgreSQL URLs for the migration runner, API, worker,
  indexer, and route verifier;
- bounded HTTPS RPC endpoint lists with no local hostnames or validator aliases;
- non-local identity issuer, audience, EdDSA JWKS, evidence-key, and jurisdiction
  provider inputs;
- a non-local NATS endpoint for transactional-outbox delivery; and
- no `SEED_DIR`, because synthetic fixtures are local-only.

The script is a deployment preflight, not a source of runtime authority. Each
service continues to validate its own configuration before binding or submitting
transactions. Deployment tooling must map its migration database secret to the
runner's `DATABASE_URL` and must not pass the local Compose fixture directory.
The checked-in local Compose defaults remain unchanged so clean-checkout local
development stays reproducible.

The contract uses names and structural properties only. It never prints secret
values, accepts no user/request input, and does not prove that a credential is
valid, that a provider is available, or that a deployment is approved.

## Alternatives considered

### Convention-only environment variables

Rejected. Omitted or copied local values can reach staging or production without
an executable failure at the deployment boundary.

### Deployment-tool-only validation

Rejected as the sole control. A manifest can be rendered or a binary started
outside the preferred deployment tool; service startup remains a second,
independent fail-closed boundary under ADR-0130 and the worker/indexer contracts.

### Separate staging and production binaries

Rejected. Duplicated startup and security logic increases release drift and does
not establish that the supplied endpoints, keys, or database are appropriate.

### Selected: shared preflight plus service-owned runtime validation

Selected because it catches cross-service omissions and local-default leakage
before rollout while preserving one immutable binary and independent authority
checks at each process boundary. The preflight is cheap enough for CI and the
deployment pipeline; runtime checks remain authoritative when a process is
started by another mechanism.

## Invariants and failure behavior

- No staging or production preflight succeeds with local database, RPC, NATS,
  identity, evidence-key, or seed-fixture values.
- The deployment label and service labels cannot disagree.
- Database TLS verification is explicit; the contract does not infer security
  from a hostname or deployment name.
- Synthetic tax fixtures are never a valid non-local input.
- A passing preflight does not imply secret validity, chain finality, legal tax
  correctness, HSM/KMS ceremony completion, or production approval.

## Evidence and remaining gates

`make deployment-config-contract` exercises one synthetic positive staging
manifest plus negative simulation and seed-directory cases. `make check` runs
the same contract. Actual staging/production secret provisioning, allow-listing,
deployment approval, provider health, HSM/KMS or multisig ceremonies, live
latency evidence, and mainnet release review remain open gates.
