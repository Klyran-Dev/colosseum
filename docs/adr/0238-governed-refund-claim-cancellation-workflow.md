# ADR-0238: Governed refund-claim cancellation workflow

## Status

Accepted for implementation. This ADR adds the server-owned compiler and
signed-transaction verification boundary for cancellation; API persistence,
finalized cancellation reconciliation, bounded automatic worker convergence, and UI are implemented. Linux validator
evidence, provider-load evidence, and production governance-key operations
remain required release gates.

## Context

ADR-0154 defines a governance-only on-chain cancellation instruction for a
pending refund claim. The protocol already rejects a cancelled claim at
execution, and the local validator fixture covers the instruction. The
off-chain surface must not ask a governance wallet to hand-author accounts or
financial values, and it must not mark a claim cancelled when an RPC provider
has only acknowledged submission.

The current claim and execute workflows are server-compiled and persist
immutable unsigned artifacts. Cancellation needs the same authority boundary,
but it must not reuse a movement artifact: cancellation has no amount,
destination, token account, or tax calculation input. It binds only the
tenant-scoped claim identity, the server-derived merchant/mint registry, the
claim PDA, and the authenticated governance wallet.

## Decision

Add a dedicated `SettlementRefundCancellationCompileRequest`, immutable
compiled cancellation artifact, and exact signed-message verifier.

The compiler derives and emits exactly these five instruction accounts in this
order:

1. registered governance authority, signer and fee-payer-writable (the
   program never mutates this account; the writable bit is required by the
   wallet's fee-payer message);
2. merchant, read-only;
3. mint, read-only;
4. merchant/mint adjustment registry PDA, read-only; and
5. the adjustment/claim-nonce refund-claim PDA, writable.

Instruction data is the canonical Anchor discriminator
`global:cancel_adjustment_refund_claim` followed by the little-endian claim
nonce. The compiler accepts only the configured settlement program, a positive
claim nonce, canonical non-default public keys, and distinct instruction
accounts. It performs no RPC lookup and never accepts merchant-side addresses
from the browser.

The API/workflow layer will persist a separate UUIDv7 cancellation intent and
unsigned artifact before wallet signing. The intent is tenant-scoped and
idempotent by client key plus claim identity. Only an authenticated governance
principal with the configured cancellation scope may prepare or submit it. The
governance wallet is a tenant-scoped UUIDv7 authority binding; the server
additionally verifies that binding against the finalized on-chain adjustment
registry before producing the artifact.
Submission verifies the actual signed bytes against the exact artifact,
including program ID, instruction data, account order, signer/writable flags,
fee-payer separation, packet bound, and absence of address lookup tables or
extra instructions. Unknown provider outcomes remain unknown. Only finalized
`AdjustmentRefundClaimCancelled` evidence bound to the exact claim PDA,
signature, slot, and event fields may transition the cancellation intent and
the claim projection to `cancelled`.

The indexer owns a bounded automatic cancellation reconciler. It claims one
submitted cancellation at a time using a short PostgreSQL UUIDv7 lease with
`SKIP LOCKED`, recovers only the persisted governance authority and signature,
and fences the finalized cancellation transition with the lease token. A
stale lease is reclaimable after the bounded crash window; the worker never
holds a database lock while calling RPC and never advances the forward
checkpoint. The API reconciliation route remains an explicit operator path
and uses the same evidence boundary, but cannot steal an active worker lease.

The browser receives only a server-built artifact and may provide the selected
governance wallet plus signed bytes. It cannot provide a program ID, merchant,
mint, registry, claim PDA, nonce, amount, or destination as authority inputs.

## Invariants and failure policy

- A cancellation artifact contains no financial amount or token-account
  topology.
- The registry and claim PDA are deterministically derived from authenticated,
  server-owned identity and the claim's immutable nonce.
- Governance authority is distinct from the claim PDA and all other instruction
  accounts; all five instruction accounts are unique.
- Claim cancellation is legal only for a pending, not executed or already
  cancelled, claim intent. State admission is enforced by storage/API; the
  program remains the final authority.
- A malformed, substituted, stale, duplicate, oversized, unsigned, or
  provider-ambiguous cancellation cannot mutate off-chain financial state.
- Cancellation submission and finalized cancellation evidence are separate
  states; database status and wallet signature do not authorize token movement.

## Alternatives considered

See `docs/architecture/alternative-analysis.md`.

1. Mark the claim cancelled only in PostgreSQL: rejected because the on-chain
   claim would remain executable.
2. Let the browser construct the cancellation instruction: rejected because it
   permits account/PDA/nonce substitution and breaks the server-owned signing
   boundary.
3. Reuse the refund movement compiler: rejected because cancellation has a
   different account shape and must not carry token amounts or movement
   topology.
4. Let an API-held governance key submit cancellation: rejected because it
   violates non-custodial wallet boundaries and expands key compromise impact.
5. **Persist a server-owned cancellation artifact and require a governance
   wallet signature with finalized proof:** selected because it preserves the
   protocol authority, auditability, replay safety, and non-custodial model.

6. **Run automatic convergence in the indexer with a short PostgreSQL lease:**
   selected because cancellation finality is a projection concern, not a
   browser responsibility; durable leases coordinate replicas, stale-lease
   reclamation survives crashes, and token-fenced CAS prevents an old worker
   from changing state after ownership expires. Broker-owned queues and locks
   across RPC were rejected because they add another authority or couple
   database capacity to external latency.

## Latency and resource budget

The compiler is pure in-process work with a fixed five-account legacy
instruction and no RPC or database dependency. The diagnostic target is p50 <=
1 ms and p99 <= 5 ms warm. Signed-message verification remains bounded by the
1,232-byte transaction packet limit, one instruction, five movement accounts,
and no address lookup tables. API and finalized-reconciliation latency are
measured separately and must not be folded into the compiler budget.

## Verification plan

1. Add failing compiler tests for the exact discriminator, nonce encoding, PDA
   derivation, account order, signer/writable flags, default-key rejection,
   duplicate-account rejection, unsupported-program rejection, and wire bound.
2. Add failing coordinator tests for account substitution, data mutation,
   extra instructions, address lookup tables, fee-payer aliasing, and accepted
   canonical signed bytes.
3. Storage/API tests now cover tenant isolation, governance scope, idempotency,
   prepared-state fencing, signed-byte immutability, unknown provider outcome,
   typed finalized event matching, and the atomic claim/cancellation transition.
4. Browser parser and accessibility tests cover account substitution, wallet
   rejection, transport acknowledgement, finalized cancellation, and
   reconciliation-required states with no implied financial success.
5. Extend the authoritative Linux validator evidence to require the complete
   prepare/sign/submit/finalize cancellation lifecycle and unchanged balances.
6. Exercise two indexer replicas against an isolated PostgreSQL/RPC fixture to
   prove cancellation lease ownership, stale reclaim, finalized event binding,
   token-fenced convergence, bounded retries, and fixed-cardinality backlog
   metrics.

## Consequences

Cancellation becomes a first-class auditable workflow instead of an implicit
database flag. It adds an intent aggregate, governance authorization, typed
event decoding, atomic finalized reconciliation, and UI states, but keeps claim
movement immutable and prevents a cancelled claim from being mistaken for a
submitted or finalized refund. Production enablement still requires validator,
provider, load, security, legal/tax ownership, and operational evidence.
