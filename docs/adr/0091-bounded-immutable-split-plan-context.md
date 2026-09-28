# ADR-0091: Bounded immutable Split Plan context

- Status: Accepted for implementation
- Date: 2026-09-27
- Owners: Settlement, Merchant, and Treasury platform owners

## Context

The domain model and settlement-flow documentation describe revenue splits, but
the first executable settlement slice currently has one `net_destination`
account and one net transfer. Treating a list of recipients as an API-only
feature would make the on-chain account topology and amount conservation
incomplete. It would also make a compromised or stale API able to substitute a
recipient without a bounded, reviewable account contract.

Revenue splits are a separate bounded context from tax policy. A split plan
must therefore be versioned and immutable, have a finite account footprint,
and be validated by the Solana program during the same atomic settlement as
the tax withholding transfer. Existing single-recipient clients cannot be
broken while this context is introduced.

## Decision

Introduce a bounded Split Plan context with an additive settlement instruction
version.

### Plan contract

- A plan is scoped by merchant, mint, and positive plan version.
- A published plan is immutable. A correction is a new version; old versions
  remain addressable for evidence and replay.
- Version 1 supports only proportional recipient allocations in basis points,
  with a denominator of 10,000 and a maximum of eight recipients.
- Each recipient is an exact pre-created token-account address. The plan does
  not discover accounts, create ATAs, invoke swaps, or perform external calls
  in the settlement path.
- Recipient addresses must be distinct, must not alias the payer source or tax
  vault, and every basis-point value must be positive. The sum must be exactly
  10,000.
- A plan digest is the canonical hash of the schema version, merchant, mint,
  plan version, ordered recipient token accounts, and basis points. The plan
  PDA is derived from the merchant, mint, and version; the digest is checked
  against the account content and included in settlement evidence.

### Allocation semantics

For net amount `N`, recipient `i` receives:

```text
floor(N * bps_i / 10_000) + residual_i
```

The residual is allocated one atomic unit at a time to the largest fractional
remainders. Ties are resolved by ascending plan ordinal. The implementation
uses checked integer arithmetic, never floating point, and proves that the
allocation sum equals `N`. This makes truncation and residual behavior
portable across Rust, TypeScript, and the Solana program.

### Settlement contract

- Keep the existing one-destination `settle` instruction and its v1 golden
  vector unchanged for compatibility.
- Add a versioned split-settlement instruction with the Split Plan PDA and
  exactly one writable remaining token account per plan recipient, in plan
  order. The program validates every account before any CPI transfer.
- The split instruction reuses the cached policy attestation for gross, tax,
  net, merchant, mint, expiry, issuer, and digest validation. The immutable
  Split Plan PDA separately binds the destination topology.
- Tax transfer, all recipient transfers, receipt creation, and the split
  settlement event remain one atomic Solana instruction. Any invalid recipient,
  alias, arithmetic overflow, unsupported token layout, stale plan, or failed
  CPI aborts the whole instruction.
- The split instruction uses the existing merchant-scoped receipt PDA, so a
  client nonce cannot succeed once through v1 and again through split v2.
- Off-chain compilation, route lookup, API contracts, browser review, indexer
  decoding, and evidence storage gain a schema/versioned split-plan field.
  They must not accept arbitrary caller-supplied recipient lists in place of a
  persisted authenticated plan.

### Bounds and budgets

- Maximum recipients: 8.
- Maximum additional writable token accounts: 8.
- No dynamic fan-out, nested plans, fixed-plus-proportional rules, or
  recipient-specific external calls in v1.
- The compiler rejects plans exceeding the bound before serialization; the
  program repeats the bound and account-order checks at the authority boundary.
- Compute-unit and transaction-size budgets are measured for one and eight
  recipients. A production release requires margin against the selected
  cluster limits and a documented account-lock contention result.

## Alternatives considered

| Option | Advantages | Costs and risks | Decision |
| --- | --- | --- | --- |
| Keep one destination and distribute off-chain later | Smallest instruction | Breaks atomic conservation, introduces custody/credit risk, and makes evidence non-atomic | Rejected |
| Put arbitrary recipient arrays in the instruction | Flexible caller behavior | Unbounded account metas, recipient substitution, poor predictability, and difficult audits | Rejected |
| Create one escrow per recipient and settle asynchronously | Isolates recipient failures | Adds partial-state recovery, extra fees, latency, and a second authority model | Rejected |
| Replace the v1 instruction and account layout | One unified contract | Breaks existing signed-message vectors and deployed-client compatibility | Rejected |
| **Immutable bounded Split Plan PDA plus additive split instruction** | Atomic, deterministic, auditable, bounded, and backward compatible | Requires a new plan lifecycle, compiler schema, indexer event, and conformance vectors | **Selected** |

## Invariants

1. `sum(recipient_amounts) = net_atomic`.
2. `tax_atomic + sum(recipient_amounts) = gross_atomic`.
3. Each recipient amount is `<= u64::MAX` and every intermediate multiplication
   is checked.
4. The ordered runtime account list exactly matches the immutable plan.
5. No source, vault, or duplicate recipient account is accepted.
6. A plan version/digest cannot be silently mutated after publication.
7. A duplicate merchant-scoped nonce cannot create a second receipt.
8. Failed validation or CPI produces no partial token movement.

## Consequences

The first implementation increment adds the pure allocation domain library,
golden vectors, immutable PDA, additive on-chain instruction, and deterministic
compiler artifact. The remaining vertical slice must add authenticated
persisted-plan/API contracts, route activation, indexer decoding, browser
review, localnet execution, and performance evidence. Until those gates pass,
the repository must not claim production revenue-split support; the existing
v1 single-destination API path remains the only enabled settlement workflow.

## Evidence required before production enablement

- Rust, TypeScript, and on-chain allocation vectors agree byte-for-byte on
  canonical plan digests and amount allocations.
- Property and fuzz tests cover zero, one-unit, exact-divisible, residual,
  tie, `u64::MAX`, overflow, duplicate, alias, and count-boundary cases.
- Validator-backed tests prove legacy SPL Token and supported Token-2022
  behavior, atomic rollback, duplicate nonce fencing, and event decoding.
- Load tests report p50/p95/p99 compile and submit latency, compute units,
  transaction bytes, writable-account contention, and eight-recipient limits.
- Public runbooks and UI states explain plan publication, supersession,
  stale-plan rejection, and reconciliation-required outcomes.
