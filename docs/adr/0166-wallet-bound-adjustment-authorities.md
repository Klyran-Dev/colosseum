# ADR-0166: Wallet-bound adjustment authorities and durable compiler intents

## Status

Accepted for implementation; ADR-0167 supplies the bounded finalized
on-chain registry verification. Validator execution, wallet submission, and
finalized reconciliation remain release gates.

## Context

The `record_adjustment` Solana instruction requires a registered proposer and
approver public key, while the HTTP adjustment workflow currently stores only
authenticated OIDC subjects. An API request containing arbitrary Solana public
keys would allow the service to compile artifacts that are not bound to the
approved actors. A subject string is not a Solana signature, and a compiled
instruction is not evidence that a key is active in the on-chain registry.

Compilation also needs durable retry semantics. A new adjustment PDA nonce on
each retry would create multiple possible evidence records and make a timeout
unsafe to recover.

## Decision

Create a wallet-authority bounded context with:

1. short-lived, tenant- and subject-scoped one-time challenges;
2. Ed25519 proofs over a canonical versioned challenge message;
3. immutable active bindings between an authenticated subject, one role
   (`proposer` or `approver`), and one Solana public key;
4. atomic rotation that revokes the prior active binding before inserting a
   newly proven binding; and
5. a durable, UUIDv7 adjustment-evidence intent that allocates one database
   nonce and snapshots the receipt identity, authority keys, token program,
   and request digest before returning a compiler artifact.

The API exposes challenge and binding endpoints only to the corresponding
authenticated adjustment role. A compile request requires an approved
adjustment, the recorded approver subject, a complete finalized receipt
binding from ADR-0165, active wallet proofs for both adjustment subjects, and
an active verified settlement route matching the receipt merchant and mint.
The browser cannot provide or override the original receipt, merchant, mint,
token program, authority keys, deltas, or nonce.

The endpoint returns `prepared_unsubmitted` evidence only. It does not sign,
submit, mark the adjustment posted, or claim token movement. ADR-0167 verifies
the active finalized on-chain adjustment registry before this evidence is
compiled; a later release must add the wallet-signed submission/finality state
machine.

## Alternatives considered

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Trust OIDC subject strings as Solana keys | No extra flow | Subjects are not cryptographic wallet proofs and may not be public keys | Rejected |
| Accept proposer/approver public keys on compile requests | Easy integration | Caller-controlled authority substitution and no durable proof | Rejected |
| Configure one server-held key pair | Low request latency | Custody, four-eyes bypass, and no user-wallet accountability | Rejected |
| **Challenge/proof wallet binding plus durable evidence intent** | Non-custodial, replay-safe, auditable, and compatible with wallet signing | Adds a challenge UX and requires on-chain registry verification before production | **Selected** |

## Invariants and failure policy

- challenge messages bind tenant, subject, role, UUIDv7 challenge, and expiry;
- a challenge is single-use and expires server-side;
- an active Solana public key cannot be bound to two tenant subjects or roles;
- proposer and approver public keys must differ;
- binding rotation is append-only evidence with explicit revocation;
- approved adjustment fields and finalized receipt identity are snapshotted
  into one durable compiler intent;
- identical retries return the same intent and nonce; different inputs fail
  with an idempotency conflict;
- incomplete receipt, route, authority, calculation, or on-chain-registry
  evidence fails closed; and
- no private key, seed phrase, bearer token, or raw tax evidence enters the
  API or database.

## Verification

1. Add domain vectors for canonical challenge messages and Ed25519 proof
   rejection (wrong subject, role, tenant, expiry, and replay).
2. Add PostgreSQL integration coverage for one-time challenge consumption,
   rotation, key uniqueness, durable nonce allocation, and retry convergence.
3. Add API authorization tests for exact scope/role and approved-subject
   binding.
4. Add compiler tests proving all account fields originate from server-owned
   records and that changed input cannot reuse an intent.
5. Verify active proposer/approver keys against the finalized on-chain
   adjustment registry before enabling non-simulation compilation or wallet
   submission.

The executable database contract drill is `make adjustment-authority-drill`.
It validates active-key uniqueness, proof-field immutability, explicit
revocation, and one-time instruction-digest binding against PostgreSQL 18.

## Consequences

The production path gains cryptographic actor binding and deterministic retry
behavior, at the cost of a wallet-proof ceremony and new durable evidence.
The compiler remains low-latency after warm reads, while correctness depends
on the finalized receipt projection, verified route, authority bindings, and
eventual registry read remaining available.
