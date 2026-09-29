# Database

The durable store is PostgreSQL 18.x. The migrations use native `uuid` storage
and `uuidv7()` defaults for internal identifiers. Historical evidence is
append-only; corrections create new rows and preserve the original calculation
or settlement record.

The persisted quote path uses a serializable transaction to select an exact
published rule release, insert calculation evidence, and complete a
merchant-scoped idempotency record together. The database is an evidence and
projection store; finalized Solana receipts remain authoritative for funds.
Atomic amounts use exact `numeric(20,0)` columns rather than signed `bigint`,
so the full Solana `u64` range is representable without narrowing at the
database boundary.

New quote rows persist normalized replay facts as application-encrypted
`facts_ciphertext` with a 96-bit nonce and versioned `facts_key_id`; plaintext
`facts_document` is reserved for legacy rows during the migration window. The
API requires an evidence-key capability before enabling durable quote storage.
Raw jurisdiction evidence is verified in memory and is not persisted.
The storage adapter resolves active and previous key IDs through an injected,
bounded provider; `API_EVIDENCE_KEY_RING_JSON` is a synthetic local rotation
fixture only. Production key material must come from an approved KMS/HSM
adapter and never from PostgreSQL or request data.

Compose mounts one clearly named synthetic rule fixture for local development
only. It is not a supported jurisdiction, legal interpretation, or production
seed and must never be copied into a release migration.

The generated sortable-ID inventory is executable in
`db/verification/uuidv7_contract.sql`. Clean-stack integration and the
PostgreSQL logical restore drill run it against the final catalog, checking
native `uuid` columns, `uuidv7()` defaults, and version-7 constraints. It
deliberately excludes CSPRNG replay claim tokens, external identity claims,
Solana identifiers, and cryptographic digests.

Migration `0004` enforces immutable published and retired rule releases and
rejects overlapping published effective windows for the same rule, jurisdiction,
and regime.

Migration `0005` adds bounded outbox delivery leases, retry/dead-letter state,
and the dispatcher index. JetStream acknowledgements never replace the
PostgreSQL event ledger.

Migration `0007` adds bounded, immutable raw Anchor settlement-event evidence
beside normalized finalized receipts. Evidence is retained for deterministic
projection rebuilds and forensic replay; it is not a substitute for the
authoritative Solana ledger.

Migration `0008` adds UUIDv7 replay-gap work rows with bounded attempts,
lease-fenced claims, dead-letter state, and operator-visible failure metadata.
The indexer service consumes these rows in bounded chunks. Replay writes
historical receipt/evidence projections idempotently and advances only the
lease-fenced suffix; it never advances `chain_checkpoints`.
The `make indexer-up` target reapplies this additive migration before starting
the indexer so an existing local PostgreSQL volume cannot silently omit the
replay ledger.

Migration `0009` adds bounded, append-only full transaction payload evidence
with a domain-separated digest and transaction-position identity. The payload
is retained for projection rebuild and forensic inspection; normalized
receipts and decoded event evidence remain separately validated.

Migration `0010` adds UUIDv7 settlement intents and append-only lifecycle
events. It stores request, instruction, and signed-transaction digests, never
private keys or signed bytes, and supports expected-state fencing for
concurrent transitions. A provider timeout is represented as `unknown` until
signature reconciliation supplies chain evidence.

Migration `0027` adds the durable `submitting` state. A verified wallet-signed
transaction must win a `signed` to `submitting` compare-and-set before the API
calls an RPC provider; the native transaction signature is persisted for crash
reconciliation, and ambiguous submission never reopens the send path. The
PostgreSQL contention drill proves that two concurrent claims produce exactly
one winner and one loser. New chain-status rows require a bound signature and
new `unknown` rows require a bounded error code; the two compatibility checks
are initially `NOT VALID` so existing installations can backfill and validate
them under a controlled expand/contract migration.

Migration `0028` adds the append-only targeted-recovery audit ledger. Each
operator attempt records a UUIDv7 request/audit identity, server-selected
stream, target address and signature, bounded operator note, terminal outcome,
and bounded recovery evidence. `requested`, `recovered`, and `failed` rows are
immutable; a requested row without a terminal row is an intentional crash or
timeout signal for reconciliation and must not be interpreted as successful
repair.

