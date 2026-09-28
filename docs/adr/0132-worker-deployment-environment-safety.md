# ADR-0132: Explicit worker deployment-environment safety

- Status: Accepted
- Date: 2026-09-27
- Deciders: Principal Low-Latency Architecture, Security, SRE
- Scope: Indexer and settlement-route-verifier process startup

## Context

The indexer and route verifier share the same binaries across local
development, staging, and production. They already reject insecure local RPC
outside local development, but a missing environment label is still accepted
when secure RPC is configured. That makes a process identity ambiguous and
allows an incorrectly provisioned worker to appear production-like without an
explicit deployment contract.

## Decision

Both workers must require an environment variable whose value is exactly one
of `local`, `staging`, or `production`:

- `INDEXER_ENV` for the finalized indexer;
- `ROUTE_VERIFIER_ENV` for the settlement-route verifier.

Insecure local RPC is permitted only when the corresponding environment is
`local`. Unknown, missing, empty, or malformed environment values fail before
the worker builds an RPC transport or opens a database pool. Compose exposes
the local default explicitly and permits an operator to override it for a
non-production local-profile test.

This is the worker counterpart to ADR-0130. It validates process configuration
only; it does not grant staging or production deployment authority, secrets,
RPC allow-listing, signer access, or release approval.

## Alternatives considered

### Continue checking only the insecure-RPC flag

Rejected. Secure transport does not establish deployment identity, and a
blank environment remains ambiguous for audit, telemetry, and incident
response.

### Infer the environment from the RPC endpoint

Rejected. Endpoint names are operator-controlled configuration and do not
provide a trustworthy authority boundary; private RPC, proxies, and failover
providers also make inference unreliable.

### Separate worker binaries per environment

Rejected. This duplicates startup logic, increases release drift, and does not
protect a directly launched binary from misconfiguration.

### Selected: explicit runtime contract in each worker

Selected because it is deterministic, cheap, testable without network access,
compatible with one immutable binary, and enforced before side effects.

## Invariants and evidence

- No worker starts with a missing, empty, or unknown deployment environment.
- Non-local workers cannot construct insecure local RPC transports.
- Local Compose remains reproducible and explicitly labeled.
- Tests cover all accepted environments, missing/unknown values, and unsafe
  insecure-RPC combinations.
- The environment contract script and the full repository check verify the
  checked-in configuration and startup contract.

Staging/production provisioning, RPC provider allow-listing, identity,
secrets, HSM/KMS, and release approval remain separate gates.
