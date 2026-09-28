# ADR-0052: On-chain settlement pause control

- Status: Accepted
- Date: 2026-09-26
- Owners: Protocol / Security
- Scope: Solana settlement program emergency control

## Context

The settlement instruction is the authority for token movement. Policy
attestations, account constraints, replay protection, and conservation checks
prevent invalid individual settlements, but they do not provide a bounded
incident response control when a signer, policy issuer, token integration, or
deployment is suspected to be compromised.

An emergency stop must be enforced by the program before any token CPI. It
must not depend on the API, Redis, an indexer notification, or an operator's
ability to stop every client. The control must also avoid silently changing
the semantics of policy registration, revocation, or evidence operations that
may be needed during incident response.

## Decision

Add a singleton `ProtocolConfig` PDA at the fixed seed `protocol-config`:

- it stores a distinct governance authority, the paused bit, and its bump;
- initialization is explicit and paid by the authority;
- only the configured authority may set or clear the paused bit;
- `settle` requires the config PDA and rejects while paused, before any token
  transfer or receipt mutation;
- policy issuer registration, issuer rotation, attestation registration, and
  attestation revocation remain available while paused so governance can
  rotate or revoke unsafe policy and preserve evidence; and
- pause/unpause emits a structured event and returns explicit error codes.

The pause authority is a governance-controlled signer in deployment. The
program upgrade authority remains a separate Anchor/Solana authority and is
not implicitly granted by the pause PDA. Production deployment must use
multisig/timelock policy with a separately documented break-glass path.

The config is mandatory for settlement rather than an optional account. A
missing or incorrectly seeded config fails account validation, which avoids a
client-selectable fail-open mode.

## Alternatives considered

- **API or NGINX kill switch:** rejected because direct wallet transactions
  and already-built transactions bypass off-chain control.
- **Per-merchant pause only:** rejected as the sole control because a shared
  program or token integration incident may require an immediate global stop;
  merchant-level controls can be added as a separate bounded policy later.
- **Pause inside the policy attestation:** rejected because it couples
  incident response to issuer availability and makes emergency action depend
  on publishing a new tax policy.
- **Upgrade the program to stop settlement:** rejected as the first response
  because upgrade propagation is slower, more privileged, and less explicit
  than a small mutable control account.
- **Pause all program instructions:** rejected because revocation, evidence,
  and governance recovery operations must remain possible during an incident.

## Invariants and failure modes

- No settlement token CPI executes when `ProtocolConfig.paused` is true.
- No non-authority signer can change `paused`.
- The config PDA is bound to the fixed seed and its stored bump.
- The pause check precedes all settlement side effects.
- A missing config cannot be interpreted as unpaused.
- Pause state changes are observable and replayable from finalized program
  events.

## Budgets and operations

The pause check is one account read and one boolean branch. It must add no
network round trip and no unbounded work. Operators must alert on every pause
and unpause event, record the signer and deployment change window, and require
an incident or change reference before clearing the pause.