Migration `0029` adds immutable, route-bound vault-reconciliation targets and
append-only finalized observation runs. A target cannot exist without an
active settlement-route identity and an operator-approved opening-balance
reference. The reconciler derives expected balance from exact finalized
receipt/release deltas and compares it with a finalized Solana token-account
observation; PostgreSQL remains a projection and evidence store.

Migration `0030` adds the append-only `protocol_pause_events` projection for
finalized on-chain `ProtocolPauseChanged` governance evidence. It is mounted
in the clean Compose bootstrap and applied by `make migrate`/`make indexer-up`;
the local integration contract fails if the table is absent or its update/delete
guards are missing.

The disposable integration harness also runs
`db/verification/targeted_recovery_audit.sql`, which proves stale-request
counting, terminal recovered/failed pairing, and rejection of UPDATE/DELETE
mutation attempts.

It also runs `db/verification/vault_reconciliation.sql`, which proves the
opening-balance plus receipt-tax minus executed-release expectation, latest-run
mismatch visibility, and append-only run enforcement.

The clean-stack integration additionally runs
`db/verification/refund_claim_reconciliation.sql` to verify the UUIDv7 lease
columns, pair constraint, and partial candidate index used by automatic claim
finality workers.

Migration `0011` binds calculations to the authenticated `merchant_id` and
requires a non-null expiration for settlement authorization. New quotes use a
15-minute database-owned lifetime. The migration intentionally fails when
legacy orphan calculations exist; operators must perform an approved,
evidence-backed tenant backfill before retrying it. Settlement compilation
loads quotes by both merchant and calculation ID and never accepts browser-
supplied financial amounts.

Migration `0012` adds the tenant-scoped settlement route registry. Active route
account fields are immutable and corrections require a new version. The API
resolves merchant, mint, vault, destination, and token-program values by
authenticated merchant plus route ID; it derives the payer source associated
token account locally and never accepts those account fields from the browser.
Only a separately authorized chain-verification workflow may promote a route
to `active`. The synthetic route fixture is local-only.

Migration `0013` adds append-only finalized route-verification evidence and
verification coordinates to the route lifecycle. Production activation must
use `finalized` evidence; the explicitly marked `local_synthetic` mode exists
only for the Docker fixture and cannot be produced by the route-verifier
service.

Migration `0014` adds the bounded normalized `facts_document` used for
authoritative tax replay. It is nullable only to preserve legacy rows that
were created before replay inputs were persisted; those rows fail closed with
`QUOTE_REPLAY_UNAVAILABLE` until an approved evidence-backed backfill exists.
Migration `0025` adds the expand/contract encrypted-facts columns. New quote
writes persist authenticated ciphertext, a per-record nonce, and a key ID in
one transaction with the result, digests, and outbox event; plaintext
`facts_document` is legacy-only. Migration `0026` adds the append-only
evidence-access audit ledger, which stores identity, purpose, outcome, request
correlation, and digests but never decrypted facts.

Migration `0015` adds the authenticated tax-rule governance lifecycle. It
stores distinct author, reviewer, and approver subjects plus a UUIDv7
governance revision, enforces the promotion state machine with a database
trigger, and appends immutable bounded `rule_governance_events`. Published and
retired governance assignments cannot be changed. The API performs the
identity scope/role checks and serializable revision fencing; the database
remains the durable state and evidence authority for this control-plane
workflow.

Migration `0016` makes rule releases tenant-owned. Quote resolution and
governance reads/transitions require the authenticated merchant, the unique
key is `(merchant_id, external_id, version)`, and calculations have a
composite foreign key to their rule owner. It deliberately fails when legacy
rule rows have no explicit owner; operators must perform an audited backfill
before retrying the migration. The synthetic fixture uses the documented
synthetic merchant only.

