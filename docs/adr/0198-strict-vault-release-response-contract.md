# ADR-0198: Strict vault-release lifecycle response contract

## Status

Accepted — vault-release lifecycle responses use canonical, fail-closed browser
parsing and server-owned refresh semantics.

## Context

The vault-release bounded context moves high-value lifecycle evidence through a
browser UI. Its Rust response type contains `u64` nonces, amounts, and chain
slots. Default JSON serialization can represent those fields as JavaScript
numbers, which silently loses precision above `Number.MAX_SAFE_INTEGER`. The
vault-release workspace also previously cast decoded JSON directly into its
financial lifecycle type and locally merged transition form fields into the
displayed claim.

## Decision

- Serialize vault-release `nonce`, `amount_atomic`, and optional `slot` as
  canonical decimal strings using the shared Rust serde helpers.
- Parse every claim and transition response at the SPA boundary. Validate
  UUIDv7 identities, Solana public keys, fixed digests, canonical `u64` values,
  bounded text, lifecycle states, and a strictly increasing execution window.
- After a successful transition, validate the transition envelope and reload
  the claim from the tenant-scoped API before rendering the new projection.
  If the reload fails, clear the displayed claim and require reconciliation;
  the browser never treats its submitted form values as authoritative evidence.
- Parse the refund-claim preparation response once and derive its lookup ID
  from the validated response rather than casting the raw JSON envelope.

The API remains the authority for lifecycle state, authorization, and chain
evidence. This is a wire-contract hardening change, not permission for the
browser to authorize or execute a vault release.

## Alternatives considered

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Direct TypeScript casts and local state merge | Smallest diff | Precision loss, malformed evidence reaches UI, stale state can look current | Rejected |
| Keep numeric JSON `u64` fields | Compatible with default serde output | Unsafe for JavaScript and incompatible with exact financial display | Rejected |
| Use decimal strings only in the frontend parser | Protects rendering | Existing API consumers still receive lossy numeric wire values | Rejected |
| **Canonical Rust string wire fields plus strict parser and server refresh** | Exact cross-language values, bounded evidence, authoritative lifecycle projection | Requires explicit contract migration and tests | **Selected** |

## Invariants and verification

- No vault claim with malformed identity, account, digest, amount, nonce, slot,
  status, or time window enters React state.
- `u64::MAX` survives Rust serialization and browser parsing byte-for-byte as a
  decimal string.
- A transition for a different claim or unsupported state is rejected.
- A transition success without a refreshed authoritative claim does not leave a
  stale or locally synthesized claim visible.
- Refund-claim preparation derives its intent ID only from the validated
  response.
- Rust storage wire-contract test and browser parser tests pass before UI
  integration is accepted.
