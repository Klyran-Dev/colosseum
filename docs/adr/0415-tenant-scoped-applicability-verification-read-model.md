# ADR-0415: Tenant-scoped applicability verification read model

- Status: Accepted
- Date: 2026-10-06
- Owners: API and evidence UX
- Related: ADR-0403, ADR-0413, ADR-0414

## Context

The verifier worker produces operational evidence asynchronously. The product
needs to show that lifecycle without allowing a browser to infer finality from
local state or to query another merchant's settlement. The API already derives
tenant scope from the authenticated principal and uses audit-read authorization
for protected evidence.

An indexer stream is a projection selector, not an authorization boundary. A
caller may provide it to select the configured chain stream, but the server
must still resolve the settlement intent through the authenticated merchant
ID. The endpoint must also distinguish no applicability event from pending,
retrying, verified, and dead-lettered evidence.

## Decision

Expose:

`GET /v1/settlement/intents/{intent_id}/applicability-binding-verification?stream_key=<server-configured-stream>`

The handler authenticates `audit:read`, derives the merchant ID from the JWT,
and loads a bounded read projection by `(merchant_id, intent_id, stream_key)`.
It returns only redacted chain identities, canonical decimal slot strings,
status, bounded retry metadata, and immutable digest evidence. It never returns
raw tax facts, signed transactions, private keys, or a signing recommendation.

The status vocabulary is explicit:

- `not_applicable`: the settlement has no applicability event in this stream;
- `pending`: a raw event is queued but not currently leased;
- `retrying`: a worker currently owns a bounded lease;
- `verified`: the finalized account proof and typed projection are committed;
- `dead_lettered`: the retry budget was exhausted and operator review is
  required.

The browser treats every status other than `verified` as non-final evidence and
keeps settlement authority at the signed Solana message/program boundary.

## Alternatives considered

| Alternative | Tenant safety | Evidence clarity | Decision |
| --- | --- | --- | --- |
| Return worker tables directly | Fail; leaks coordination details | Partial | Rejected |
| Accept merchant ID from the query | Fail; caller becomes authority | Partial | Rejected |
| Return only a boolean | Partial | Fail; hides pending/dead-letter states | Rejected |
| Read by signature only | Partial; weak intent ownership | Partial | Rejected |
| **Read by authenticated merchant + settlement intent + stream selector** | **Pass** | **Pass; explicit lifecycle** | **Selected** |

## Consequences

The UI can explain evidence state without making a compliance claim, and
cross-tenant intent identifiers fail closed. The stream selector must be
provided by the configured UI/session context, and the API/UI contract must be
versioned if a deployment later supports multiple streams in one tenant.
