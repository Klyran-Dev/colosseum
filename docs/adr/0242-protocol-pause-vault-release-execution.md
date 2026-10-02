# ADR-0242: Protocol pause gates vault-release execution

## Status

Accepted for implementation. Every on-chain token-moving instruction in the
settlement program must read the protocol pause PDA and fail before any CPI
when the protocol is paused.

## Context

The settlement, document, split, and adjustment-refund instructions already
reject execution while `ProtocolConfig.paused` is true. Vault release claims
are governed by a separate proposal/approval lifecycle, but their execution
still transfers tokens from the tax vault. The existing `execute_vault_release`
instruction did not include the protocol configuration account and therefore
could bypass an emergency pause.

That bypass violates the emergency-stop invariant and creates an unsafe
operator surprise: a pause would not mean that all protocol-controlled token
movement had stopped.

## Decision

Add the read-only `protocol_config` PDA to `ExecuteVaultRelease` and require
the release validation context to carry the pause state. The validation fails
with `ProtocolPaused` before account-side CPI transfer or claim mutation.
Claim creation and governance cancellation remain available while paused so
operators can document or unwind pending governance state; only token movement
is stopped.

The instruction account order is versioned and changes intentionally. The
localnet conformance client, account-order tests, and public wire-contract
documentation must update together. This is a fail-closed compatibility
change, not an optional client hint.

## Alternatives considered

See `docs/architecture/alternative-analysis.md`.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Pause only ordinary checkout settlement | No account-list change | Treasury release remains an emergency-stop bypass | Rejected |
| Check pause in the API only | No program change | Direct wallet/RPC callers can bypass the API; database state is not chain authority | Rejected |
| Add a mutable release-specific pause flag | Localized state | Duplicates governance state and can diverge from the protocol emergency stop | Rejected |
| **Read the existing protocol pause PDA in release execution** | One authoritative pause state, fail-closed before CPI, consistent with all other movement instructions | Adds one read-only account to the versioned instruction | **Selected** |

## Invariants and failure policy

1. A paused protocol cannot execute a vault release.
2. The pause check happens before the first token CPI and before claim state is
   marked executed.
3. An unpaused protocol preserves the existing two-person authority, claim
   window, account-binding, and conservation checks.
4. Claim creation and governance cancellation do not transfer tokens and remain
   available during a pause.
5. Missing, substituted, or incorrectly ordered protocol configuration fails
   through Anchor PDA/account validation; clients must not omit or replace it.

## Verification

- A failing-first program test rejects release execution with a paused context.
- The real localnet conformance case proves paused release execution leaves
  vault and destination balances unchanged; the retained evidence verifier
  requires the explicit `blocks paused vault release` case label.
- The unpaused release and duplicate/cancelled-claim cases remain covered.
- `cargo fmt --all -- --check`, Clippy, workspace tests, and the Linux validator
  conformance gate remain required.

## Consequences

Emergency pause semantics become uniform across all protocol token movement.
The vault-release instruction account list changes and must be deployed and
compiled atomically with clients; old signed artifacts are intentionally not
accepted across the account-contract change.
