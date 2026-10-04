# ADR-0295: Server-owned reference selection for revenue plans

## Status

Accepted — implementation complete for the bounded catalog and proposal
contract. Production movement remains gated independently.

## Context

The partner-payout form currently asks an operator to paste a split-plan
fingerprint and a tax-policy fingerprint. That is technically precise but is
not a safe or supportable production workflow: operators can select a digest
from the wrong asset or policy, and the browser has no authoritative way to
explain whether the referenced version is published, effective, and tenant
owned.

The browser also currently supplies the recipient owner and asset even though
the server already owns a finalized-verified payout-destination registry.
Those values are public keys rather than secrets, but accepting them as the
plan authority creates unnecessary substitution surface and makes the UI
look like an account-topology editor.

## Decision

Add a tenant-scoped, authenticated reference catalog for the revenue-plan
workspace. It returns only bounded, currently selectable records:

- active finalized-verified payout destinations, identified by an opaque
  destination-route UUIDv7 and described by partner owner, asset, and route
  label;
- active, effective, integrity-validated settlement split plans, identified
  by their UUIDv7 and described by plan key, version, and asset;
- published, effective, integrity-validated jurisdiction policies, identified
  by their UUIDv7 and described by jurisdiction, regime, and effective window.

The create-plan request selects `destination_route_id`, `split_plan_id`, and
`policy_id`. The API resolves all three records under the authenticated
merchant boundary in one bounded request, verifies that the destination and
split plan use the same asset, and derives recipient, plan digest, policy
digest, and route bindings from those records. The browser never submits raw
digests, destination token accounts, token programs, or merchant-side route
topology.

The stored revenue plan continues to persist immutable digests and route
identities for replay and evidence. This is a presentation and input-boundary
change, not a weakening of the cryptographic binding. Existing persisted plans
remain readable; legacy clients that submit raw digest/topology fields are
rejected with an actionable contract error rather than silently accepted.

## Alternatives considered

### Keep raw digest and public-key fields in the form

Rejected. It preserves technical input errors, creates account and digest
substitution surface, and makes the operator responsible for joining several
authoritative registries manually.

### Let the browser list records and send their digests

Rejected. A digest copied from a list is still caller-controlled input and the
API would need to repeat the same resolution and integrity checks. It also
does not give the UI a stable route identity for an auditable selection.

### Add a generic search service or client-side cache

Rejected for the critical create path. It adds an operational dependency,
stale-selection behavior, and a second authorization boundary without
improving authority. A bounded authenticated catalog request is sufficient.

### Use server-owned UUIDv7 references and resolve them at the API boundary

Selected. It gives operators a comprehensible selection workflow while the
server remains the authority for tenant scope, lifecycle, effective windows,
canonical digests, and account topology. UUIDs are references only, never
secrets or proof of chain state.

## Invariants

- Every selected reference belongs to the authenticated merchant.
- Every split plan and policy is active/effective and passes canonical digest
  validation at selection time.
- Every destination route is active, finalized-verified, and effective.
- Destination route asset equals split-plan asset equals the stored plan asset.
- Stored plan digests are derived from server-owned records, not the request.
- The selection catalog is bounded and never returns inactive or unverified
  records as selectable options.
- Idempotent replays use the same resolved server-owned inputs and digest.

## Performance and failure policy

The catalog is a bounded PostgreSQL read with a maximum of 64 records per
reference type and no chain RPC. Plan creation performs bounded indexed
lookups and canonical validation; it does not call Solana, derive ATAs, or
invoke a tax provider. Missing, stale, cross-tenant, ambiguous, or corrupted
references fail closed with an actionable conflict/integrity response.

## UX consequences

The primary form uses labels such as “Verified payout destination”, “Settlement
split”, and “Published tax policy”. Exact IDs and digests remain available in
the record-evidence disclosure for operators and auditors. Empty and stale
catalog states explain the safe next action: provision/verify the destination,
publish a split plan, or publish a policy before proposing the payout plan.

## Release gate

This boundary does not enable wallet signing or token movement. Revenue claim
compilation, submission, Linux validator conformance, signer controls, and
production legal/security evidence remain independent release gates.

The local evidence gate is the real-browser `make web-browser-check` journey:
it loads the authenticated catalog through the NGINX-served SPA, selects the
three server-owned records, submits only UUIDv7 references and bounded
business inputs, asserts that raw digests and account topology are absent from
the request, and runs WCAG A/AA axe checks on the resulting surface. This is
synthetic local evidence and does not replace staging identity, live-wallet,
validator, or production authorization evidence.
