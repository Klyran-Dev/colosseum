# ADR-0219: JWKS JSON deployment-preflight parity

- Status: Accepted
- Date: 2026-10-01
- Deciders: Principal Low-Latency Architecture, Security, SRE
- Bounded contexts: Identity Configuration, API Startup, Deployment Safety

## Context

The API startup contract accepts exactly one of two authentication-key
representations: a bounded structured `API_AUTH_JWKS_JSON` document for
rotating identity-provider keys, or the compatibility pair
`API_AUTH_JWKS_KID`/`API_AUTH_JWKS_PUBLIC_KEY`. The repository deployment
preflight required the compatibility pair unconditionally. A valid staged or
production deployment using a provider-managed JWKS document would therefore
be rejected before rollout, while weakening the check would risk allowing
local identity fixtures through.

## Decision

Update `scripts/verify_deployment_config.sh` to accept exactly one
authentication-key representation:

- when `API_AUTH_JWKS_JSON` is present, require a non-empty structured value,
  reject known local-development material, and reject either compatibility
  variable;
- otherwise require the existing `API_AUTH_JWKS_KID` and
  `API_AUTH_JWKS_PUBLIC_KEY` pair and retain their local-value checks.

The preflight remains structural and secret-safe. It does not parse or verify
provider signatures, key validity, issuer metadata, rotation state, or HSM/KMS
ceremony completion; the API startup parser and identity provider remain
authoritative for those controls.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| Require only the legacy key pair | Rejected | Blocks the supported rotated-JWKS deployment form. |
| Allow both forms simultaneously | Rejected | Creates ambiguous authority and key-rotation drift. |
| Remove key checks from the repository preflight | Rejected | Permits local identity fixtures and missing authentication material to reach rollout. |
| Parse full JWKS cryptography in the shell preflight | Rejected | Duplicates the Rust authority parser and expands the deployment toolchain. |
| **Accept exactly one supported form with structural/local-value checks** | **Selected** | Preserves fail-closed deployment safety and parity with API startup. |

## Invariants and failure policy

- Exactly one of structured JWKS or the compatibility pair is configured.
- Empty or known local-development JWKS material fails the preflight.
- A partial compatibility pair fails the preflight.
- Secrets and key material are never printed by the checker.
- Passing the preflight does not prove key validity, provider availability,
  HSM/KMS custody, or deployment approval.

## Verification and release impact

- The deployment contract test proves one positive structured-JWKS staging
  configuration and retains negative simulation, insecure-RPC, and seed-fixture
  cases.
- The API startup contract remains covered by its Rust tests and uses the same
  mutually exclusive representation rule.
- `make check` and `make deployment-config-contract` are required before merge.
- Live identity-provider rotation and production key-ceremony evidence remain
  release gates.
