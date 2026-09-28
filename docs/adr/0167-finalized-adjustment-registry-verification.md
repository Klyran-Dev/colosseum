# ADR-0167: Finalized server-owned adjustment-registry verification

## Status

Accepted for implementation.

## Context

ADR-0166 binds authenticated subjects to wallet public keys and creates a
retry-stable unsigned `record_adjustment` intent. The Solana program still
authorizes the instruction through the merchant/mint adjustment-registry PDA.
Compilation must not be enabled in a non-simulation environment until the
service proves that the registry is the canonical account for the configured
program, is observed at finalized commitment, is active, and contains the
same proposer and approver keys that were wallet-bound by the API.

Trusting a route row, client-supplied registry address, processed RPC response,
or a single unvalidated account payload would leave an authority-substitution
gap. A registry read is also evidence that must remain bound to the durable
compiler intent so retries cannot silently switch authorities after a registry
rotation.

## Decision

Add a server-owned verifier to the settlement RPC coordinator. For every
adjustment compile request it will:

1. derive the registry PDA from the fixed settlement program ID and the
   server-owned finalized receipt merchant and mint;
2. read the derived address with bounded `getAccountInfo` JSON-RPC requests at
   `finalized` commitment and base64 encoding;
3. require the settlement program owner, non-executable state, exact Anchor
   discriminator and exact bounded account layout;
4. require merchant, mint, proposer, approver, active flag, and PDA bump to
   match the server-owned request;
5. compare successful responses from all configured providers by canonical
   account bytes and fail closed on disagreement; and
6. return the finalized slot and a domain-separated account-data digest.

The API persists registry address, finalized slot, commitment, and account
digest in the immutable evidence intent. These fields participate in the
request digest and are immutable after creation. A retry whose registry state
has changed therefore fails with an intent conflict instead of compiling a
different authority snapshot. The verifier does not cache or promote a route;
the on-chain registry remains authoritative and the account read is only a
bounded admission/evidence step.

## Alternatives considered

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Trust the database route and wallet bindings | Fast and simple | Does not prove the active on-chain authority or program ownership | Rejected |
| Accept a registry address from the browser | Flexible client integration | Allows PDA/account substitution and breaks server ownership | Rejected |
| Read at `processed` or `confirmed` commitment | Lower latency | A reorg or stale provider response can authorize the wrong registry state | Rejected |
| Read once and cache indefinitely | Lowest steady-state RPC load | Rotation and revocation become invisible; cache becomes an authority | Rejected |
| **Derived finalized read with bounded provider agreement and intent snapshot** | Binds PDA, program, fields, commitment, provider evidence, and retry identity | Adds one bounded RPC admission step and evidence columns | **Selected** |

## Invariants and failure policy

- the registry address is derived from server-owned merchant/mint/program data;
- registry account owner and Anchor discriminator are exact;
- registry data length and field offsets are bounded and version-specific;
- active registry proposer and approver equal the active wallet bindings;
- proposer and approver remain distinct;
- finalized provider responses with different bytes fail closed;
- provider transport failure may fail over, but no response means no compile;
- registry evidence is immutable inside the UUIDv7 compiler intent; and
- this verifier authorizes evidence compilation only, never signing, token
  movement, posted state, or legal compliance.

## Verification

- pure parser tests cover wrong owner, discriminator, layout, PDA bump,
  merchant/mint/key substitution, inactive state, and provider disagreement;
- coordinator tests prove finalized/base64 request shape, bounded timeout, and
  failover behavior;
- storage tests and the PostgreSQL drill prove immutable registry evidence and
  changed-registry retry conflict;
- API boundary tests and coordinator error mapping prove non-simulation
  compilation requires the coordinator and fails closed for missing or
  conflicting registry evidence;
- the authenticated Docker integration proves wallet-bound proposer/approver
  binding, finalized receipt identity, finalized registry verification,
  immutable evidence-intent compilation, exact retry replay, and the
  process-crash reconciliation path; and
- the validator conformance suite must initialize, rotate, deactivate, and
  read the same registry PDA before production enablement.

## Consequences

The adjustment evidence compiler can be admitted in non-simulation mode with a
cryptographically and chain-bound authority snapshot. Compilation is slightly
more expensive than a warm database-only path, but the account read is bounded,
parallelizable with other independent evidence reads where safe, and never
placed in the token movement or wallet-signing path.
