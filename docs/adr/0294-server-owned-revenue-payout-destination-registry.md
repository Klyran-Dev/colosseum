# ADR-0294: Server-owned revenue payout-destination registry

## Status

Accepted — implementation in progress. Revenue-stream claim compilation and
submission remain disabled until this boundary, finalized verifier evidence,
and Linux validator conformance are complete.

## Context

A revenue-stream claim needs an exact SPL or Token-2022 destination token
account. The plan identifies a recipient owner and an asset, but those values
are not proof that a particular token account exists, is controlled by that
owner, or is bound to the intended mint and token program. The existing
settlement route's `net_destination` is merchant topology and cannot be reused
for a partner payout.

The browser must not provide merchant-side account addresses, token-program
choices, or a client-selected destination. Deriving or creating an associated
token account during wallet signing would add an RPC dependency, mutable
account topology, and an avoidable race to the latency-critical path.

## Decision

Create a dedicated `revenue_stream_destination_routes` bounded context. A
route is a server-owned, versioned record keyed by merchant, recipient owner,
and mint. It contains the exact destination token account and token program,
plus the verification digest and finalized account evidence that proved:

1. the destination account exists at finalized commitment;
2. its owner is the configured SPL or Token-2022 program;
3. its canonical token-account layout is supported;
4. its mint equals the configured mint; and
5. its token authority equals the configured recipient owner.

Only an active, effective, finalized-verified route can be resolved by the
application layer. Activation is performed by the bounded route-verifier
workflow using `getMultipleAccounts`, fixed account order, provider agreement,
and an atomic database transition. `local_synthetic` evidence is accepted only
by Docker fixtures and cannot activate a staging or production route.

New revenue plans bind the resolved destination route version/digest in the
same proposal transaction. Existing plans without this binding remain
readable but cannot compile a claim. A route is immutable after activation;
changes create a new version and require new finalized evidence. The browser
receives only an opaque route identity or a prepared claim envelope; it never
chooses destination topology.

The claim compiler resolves the plan's bound route from PostgreSQL, verifies
its effective window and evidence status, and emits the exact destination
account in the deterministic instruction. It does not call RPC, create ATAs,
or infer account addresses.

## Alternatives considered

### Browser-supplied destination account

Rejected. It creates an account-substitution boundary and would require the API
and signer to trust topology supplied by an untrusted client.

### Derive or create an ATA during signing

Rejected. It adds an RPC call and potentially a second instruction to the hot
path, permits mutable topology, and creates a new failure/replay surface.

### Reuse the merchant settlement route

Rejected. The merchant's net destination is not the partner's token account;
reusing it would violate recipient-owner and tenant-account invariants.

### Store a destination without chain verification

Rejected. Database state is configuration, not proof of Solana account state.

### Dedicated immutable route with finalized verification

Selected. It preserves authority separation, deterministic compilation,
bounded latency, explicit Token-2022 compatibility, and replayable evidence.

## Invariants

- One active route exists for a merchant, recipient owner, and mint at an
  effective instant.
- Active routes have finalized verification evidence and a non-zero digest.
- Destination, mint, token program, and recipient owner are valid, distinct
  where required, canonical Solana public keys.
- A plan cannot compile or submit a claim without an active bound route.
- Route data is tenant-scoped; a route from another merchant is never
  resolvable through an authenticated plan.
- Route verification is outside quote, compile, wallet-signing, and submit
  hot paths.

## Consequences

The first setup flow must include destination registration and verification,
with clear pending/failed/active states. This adds an operational step but
prevents the UI from presenting a payout as executable before chain topology is
known. The registry also gives support and audit users a durable explanation of
which account received a claim and which finalized evidence authorized it.

## Release gate

Do not expose revenue claim compilation, wallet submission, or a UI state that
implies transfer completion until migration, verifier tests, message binding,
Linux validator conformance, and production signer/approval controls pass.