Migration `0017` adds the system-level indexer-operations boundary. Dead-letter
replay gaps can be acknowledged only through the authenticated operator API,
and requeue requires that acknowledgement. Bounded operator notes and subjects
are stored on the current gap for read efficiency and in the append-only
`replay_gap_operator_actions` ledger for audit evidence. Requeue clears the
current acknowledgement so a later retry exhaustion requires fresh review;
neither action advances a chain checkpoint or represents finalized settlement.

Migration `0018` adds the tenant-scoped immutable Split Plan registry. Active
plans require bounded recipient JSON, canonical basis-point conservation, and
activation evidence. The storage adapter recomputes the canonical digest before
quote-bound compilation; the local synthetic plan fixture is explicitly not
chain or production configuration.

Migration `0019` adds the finalized typed `SplitSettlementExecuted` projection
beside immutable raw event evidence. It preserves split-plan version and digest
binding for bounded operational reads and replay; it does not infer recipient
balances or replace finalized Solana state.

Migration `0021` adds immutable, tenant-scoped calculation-attestation
evidence. The quote-bound API writes the signed document, promotes the
calculation to `attested`, and appends its first outbox event in one
serializable transaction. Replays must match the complete signed document;
issuer keys remain outside the API and PostgreSQL stores only public evidence.
`make integration-check` exercises this path with a synthetic issuer process
in a separate non-root container; its ephemeral test key is never stored in
the repository or API image.

Migration `0022` adds immutable, tenant-scoped `jurisdiction_policies`. A
published policy is canonically digested, bound to one published rule release,
and selected only within its effective window. The quote adapter verifies the
policy digest before using it; published or retired policies cannot be mutated.
Provider-signed jurisdiction evidence is verified by the API and is never
stored as raw sensitive evidence in PostgreSQL or written to Solana.
The same migration adds `jurisdiction_policy_events` and database transition
guards. Policy drafts and transitions are created through the authenticated API
with distinct author/reviewer/approver subjects; creation and transition
evidence plus the first outbox publication record are committed transactionally.
Existing installations receive governance columns through expand/contract-safe
`ADD COLUMN IF NOT EXISTS` statements without synthetic actor backfills.

Migration `0023` adds tenant-scoped vault-release claim commands and immutable
claim-transition evidence. Migration `0024` tightens the state invariants:
financial fields, destination, nonce, bounded execution window, and a bound
claim PDA remain immutable once approved or finalized, while chain evidence may
advance only through the audited transition path. The database records intent
and evidence only; the Solana `ReleaseClaim` PDA and finalized token balances
remain authoritative.
Claim IDs and event IDs use UUIDv7, while Solana addresses, nonces, and
transaction signatures retain their native representations.

Migration `0032` adds the finalized typed `AdjustmentRefundExecuted`
projection. It stores the fixed event identity, claim/adjustment/receipt
bindings, merchant and mint, exact refund magnitudes, funder, destination,
approver, reference digest, and raw-evidence digest with stream/signature/event
idempotency and claim uniqueness. It is an operational evidence projection,
not a balance ledger or paid-state authority; independent finalized token
account reconciliation remains required.

Migration `0033` adds immutable transaction-bound refund balance proofs. Each
row is foreign-keyed to one indexed finalized refund event and stores the
transaction slot, claim/account topology, exact pre/post atomic balances, and
the canonical proof digest. PostgreSQL enforces conservation and append-only
mutation guards. The table is evidence only; it does not create `posted`,
`chain_executed`, or `paid` authority.

Migration `0034` adds the server-owned receipt PDA, payer, merchant, mint,
nonce, and policy-digest identity fields to `indexed_receipts`. It is an
expand migration: legacy rows remain nullable until the retained raw-event
projection rebuild backfills them and a later validation migration makes the
completeness constraint strict. New indexer writes are complete, and readers
fail closed on incomplete legacy identity rather than accepting caller data.

Migration `0035` adds partial/indexed lookup paths for the tenant-scoped,
finalized settlement-intent to receipt-projection binding described by
ADR-0165. It adds no mutable receipt authority; the adjustment compiler now
consumes this binding together with ADR-0167 registry evidence.

Migration `0036` adds one-time adjustment-authority challenges, immutable
wallet-proof bindings, a bounded adjustment nonce sequence, and durable
record-adjustment evidence intents. Each intent snapshots the derived
finalized registry address, slot, commitment, and account digest. The intent is
an unsigned compiler artifact; it is not a chain submission or funds ledger.

