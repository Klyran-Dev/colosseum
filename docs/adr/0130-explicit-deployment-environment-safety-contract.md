# ADR-0130: Explicit deployment-environment safety contract

- Status: Accepted
- Date: 2026-09-27
- Deciders: Principal Low-Latency Architecture, Security, SRE
- Scope: API process startup and local/staging/production configuration

## Context

The same Rust API binary is used for local development, staging, and the
future production deployment. A blank or misspelled environment value can
silently change transport security or leave simulation behavior enabled at the
wrong boundary. Compose also needs safe local defaults without becoming a
production deployment template.

Environment selection is an authority boundary. It must be explicit,
machine-validated, and fail closed before the API binds its listener. Local
insecure RPC is useful for a synthetic validator, but it must never be
available to staging or production through a permissive boolean combination.

## Decision

Introduce a small pure Rust configuration contract in the API startup path:

- `API_ENV` is required and must be exactly `local`, `staging`, or
  `production`.
- `API_SIMULATION_MODE=true` is valid only for `API_ENV=local`.
- `API_ALLOW_INSECURE_LOCAL_RPC=true` is valid only for `API_ENV=local`.
- Boolean configuration accepts only the literal values `true` and `false`.
- Compose defaults explicitly to `API_ENV=local`; it does not provide staging
  or production credentials, endpoints, or deployment authority.
- The validated environment is reused by RPC construction instead of repeating
  independent string comparisons in the process entry point.

Staging and production still require separate secret, RPC, identity, key,
database, and approval provisioning. This contract prevents unsafe mode
combinations; it does not claim that environment labels alone constitute a
production launch approval.

## Alternatives considered

### Blank environment with permissive defaults

Rejected. Missing configuration is ambiguous and can make an operator believe
the process is protected when it is not.

### Separate binaries per environment

Rejected. It duplicates security-sensitive startup logic and increases release
drift without improving authority separation.

### Deployment-tool-only validation

Rejected. A process can be started outside the deployment tool, and the binary
must defend its own listener and RPC boundary.

### Selected: explicit runtime contract plus checked-in local defaults

Selected because the validation is deterministic, cheap, testable without
network access, and enforced at the last authority boundary before service
startup while preserving one immutable binary across environments.

## Invariants and failure behavior

- The API does not bind a listener when `API_ENV` is missing, unknown, or
  inconsistent with simulation/insecure-RPC flags.
- Non-local environments cannot construct an insecure RPC transport.
- A malformed boolean is an error, never an implicit false or true.
- Local Compose remains reproducible and explicitly labeled as local.
- Environment validation does not accept merchant-supplied or request-supplied
  values; it is process configuration only.

## Evidence and remaining gates

The Rust configuration tests, `make environment-contract`, Compose contract,
and startup wiring provide local evidence. Staging/production secret
provisioning, deployment approvals, HSM/KMS and multisig ceremonies, mainnet
allow-listing, and emergency pause rehearsal remain independent release gates.
