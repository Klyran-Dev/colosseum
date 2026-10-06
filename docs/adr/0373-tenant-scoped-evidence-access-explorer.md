# ADR-0373: Tenant-scoped evidence-access explorer

- Status: Accepted
- Date: 2026-10-05
- Deciders: Principal Low-Latency Blockchain Architecture and Principal UI Architecture
- Scope: Evidence UI, API read model, PostgreSQL evidence-access audit ledger

## Context

The product already records append-only evidence-access audit rows when a
protected aggregate or document calculation is replayed. Operators can see
the result of an individual replay, but there is no dedicated, read-only
workspace for reviewing the history of sensitive evidence access. That leaves
support, security, and tax operators without a bounded way to answer who
accessed evidence, why, when, and whether the request was allowed.

The explorer must not become a generic database browser, expose decrypted tax
facts, or imply that an audit row is proof of settlement or legal compliance.
It must preserve tenant isolation, server-side authorization, UUIDv7 identity,
bounded query work, localized presentation, and explicit empty/stale/error
states.

## Decision

Add a read-only `Evidence & audit` workspace backed by
`GET /v1/audit/evidence-access`.

The API:

1. authenticates the principal and requires the `audit:read` scope;
2. derives the merchant boundary from the authenticated principal;
3. reads only `evidence_access_audit_events` for that merchant;
4. accepts an optional UUIDv7 calculation filter and a limit between 1 and
   100; and
5. returns typed audit metadata: UUIDv7 audit ID, calculation ID, actor
   subject, request ID, purpose, outcome, optional facts digest, and UTC
   occurrence time.

The API never returns encrypted or decrypted tax facts, raw request bodies,
wallet secrets, bearer credentials, or caller-supplied merchant identifiers.
The UI renders the result as a chronological, read-only evidence ledger and
keeps digest/request identifiers behind a technical disclosure. It explains
that the ledger records access activity only; finalized Solana evidence remains
the authority for settlement.

The first slice uses bounded offset-free “latest N” retrieval. It does not
invent a cursor contract or claim complete historical export. A future export
or cursor protocol requires a separate decision based on measured retention
and query plans.

## Alternatives considered

### Reuse the recovery workspace

Rejected. Replay gaps and sensitive evidence access have different operators,
authorization, retention, and failure semantics. Combining them would keep
the current navigation dense and make a recovery action appear equivalent to
an audit read.

### Expose a generic `audit_events` database browser

Rejected. The legacy table is not a tenant-scoped read model for all domains,
and a generic JSON payload browser would create an uncontrolled data-disclosure
and cardinality surface.

### Read audit rows directly from the browser

Rejected. Tenant scope, role checks, row limits, and redaction must be enforced
at the API boundary. The browser is a presentation client, never an audit
authority.

### Create a separate audit warehouse or event broker projection

Rejected for this slice. It adds asynchronous freshness and operational
coordination to a read path that already has a bounded append-only PostgreSQL
ledger. A projection can be reconsidered when retention or query-plan evidence
requires it.

### **Use a tenant-scoped, bounded PostgreSQL read model over the existing ledger**

Selected. It has the smallest authority surface, preserves append-only source
rows, gives support operators an immediately useful product surface, and keeps
the request to one indexed query with no settlement-path dependency.

## Invariants and budgets

- Every returned row belongs to the authenticated principal's merchant.
- The limit is validated before storage access and never exceeds 100.
- Results are ordered by `occurred_at DESC, id DESC` for deterministic ties.
- The endpoint is read-only and cannot mutate evidence or settlement state.
- Facts remain encrypted/off-path; only the existing digest commitment may be
  displayed to an authorized audit reader.
- The browser must represent unavailable, unauthorized, timeout, empty, and
  loaded states without fabricating rows or success.
- The local target is one bounded database round trip with a 900 ms statement
  timeout and no dependency on RPC, wallet, tax-provider, or message-bus work.

## Threat and privacy boundary

The endpoint is protected by OIDC-compatible authentication and the existing
`audit:read` scope. Merchant identity is never accepted from query parameters.
Actor subjects and request IDs are operational metadata; the UI presents them
only to the authorized audit workspace and keeps cryptographic digests in
progressive disclosure. Error responses use the shared operator-safe error
contract.

## Verification

- Write API authorization, tenant-isolation, limit, ordering, and digest
  serialization tests before implementation.
- Add a frontend parser test that rejects malformed or oversized responses.
- Add component coverage for empty, loaded, unauthorized, unavailable, and
  technical-disclosure states in English, German, Arabic RTL, and Japanese.
- Add real-browser navigation and axe coverage for the new workspace.
- Run the Rust, web, public-documentation, and integration gates.

## Consequences

The Records & issues group gains a focused audit surface without adding a
second authority or slowing financial operations. The workspace is deliberately
an access-history view, not a complete audit export or settlement evidence
viewer; those broader capabilities remain explicit future decisions.