Migration `0037` adds the finalized typed `SettlementAdjustmentRecorded`
projection while retaining raw Anchor bytes as authoritative evidence.
Migration `0038` adds the separate UUIDv7 wallet-submission/reconciliation
lifecycle for those immutable evidence intents. Chain-intent status is
transport state only; it cannot replace finalized event matching.
Migration `0039` expands the typed projection with the on-chain adjustment
evidence digest; legacy rows remain incomplete until raw-event rebuild and are
rejected by reconciliation rather than silently upgraded.

Migration `0040` adds the tenant-scoped, versioned refund funding
configuration boundary. It records the explicit tax-vault, vault-authority,
merchant-funder, net-source, mint, and token-program topology with UUIDv7
identity, idempotent creation, four-eyes approval, append-only lifecycle
events, and immutable account fields. A configuration is never active without
separate finalized Solana topology verification; this table cannot authorize
token movement by itself.

Migration `0041` adds immutable finalized verification slot, commitment,
bounded evidence, verifier version, and verification timestamp fields. Only
the server-owned topology verifier may populate them while moving a reviewed
configuration through `pending_verification` to `active`.

Migration `0042` adds the explicit refund-funder subject and the durable Refund
Movement bounded context: separate claim and execute intents, per-adjustment
nonce allocation, immutable server-owned wallet artifacts, bounded lifecycle
state, append-only lifecycle events, and tenant-scoped foreign keys. These
rows are transport/reconciliation state only; a refund is not successful until
finalized transaction-bound event and balance evidence is verified.

Migration `0043` hardens the database guards so all claim and execute
financial, signer, topology, receipt, nonce, and unsigned-artifact fields are
immutable after creation. It also prevents reviewed funding configurations
from advancing without the explicit refund-funder subject.

Migration `0044` extends the cryptographic wallet-authority role set with
`refund_funder`. Funding activation now requires an active binding for the
configured subject whose public key exactly equals the verified refund-funder
system account.

Migration `0045` aligns the claim and execute lifecycle triggers with bounded
unknown-state recovery. An ambiguous provider outcome may converge only to a
later chain-evidence state; it cannot be rewritten as success without the
finalized transaction-bound event and balance proof.

Migration `0046` adds a short UUIDv7 reconciliation lease to refund claim
intents. Automatic finality workers claim candidates with `FOR UPDATE SKIP
LOCKED`, release the lease during external RPC recovery, and reclaim stale
leases after the bounded crash window. Lease ownership is coordination only;
the final claim transition remains a signature/event-bound compare-and-set.

Migration `0047` adds the equivalent UUIDv7 lease fence to refund-funding
topology verification. Route-verifier replicas claim approved or stale
pending configurations before external finalized RPC work; activation requires
the exact claim token and clears it atomically with immutable evidence. A
provider failure releases only the matching token, so a stale worker cannot
erase a newer verifier's lease.

Migration requirements:

- run migrations through a reviewed migration runner, never by editing production tables manually;
- use expand/contract changes for online deployments;
- set bounded `lock_timeout`, `statement_timeout`, and `idle_in_transaction_session_timeout`;
- use concurrent indexes where supported and justified;
- verify backups and restore into an isolated environment;
- run `make backup-restore-drill` to verify the Docker-isolated logical backup
  and restore path from a clean migration/fixture source; production additionally requires encrypted off-site
  backups, point-in-time recovery, and measured RPO/RTO evidence;
- run `make postgres-pitr-drill` to verify the Docker-isolated PostgreSQL 18
  physical base-backup and exact-LSN WAL replay boundary; production still
  requires encrypted off-site archive storage, retention/immutability,
  monitoring, measured RPO/RTO, and restore authorization;
- measure query plans and index health under representative evidence volume;
  `make postgres-index-plan-drill` provides a bounded PostgreSQL 18 planner
  gate for the declared evidence predicates, but does not replace
  production-volume measurement;
- never treat PostgreSQL rows as the source of truth for settled funds; chain receipts remain authoritative.
