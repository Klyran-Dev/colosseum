# ADR-0411: Versioned Solana applicability settlement binding

- Status: Accepted
- Date: 2026-10-06
- Deciders: Principal Low-Latency Architecture, Solana, Tax Domain, Security
- Scope: Quote-to-settlement binding for server-owned applicability contexts

## Context

ADR-0409 persists a tenant-scoped, immutable applicability context and records
its exact attestation digest with a quote. A settlement must carry that same
meaning into the wallet-signed Solana message and finalized receipt. A database
row or browser response field is not sufficient evidence: a caller must not be
able to replace the context after quote creation, and the program must reject a
missing or empty binding.

The existing `settle` and `settle_document` instructions already have deployed
wire layouts and cross-language vectors. Inserting optional fields into either
instruction would change its discriminator payload and could cause old clients,
old IDLs, or already-reviewed message parsers to interpret bytes incorrectly.
Changing the existing receipt account layout would also make old receipt
accounts unsafe to deserialize.

## Decision

Add separately versioned `settle_with_applicability` and
`settle_document_with_applicability` instructions. They retain the existing
settlement fields and append:

1. a 16-byte UUIDv7 context identifier;
2. a 32-byte signed applicability-attestation digest.

The new instructions create a separate immutable
`ApplicabilitySettlementBinding` PDA, scoped to the new receipt PDA. The PDA
stores the receipt, merchant, mint, context identifier, attestation digest, and
bump. The existing aggregate and document receipt account layouts remain
unchanged. The program checks both binding values are non-zero, binds the
binding PDA to the receipt/merchant/mint, and emits a dedicated event. The
off-chain client derives the binding PDA and includes it in the canonical
account order; the API obtains both values only from the stored quote and
server-owned context projection.

The API selects a versioned applicability-bound compiler whenever a quote has a
published context. Legacy compilation remains available only for explicitly
arithmetic-only compatibility quotes. The browser may supply payer and nonce,
but never context ID, attestation digest, merchant-side accounts, or program ID.

The binding PDA is evidence, not a replacement for the signed applicability
attestation or finalized chain observation. Indexing must verify the event and
binding account against the stored quote before marking the quote settled.

## Alternatives considered

| Alternative | Compatibility | On-chain authority | Latency/complexity | Decision |
| --- | --- | --- | --- | --- |
| Add optional fields to `settle`/`settle_document` | Fails old wire layouts and IDLs | Partial | Low implementation effort, high migration risk | Rejected |
| Put context only in API/receipt metadata | Passes no program-level binding | Fails; wallet can omit or replace it | Fast but not auditable on-chain | Rejected |
| Add a second post-settlement binding instruction | A payment can succeed without the binding | Fails atomicity | Adds a race and a second wallet action | Rejected |
| Replace existing receipt layouts with larger structs | Risks old account deserialization and migration | Partial | Requires account migration | Rejected |
| **Versioned instructions plus a separate binding PDA** | **Preserves existing contracts** | **Passes atomic, PDA-scoped binding** | **Bounded extra account/data cost** | **Selected** |

## Invariants

- The new discriminator is distinct from every legacy settlement discriminator.
- Context ID and attestation digest are non-zero and appear exactly once in the
  instruction data and binding PDA.
- Binding PDA seeds include the new receipt PDA; its stored merchant and mint
  match the settlement accounts.
- The API derives values from the persisted quote/context and rejects a client
  override before compilation.
- Legacy vectors remain byte-for-byte unchanged.
- A finalized settlement is not marked applicability-bound until the indexed
  binding PDA/event matches the quote's stored context ID and digest.

## Implementation sequence

1. Add failing settlement-client vectors for both new instruction variants and
   PDA/account ordering.
2. Add the bounded binding account and versioned Anchor instructions while
   preserving legacy handlers and receipt layouts.
3. Add API selection, request/instruction digest binding, and replay tests.
4. Add indexer verification and evidence projection tests.
5. Add UI evidence disclosure only after the API exposes finalized binding
   status; never present a prepared artifact as settled.

## Consequences

The new path consumes one bounded account and 48 additional instruction bytes,
which remains inside the existing packet safety budget. There are two supported
settlement wire contracts during migration, so the program IDL, vectors,
browser envelope validator, and indexer must be version-aware. This deliberate
compatibility cost is preferable to silently changing the meaning of existing
signed bytes.
