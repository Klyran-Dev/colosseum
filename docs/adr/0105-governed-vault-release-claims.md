# ADR-0105: Governed vault-release claims

- Status: Accepted
- Date: 2026-09-27
- Owners: Protocol, Treasury, and SRE
- Scope: Solana settlement program, vault-release bounded context, and release evidence

## Context

Settlement withholds tax into a merchant/asset-scoped PDA-controlled token
vault. A production tax protocol must support audited remittance, correction,
and treasury release without giving the API, indexer, policy issuer, or a
single operator a unilateral transfer capability. The current program has the
vault authority and inbound withholding path, but no bounded outbound claim
lifecycle.

The release path must remain atomic, replay-resistant, explicit about the
destination and amount, and safe under pause, expiry, operator error, and
duplicate submission. Historical settlement receipts remain immutable; a
release is a new evidence-bearing event, never a mutation of a receipt.

## Decision

Add a Solana-native vault-release bounded context with three explicit roles:

1. a governance authority initializes, rotates, or disables the release
   registry;
2. a proposal authority creates a claim; and
3. a distinct approval authority executes or rejects the claim.

The `VaultReleaseRegistry` PDA is scoped by merchant and mint. It stores the
governance, proposer, approver, active flag, and bump. The proposer and
approver must be distinct. A `ReleaseClaim` PDA is scoped by merchant, mint,
and a caller-provided nonce. It immutably binds the amount, destination token
account, reference digest, proposer, execution window, and executed/cancelled
state.

`execute_vault_release` validates the registry, claim binding, token program,
mint, vault PDA authority, destination, active status, time window, and vault
balance, then signs one checked token transfer with the vault-authority PDA and
marks the claim executed in the same instruction. `cancel_vault_release_claim`
is governance-controlled, can only cancel a pending claim, and cannot rewrite
the claim's financial fields. An expired claim cannot execute.

The claim nonce and PDA prevent replay. The claim state prevents duplicate
execution. Solana atomicity ensures a failed transfer cannot leave a partial
vault balance or a consumed claim. Release events contain the claim, scope,
amount, destination, reference digest, and actor identities for finalized
indexing and reconciliation.

This ADR does not claim that a release is tax remittance, legal filing, or
custody approval. Those meanings require a jurisdiction/product-specific
policy, legal owner, and off-chain evidence workflow.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| API signs a direct token transfer | Rejected | Custodies a privileged key and bypasses durable on-chain governance. |
| One release authority executes an amount supplied in the transaction | Rejected | No four-eyes control and weak audit binding. |
| Asynchronous per-recipient escrow accounts | Rejected for this slice | Adds partial-state recovery, more writable accounts, and a second release authority model. |
| Off-chain claim ledger with a later unconstrained transfer | Rejected | The ledger is not authoritative for token balances and cannot prevent destination substitution. |
| **Merchant/mint registry with separate proposer/approver keys and immutable release-claim PDAs** | **Selected** | Four-eyes control, exact amount/destination binding, PDA-signed checked CPI, atomic state/transfer, and finalized-chain evidence. |

## Invariants and failure behavior

- Registry merchant, mint, and token program scope are exact.
- Governance, proposer, and approver subjects are non-default; proposer and
  approver are distinct.
- Claim amount is non-zero and bounded by the vault balance at execution.
- Claim destination and mint are immutable and distinct from the vault.
- Reference digest is exactly 32 non-zero bytes.
- Execution requires `execute_after <= now <= expires_at` and a live registry.
- The approver cannot be the proposer recorded on the claim.
- An executed or cancelled claim cannot execute or cancel again.
- A duplicate claim nonce fails PDA initialization without token movement.
- Any validation, CPI, or account mismatch fails closed and leaves all state and
  token balances unchanged.
- Release events are append-only chain evidence; off-chain projections are
  rebuildable and never authorize a transfer.

## Budgets and operational boundaries

- One claim creation and one execution instruction; one checked token CPI per
  execution.
- No network, database, unbounded loop, dynamic rule graph, or floating-point
  operation on-chain.
- Claim window is bounded to 90 days to prevent indefinite authorization.
- The release registry is merchant/mint sharded; no global vault counter is
  writable in the hot path.
- The off-chain API must persist an idempotent claim command and outbox record
  before returning a release artifact. It must never hold the release key.

## Consequences and follow-on gates

This adds two registry roles, claim state, new event decoding, database/API
projection, treasury UI, and a key-ceremony surface. The on-chain slice and
validator evidence are necessary but not sufficient for production enablement.
HSM/KMS or multisig deployment, legal/tax owner approval, finalized indexer
projection, encrypted evidence retention, and staging drills remain release
gates.

## Evidence

- `programs/settlement/src/lib.rs`
- `db/migrations/0023_vault_release_claims.sql`
- `db/migrations/0024_vault_release_claim_state_invariants.sql`
- `crates/storage/src/lib.rs`
- `crates/api/src/lib.rs`
- `apps/web/src/VaultReleasePanel.tsx`
- `apps/web/src/solana.localnet.test.ts`
- `scripts/verify_local_stack.sh`
