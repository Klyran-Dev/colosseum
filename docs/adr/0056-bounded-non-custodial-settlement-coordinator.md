# ADR-0056: Bounded, non-custodial settlement transaction coordinator

- Status: Accepted
- Date: 2026-09-26
- Decision owners: Principal low-latency architecture and settlement domain
- Scope: `crates/settlement-coordinator` and the settlement submission boundary

## Context

ADR-0055 produces a deterministic instruction, but an instruction is not a
settlement. A production flow needs a safe boundary between wallet signing,
RPC submission, duplicate delivery, and observed commitment. That boundary
must not receive seed phrases or private keys, must not mutate a signed
transaction while retrying, and must not claim confirmation from an HTTP 200
or a local optimistic state.

Solana transaction bytes are bounded by the network packet limit. Submission
is also latency-sensitive and provider-dependent, so an unbounded queue,
unbounded retry loop, or synchronous database transaction around an RPC call
would create tail-latency and correctness failures.

## Decision

Create a settlement-coordinator bounded context with these rules:

- The client supplies a canonical base64 encoding of an already wallet-signed
  transaction. The service never accepts or stores a private key.
- The coordinator decodes and re-encodes base64 before submission, rejects
  empty/non-canonical input, and rejects bytes above the Solana packet budget
  (`1232` bytes).
- The signed bytes are immutable for every provider attempt. Retries may
  repeat the same transaction signature; they may not rebuild, resign, or
  alter blockhash, instructions, account metas, or compute-budget settings.
- The coordinator uses the existing bounded `indexer::JsonRpcTransport` port,
  preserving HTTPS requirements, response limits, request IDs, and provider
  transport policy. It uses at most three providers and a caller-supplied
  attempt deadline.
- `sendTransaction` is called with base64 encoding, preflight enabled, and
  `maxRetries: 0`; application-level bounded failover owns retry policy.
- A successful provider result means only `submitted`; it does not mean
  processed, confirmed, or finalized. Status is read through
  `getSignatureStatuses` and maps provider evidence to
  `submitted|processed|confirmed|finalized|rejected|unknown`.
- A transport failure after an accepted submission is reported as `unknown`,
  not as a safe-to-retry-new-transaction signal. The caller must reconcile by
  signature before any further action.
- Durable intent, idempotency fencing, and append-only lifecycle events belong
  in the storage adapter and are required before exposing a production submit
  endpoint. This crate remains network/domain coordination and does not make
  an in-memory status authoritative.

## Alternatives considered

1. **Custodial signing in the API:** rejected; it expands key-compromise scope
   and violates the non-custodial protocol boundary.
2. **Browser-only RPC submission:** rejected as the sole path; provider
   selection, bounded failover, evidence capture, and operational SLOs would
   be outside the platform boundary. Wallet signing remains client-side, but
   submission and observation have a server-side auditable port.
3. **Rebuild a transaction for every retry:** rejected; a fresh blockhash or
   altered message changes the signed intent and can create duplicate or
   mismatched settlement evidence.
4. **Reuse the indexer JSON-RPC transport:** accepted over a second HTTP
   implementation because it already centralizes TLS, timeout, response-size,
   request-envelope, and local-development rules. The coordinator depends on
   the infrastructure port, not on indexer projection state.
5. **Treat an RPC HTTP success as confirmation:** rejected; commitment is a
   chain observation and must be read separately.

## Release evidence

- Unit tests cover canonical base64, packet-size bounds, immutable retry bytes,
  JSON-RPC request shape, malformed provider responses, commitment mapping,
  rejected transactions, and unknown-after-acceptance semantics.
- A later storage/API ADR must add UUIDv7 durable intents, idempotency fencing,
  signature reconciliation, and lifecycle event retention before production
  submission is enabled.
- Localnet transaction tests must cover legacy SPL Token and Token-2022,
  stale blockhash, duplicate submission, pause state, and finality. The local
  validator profile currently requires a host with `io_uring` support.
