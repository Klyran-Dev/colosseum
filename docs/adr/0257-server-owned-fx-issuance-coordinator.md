# ADR-0257: Server-owned FX issuance coordinator

- Status: Implemented as a vendor-neutral composition boundary; API wiring pending
- Date: 2026-10-02
- Decision owners: FX Provider bounded context, Platform/SRE, Tax/Legal

## Context

The repository now has bounded provider acquisition, deterministic outcome
ordering, policy snapshot reload, registry-bound async issuance, and separate
HTTP provider/signer transports. They are still independently callable library
boundaries. A production composition needs one server-owned application
orchestrator that performs the complete acquisition-to-evidence sequence while
keeping the policy snapshot stable across concurrent reloads.

## Decision

Add `FxEvidenceCoordinator` to the `fx-provider` bounded context. It receives
only a server-created `FxEvidenceRequest` and owns a configured immutable set
of provider sources, a policy registry, signer, acquisition bounds, and signing
bounds.

For each request it:

1. resolves the exact policy digest and creates the async issuer from one
   registry snapshot before network work;
2. validates the currency pair, tenant-bound merchant/facts digests, source
   identities, and bounded topology through the existing acquisition boundary;
3. acquires candidates concurrently with per-provider and aggregate deadlines;
4. normalizes outcomes and selects exact agreeing candidates by policy order;
5. signs exactly one canonical evidence payload through the async signer;
6. returns only locally verified `IssuedFxEvidence`.

The coordinator never accepts provider endpoints, provider identities, signer
identity, policy JSON, legal authority, or FX evidence from a browser or quote
caller. It does not persist quotes, call the tax engine, submit Solana
transactions, retry signing, or decide legal coverage. Persistence and API
authorization remain application adapters; provider, legal, custody, regional
SLO, and production activation decisions remain in the decision register.

The policy and issuer are captured from the same immutable registry snapshot
before provider work. A reload during acquisition therefore cannot mix old
provider admission with a new signing policy.

## Alternatives considered

- Let the API call acquisition and issuance separately: rejected because a
  reload race could mix policy snapshots and duplicate orchestration logic.
- Put orchestration in the pure tax engine: rejected because network, clock,
  provider, and custody dependencies violate domain purity.
- Add a broker workflow: rejected for the quote path because it adds expiry,
  delivery, and signing ambiguity.
- **Server-owned bounded coordinator in `fx-provider`:** selected because it
  composes existing ports without creating a second settlement or tax
  authority.

## Verification

Tests must prove successful acquisition-to-issued-evidence flow, policy digest
binding, no source work on missing policy, snapshot isolation during reload,
provider agreement failures, signer timeout/failure, cancellation, bounded
source topology, and final envelope verification. API wiring must later add
authenticated tenant scope and durable evidence/outbox behavior without
changing this coordinator's authority boundary.
