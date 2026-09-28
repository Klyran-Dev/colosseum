# ADR-0174: Finalized refund-funding topology verification

## Status

Accepted for implementation.

## Context

ADR-0173 adds a tenant-scoped, four-eyes funding configuration, but an
approved database row is not proof that the configured Solana accounts have
the required ownership, mint, token-program, or vault-authority relationship.
Activating that row from browser input, a processed RPC response, or a single
provider would create an authority-substitution and stale-topology risk before
any refund claim is compiled.

The refund program accepts a bounded legacy-SPL topology for the first movement
contract: a merchant system account, a mint, a tax-vault token account owned by
the deterministic vault-authority PDA, a merchant-funded source token account
owned by the configured refund funder, and the configured refund-funder wallet.
Token-2022 extensions remain outside this activation contract until their
transfer-hook, fee, delegate, and authority semantics have a separate ADR and
validator evidence.

## Decision

Add a server-owned finalized topology verifier as the only path from
`approved` to `pending_verification` to `active`:

1. derive and validate the vault-authority PDA from the fixed settlement
   program, merchant, and mint; never accept a PDA supplied by the browser;
2. read the complete account set in one bounded `getMultipleAccounts` request
   at `finalized` commitment with base64 encoding;
3. require a system-owned, non-executable merchant account with empty data;
4. require the canonical legacy SPL Token program, a non-executable 82-byte
   mint account, and the configured mint decimals;
5. require exact 165-byte token-account layouts for the tax vault and net
   source, matching mint and token program, with tax-vault owner equal to the
   derived vault-authority PDA and net-source owner equal to the configured
   refund-funder key;
6. require the refund-funder account to be a non-executable system account;
7. compare successful responses from all configured providers by canonical
   slot and account evidence, failing closed on disagreement; and
8. persist the finalized slot, commitment, account digest, PDA, and verifier
   version in immutable activation evidence. Any later topology change
   requires a new configuration version and new four-eyes approval.

The verifier is a bounded indexer/provider capability, not a tax calculation,
wallet signer, or token-movement authority. It is not placed on the quote or
wallet-signing hot path. Activation is server-owned and cannot be requested by
the browser as an arbitrary status transition. The claim compiler will accept
only an active configuration with unexpired verification evidence and exact
configuration-digest binding.

## Alternatives considered

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Trust approved PostgreSQL account strings | No RPC latency | Does not prove owner, mint, layout, PDA, or finality | Rejected |
| Reuse generic settlement-route verification unchanged | Existing code and tests | Omits refund-funder wallet/source authority semantics and activation evidence | Rejected |
| Accept operator-uploaded account snapshots | Useful offline workflow | Caller-controlled evidence is not chain proof | Rejected |
| Verify each account with independent provider calls | Smaller individual responses | Mixed slots can produce a topology that never existed atomically | Rejected |
| **Complete finalized account-set read with provider agreement and immutable digest** | Atomic observation boundary, exact authority checks, replayable evidence, bounded failover | Adds a provisioning RPC operation and legacy-SPL first-scope constraint | **Selected** |

## Invariants and failure policy

- only `approved` configurations can enter verification;
- a successful provider response must contain every expected account exactly
  once and remain within the bounded response/data limits;
- account owner, executable flag, data length, mint, token program, and
  authority fields are exact and canonical;
- the configured refund-funder key is distinct from the vault authority and
  all token accounts;
- provider disagreement, missing accounts, unsupported extensions, stale
  verification windows, malformed base64, or invalid PDA derivation prevents
  activation;
- activation evidence is immutable and tenant-scoped;
- no activation evidence authorizes signing, submission, execution, or a
  paid/legal-compliance state; and
- configuration replacement is additive and versioned, never an in-place
  mutation of an active financial topology.

## Verification

- pure parser tests cover wrong owner, executable accounts, missing accounts,
  non-legacy token programs, extension-bearing layouts, mint mismatches,
  wrong token-account authorities, PDA substitution, and provider disagreement;
- provider tests prove finalized/base64 request shape, bounded response size,
  one-slot account-set evidence, deadline propagation, and complete-attempt
  failover;
- storage tests prove approval-only activation, immutable verification
  evidence, tenant isolation, idempotent replay, and topology-digest binding;
- API tests prove that browser-provided status/evidence cannot activate a
  configuration and that exact verification roles are enforced; and
- Linux validator conformance proves the configured topology, wrong-topology
  rejection, account rotation requiring a new version, and claim compilation
  rejection until the active verified configuration exists.

## Consequences

Refund configuration becomes a truthful chain-bound provisioning capability,
at the cost of one bounded finalized RPC operation and an explicit initial
legacy-SPL scope. The design keeps the low-latency claim path warm and
deterministic while preventing a database or browser from becoming the source
of authority for token movement.
