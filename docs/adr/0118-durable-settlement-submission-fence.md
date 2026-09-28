# ADR-0118: Durable settlement submission fence

- Status: Accepted
- Date: 2026-09-27
- Deciders: Principal Low-Latency Architecture and SRE
- Scope: settlement intent lifecycle, wallet-signed transaction submission

## Context

The settlement API persists a `signed` intent before calling an RPC provider.
Two concurrent requests carrying the same wallet-signed bytes can therefore
both observe `signed` and both call `sendTransaction`. Solana treats an exact
duplicate signature safely, but duplicate network work is still an avoidable
failure mode and a provider response cannot be trusted to identify a different
transaction. A process crash after provider acceptance can also leave the
database at `signed` without a durable chain identifier for reconciliation.

## Decision

Add a durable `submitting` lifecycle state and make submission a compare-and-
set operation:

1. Validate the exact signed message and blockhash before claiming submission.
2. Derive the transaction signature from the verified signed bytes.
3. Atomically transition `signed -> submitting`, persisting that signature.
   Only the request that wins this transition may call `sendTransaction`.
4. Require every provider response signature to equal the derived signature;
   a mismatch is provider evidence failure and never advances the intent.
5. On an acknowledged provider response, transition `submitting -> submitted`.
6. On an ambiguous provider failure, transition `submitting -> unknown` with a
   bounded error code and the same signature. The signed bytes are never stored.
7. Reconciliation observes the persisted signature for `submitting` or
   `unknown` intents and advances only through the monotonic lifecycle table.
   It never resubmits an ambiguous transaction automatically.

The `submitting` state is intentionally visible. A process crash leaves a
recoverable intent rather than silently reopening the send path. Reconciliation
or an operational worker must resolve it from finalized-provider evidence.

## Alternatives considered

### Database/advisory lock held across RPC

Rejected. It serializes requests but holds a database resource across an
unbounded external call, is difficult to recover after process loss, and adds
contention to the settlement critical path.

### In-process mutex or single-flight map

Rejected. It does not fence requests across replicas and loses ownership on
restart.

### Allow duplicate `sendTransaction` calls and rely on Solana deduplication

Rejected. It increases provider load, hides lifecycle races, and does not bind
the provider response to the exact verified signed message.

### **Durable compare-and-set lifecycle state**

Selected. It gives one submitting request ownership across replicas, keeps the
RPC call outside a database transaction, makes crashes observable, and uses a
native Solana signature for safe reconciliation without storing wallet-signed
bytes.

## Invariants

- At most one request can transition an intent from `signed` to `submitting`.
- A `submitting`, `submitted`, or later intent has a non-null signature bound
  to the verified signed transaction.
- The provider-returned signature must equal the locally derived signature.
- An ambiguous provider result never returns a successful settlement response.
- Reconciliation may advance state from persisted chain evidence but never
  resubmits an ambiguous transaction.
- Signed transaction bytes and private keys are never persisted or logged.
- A transition and its append-only lifecycle event remain one database
  transaction.

## Verification and release impact

- Coordinator tests derive a real signature and reject a mismatched provider
  response.
- Storage tests cover the expanded lifecycle transition table and bounded error
  codes.
- The Docker local-stack drill now exercises two concurrent API submissions
  against PostgreSQL 18 and a validating RPC fixture. The 2026-09-27 run
  returned exactly one `200` and one `409`, made exactly one fixture
  `sendTransaction` call, and read back one durable `submitted` intent with the
  native Solana signature. The drill is reproducible with
  `VERIFY_LOCAL_STACK_SKIP_BUILD=true ./scripts/verify_local_stack.sh`.
- This evidence does not prove crash recovery, provider timeout ambiguity,
  validator-backed settlement, multi-region provider failover, or production RPC
  failover. Those remain separate release gates.
- ADR-0134 adds the executable local two-replica submission drill; retained
  local evidence now includes a 2026-09-27 run with one `200`, one `409`, one
  validating RPC send, and one durable native signature. It still does not
  close live validator, multi-region, or production-capacity gates.
