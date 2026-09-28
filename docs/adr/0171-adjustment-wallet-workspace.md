# ADR-0171: Wallet-bound adjustment browser workflow

- Status: Accepted — implementation in progress
- Date: 2026-09-28
- Owners: Web Platform, Settlement, Tax Platform

## Context

ADR-0170 defines a durable chain-intent lifecycle for an approved adjustment.
The API and coordinator verify the wallet-signed `record_adjustment` message,
but the browser workspace currently stops at governance evidence. Operators
need a production workflow that can compile the server-owned artifact, show
the exact authority boundary, invoke the connected wallet, submit unchanged
signed bytes, and recover from ambiguous RPC or indexer lag without claiming a
financial outcome.

## Decision

Extend the existing lazy-loaded adjustment workspace with a wallet-bound
recording workflow:

1. An approved adjustment can request a server-owned compile. The browser
   parses the bounded response and displays registry evidence, instruction
   identity, account count, and chain-intent ID.
2. The wallet is connected only when the operator explicitly starts signing.
   The browser requires the connected public key to equal the compiled
   approver account and serializes exactly one record-only instruction. It
   does not add compute-budget instructions, mutate account metas, or calculate
   financial values.
3. The API receives the unchanged compiled instruction and canonical signed
   bytes through the ADR-0170 endpoint. The browser displays durable status,
   wallet rejection, timeout, unknown outcome, provider disagreement, and
   indexer-lag states as distinct states.
4. Reconciliation is an explicit operator action. Only the API's finalized
   typed event match can render `finalized`; this means finalized recording
   evidence, never payment, filing, or legal compliance.

The browser keeps all state disposable. It does not persist signed bytes,
private keys, access tokens, or financial authority in local storage. Existing
checkout wallet adapters are reused so wallet discovery, signing, and bounded
browser deadlines have one implementation.

## Alternatives considered

| Option | Decision | Reason |
| --- | --- | --- |
| Add a second wallet adapter inside the adjustment panel | Rejected | Duplicates wallet security, serialization, timeout, and rejection behavior. |
| Let the browser build or alter the record instruction | Rejected | Creates an account/data substitution surface and diverges from the server compiler. |
| Mark the adjustment recorded after wallet signing or RPC acknowledgement | Rejected | Neither proves finalized event emission or exact intent binding. |
| Persist signed bytes in browser storage for retry | Rejected | Increases exposure of sensitive transaction material and survives operator context changes. |
| **Reuse the shared injected-wallet boundary with server-owned compile, exact signed submission, and explicit reconciliation** | **Selected** | Keeps one wallet trust boundary while preserving immutable server authority and supportable failure states. |

## Invariants and UX contract

- The compile response must contain UUIDv7 IDs, 32-byte digests, a prepared
  lifecycle, the fixed Solana program, exactly ten bounded account metas, and
  canonical instruction data.
- The connected wallet must be the first, signer/writable approver account.
- No UI state says “paid,” “refunded,” “compliant,” or “tax filed.”
- A timeout after submission never enables a second signature; the UI loads
  the durable chain intent or shows an explicit unknown state.
- `finalized` is shown only after the server returns it from finalized event
  reconciliation; the browser never infers it from a status callback.
- Every state is keyboard accessible, announced with semantic status/error
  regions, and usable at narrow widths and RTL direction.

## Verification and remaining gates

Failing-first tests cover bounded compile parsing, account substitution,
wallet mismatch, canonical signing handoff, API request shape, timeout and
unknown recovery, reconciliation-required messaging, accessibility, and the
no-payment language contract. Full browser wallet-provider and pinned Linux
validator evidence remain release gates.
