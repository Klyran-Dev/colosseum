# ADR-0220: Jurisdiction-provider JSON deployment-preflight parity

- Status: Accepted
- Date: 2026-10-01
- Deciders: Principal Low-Latency Architecture, Security, SRE
- Bounded contexts: Jurisdiction Evidence, Identity Configuration, Deployment Safety

## Context

API startup supports a structured `API_JURISDICTION_PROVIDER_KEYS_JSON`
registry for provider-key rotation and a compatibility
`API_JURISDICTION_PROVIDER_KEY_ID` / `API_JURISDICTION_PROVIDER_PUBLIC_KEY`
pair. The staging/production deployment preflight must enforce the same
mutually exclusive configuration contract. Requiring only the legacy pair
would reject a valid provider-managed rotation rollout; accepting both forms
would create ambiguous authority and make rotation drift difficult to detect.

## Decision

The deployment preflight accepts exactly one jurisdiction-provider key form:

- when `API_JURISDICTION_PROVIDER_KEYS_JSON` is present, require a non-empty
  object-shaped document, reject known local-development material, and reject
  either compatibility variable;
- otherwise require both compatibility variables and retain their local-value
  rejection checks.

The shell checker performs only secret-safe structural checks. Rust startup,
the provider registry parser, signature verification, key validity, provider
rotation state, and HSM/KMS custody remain authoritative.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| Require only the legacy key pair | Rejected | Blocks the supported structured registry and planned key rotation. |
| Allow structured and legacy forms together | Rejected | Creates two possible authorities and configuration drift. |
| Remove jurisdiction-key checks from preflight | Rejected | Allows missing or local provider-verification material into rollout. |
| Parse provider-key cryptography in POSIX shell | Rejected | Duplicates the Rust parser and expands the preflight attack/tooling surface. |
| **Accept exactly one supported form with bounded structural checks** | **Selected** | Preserves fail-closed parity with API startup without making shell the cryptographic authority. |

## Invariants and failure policy

- Exactly one structured registry or complete compatibility pair is present.
- Empty or known local-development registry material fails the preflight.
- Partial compatibility configuration fails the preflight.
- Key material is never printed by the checker.
- Passing preflight does not prove provider signatures, live rotation,
  HSM/KMS custody, legal approval, or production deployment authorization.

## Verification and release impact

- The deployment contract test proves a positive structured registry case and
  retains negative simulation, insecure-RPC, seed-fixture, and mixed-form cases.
- The API startup parser remains covered by Rust tests and remains the runtime
  authority for registry syntax and cryptographic semantics.
- `make deployment-config-contract` and `make check` are required before
  merge. Live provider rotation and HSM/KMS ceremony evidence remain open
  production gates.
