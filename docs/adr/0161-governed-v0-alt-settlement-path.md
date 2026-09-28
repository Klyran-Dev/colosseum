# ADR-0161: Governed v0+ALT settlement admission

- Status: Accepted; implementation boundary complete
- Date: 2026-09-28
- Deciders: Principal Low-Latency Architecture, Settlement, and SRE
- Scope: wallet-signed Solana v0 transaction verification

## Context

The settlement compiler and coordinator currently support a bounded legacy
transaction shape and reject address-table lookups. Legacy messages are the
safe default, but split fan-out and future account growth require a governed
v0 path. Accepting a v0 message by checking only static keys is unsafe: the
instruction can reference dynamically loaded writable accounts, an untrusted
lookup-table account, an out-of-range index, or an extra account that is not
part of the compiled settlement.

Address Lookup Tables (ALTs) are chain state, not client-provided metadata.
The API must resolve them from the configured RPC provider set, apply a
bounded finalized/commitment policy, and compare the expanded account domain
with the server-created compiled instruction before submission.

## Decision

Add a bounded v0 admission path with these rules:

1. Legacy messages continue through the existing synchronous verifier.
2. v0 messages are accepted only through the async server-owned lookup
   resolver. The existing synchronous method fails closed with an explicit
   resolution-required error when a v0 lookup is present.
3. The resolver accepts at most two lookup tables and at most 32 loaded
   addresses per table for the current settlement shape. Each table is loaded
   from a configured RPC provider with `getAddressLookupTable`-equivalent
   finalized semantics, bounded timeout, provider failover, and no client
   supplied table contents.
4. Every lookup table key is unique; every requested index is in range; loaded
   writable/readonly ordering is preserved; duplicate expanded addresses and
   unused expanded addresses are rejected.
5. The expanded static plus loaded key domain must contain exactly the
   compiled program and compiled settlement accounts. Instruction data,
   program identity, payer signature, signer flags, writable flags, and
   conservative serialized-size budget remain mandatory.
6. Only v0 is supported by this path. Unknown/future message versions and
   v1 are rejected until separately governed.

The verifier does not trust a client-provided ALT digest or account list. The
compiled settlement remains the authority for expected accounts; the chain
lookup response is only evidence used to resolve the signed message.

## Alternatives considered

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Keep rejecting v0+ALT permanently | Smallest attack surface | Prevents bounded account growth and leaves the production transaction path incomplete | Rejected |
| Trust client-provided table contents/digests | Low latency and no RPC read | Client can substitute addresses or privileges; not an authority boundary | Rejected |
| Accept any v0 table returned by any RPC | Broad wallet compatibility | No table allow-list/shape bound, provider disagreement, and extra-account risk | Rejected |
| **Server-owned bounded finalized lookup resolution and exact expanded-domain verification** | Preserves wallet compatibility while retaining authority, failover, size, and privilege controls | Adds bounded RPC reads before submission and a new contract to operate | **Selected** |

## Invariants and budgets

- No v0 transaction is submitted before all referenced tables are resolved and
  verified against the compiled settlement.
- At most two table reads, 32 loaded addresses per table, and one bounded
  resolver attempt per configured provider are performed for one admission.
- A provider timeout, malformed response, table disagreement, stale lookup,
  duplicate address, out-of-range index, or unexpected account fails closed.
- Lookup resolution is outside the deterministic tax engine and never changes
  the signed transaction bytes.
- The serialized transaction remains below the existing 976-byte settlement
  admission ceiling; target-cluster compute-unit and packet evidence remain
  separate release gates.

## Verification and rollout

- Unit-test legacy compatibility, valid v0 expansion, writable/readonly
  ordering, duplicate/out-of-range/extra-account rejection, and resolver
  timeout/failover behavior.
- The validator-backed TypeScript conformance suite creates and extends a real
  ALT, retrieves it at finalized commitment, compiles and signs a v0 message,
  proves the submitted transaction contains a lookup, and asserts finalized
  100/25/75 token conservation. A separate Docker RPC fixture remains useful
  for API-level resolver integration tests.
- Add Linux validator-backed v0+ALT conformance before marking the production
  readiness checkbox complete. Local unit/fixture evidence does not substitute
  for target-cluster execution.
- Keep v1 and dynamic table governance open until their own ADR and evidence
  exist.
