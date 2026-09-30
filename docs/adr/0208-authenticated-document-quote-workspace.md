# ADR-0208: Authenticated document-quote workspace

- Status: Accepted for implementation
- Date: 2026-09-30
- Owners: Web Platform, Tax Platform, Jurisdiction, Security

## Context

The API has a production document-quote boundary that authenticates the
merchant, resolves jurisdiction from signed provider evidence, calculates
with the published Rust rule, persists immutable facts/result evidence, and
returns a UUIDv7 calculation. The SPA currently exposes only an isolated
document simulation workbench, which cannot create settlement authority. That
forces operators to use a separate client to start the real document flow.

## Decision

Add a dedicated lazy authenticated document-quote workspace. It collects
document facts, published rule selector, merchant-scoped idempotency key, and
provider-signed jurisdiction evidence, then calls only
`POST /v1/tax/document-quote`. It displays the returned persisted quote,
expiry, complete line trace, facts digest, rule digest, and explicit
`calculated_unattested`/`attested` state. The workspace links the returned
calculation ID to document checkout but does not verify attestations, compile
settlement, or infer legal compliance.

The browser never supplies merchant identity, jurisdiction resolution, tax
totals, rule contents, or private issuer keys. Provider evidence is submitted
as untrusted JSON and remains non-authoritative until the server verifies it.
The existing simulation workbench remains visibly isolated and cannot create a
quote or settlement intent.

## Alternatives considered

| Alternative | Advantages | Rejection reason |
| --- | --- | --- |
| Reuse simulation form and change its endpoint | Smallest UI diff | Would blur simulation and production authority and risks sending caller-owned rule data to the quote boundary. |
| Hide document quotes behind aggregate quote UI | Fewer navigation items | Aggregate/document facts and attestation contracts are distinct; the UI would conceal scope and evidence differences. |
| Require operators to use an API client | No additional SPA work | Not a supportable or accessible product workflow and creates an undocumented production path. |
| **Dedicated authenticated document-quote workspace** | Explicit authority boundary, complete evidence trace, idempotent retry UX, and direct handoff to document checkout | Adds a separately tested lazy workspace; selected. |

## Invariants and verification

- The server derives tenant identity and jurisdiction resolution; browser input
  cannot override either authority.
- All financial values are displayed from server-returned decimal strings and
  remain exact atomic values in the browser.
- Same-key retry must show the same persisted calculation; a conflicting key
  must show the server error and never fabricate a new result.
- Expired, unavailable, malformed, or unauthorized responses are visible and
  do not unlock attestation or settlement controls.
- Unit, accessibility, real-browser lazy-workspace, and API contract tests
  cover the workspace; Docker integration remains the authority for
  persistence, tenant isolation, idempotency, and jurisdiction evidence.

## Consequences

Merchants can start the real document evidence chain in the product UI while
simulation remains useful for rule authoring. The workspace does not claim
universal jurisdiction coverage or legal/tax compliance; only published,
reviewed coverage records can be used by production policy.
