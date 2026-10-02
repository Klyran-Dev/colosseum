# ADR-0260: Authenticated server-owned FX quote composition

- Status: Implemented as an explicit application boundary; non-local activation remains gated
- Date: 2026-10-02
- Owners: API/Tax, FX Provider, Security/Privacy, Tax/Legal
- Scope: authenticated aggregate and document quote composition

## Context

The FX provider bounded context now has bounded provider acquisition, policy
agreement, server-owned signing, and durable quote-envelope persistence. The
authenticated quote handlers still reject every client FX conversion and pass
`fx_evidence: None` to storage. That leaves the production composition gap
between an authenticated merchant request and the existing coordinator/storage
contracts.

The application must accept the source currency as a fact of the merchant's
request, but it must not accept a provider, policy digest, reference rate,
evidence envelope, signer identity, or provider endpoint from the browser. The
target currency remains the `TaxFacts.currency`/settlement currency. Same-currency
requests must remain on the no-FX hot path.

## Decision

Add a server-owned asynchronous issuance port in the FX Provider bounded
context. The API stores an `Arc<dyn AsyncFxEvidenceIssuance>` and receives a
server-created `FxEvidenceRequest` only after authentication and canonical
pre-resolution facts hashing. The port returns locally verified
`IssuedFxEvidence`; the API converts its exact signed projection into
`TaxFacts.fx_conversion` and persists the complete `PersistedFxEvidence` in the
same quote transaction already used for encrypted facts and the first outbox
record.

The request contract adds an optional `source_currency` field to aggregate and
document quote requests:

- absent means no FX is requested and preserves the existing path;
- present and equal to the target currency is rejected;
- present and distinct requires a configured server-owned issuance port;
- a missing port returns a bounded service-unavailable error and never falls
  back to caller-owned evidence or a synthetic rate.

Policy digest, reference rate, provider topology, legal acceptance, signer
identity, custody, and clock remain server-owned configuration behind the port.
No provider or HSM/KMS vendor is selected by this ADR. `AppState` may carry the
port for local synthetic fixtures and a future approved deployment composition,
but the process remains non-local fail-closed until the decision register and
release evidence are frozen.

The API computes the pre-resolution facts digest once, verifies jurisdiction
evidence against that digest, issues FX evidence against the same digest, then
passes the enriched facts and typed persisted envelope to storage. The two
independent bounded dependencies are kept out of the tax-engine domain library;
the API does not perform retries beyond the coordinator's explicit budgets.

Quote idempotency uses the pre-resolution facts digest, selectors, jurisdiction
evidence digest, and normalized source currency. It deliberately excludes the
provider quote ID, observation/expiry window, signature, and complete envelope.
This means a bounded concurrent retry can obtain a replacement server-owned
quote, while the existing immutable database response is still returned rather
than being treated as a request conflict. The complete envelope remains
immutable evidence and is validated by the storage/replay boundary.

## Alternatives considered

| Alternative | Result | Reason |
| --- | --- | --- |
| Accept `FxEvidenceEnvelope` or `FxConversionEvidence` from the browser | Rejected | Caller-controlled evidence can bypass provider policy, custody, and legal admission. |
| Call providers directly from each quote handler | Rejected | Duplicates orchestration, creates policy-reload races, and couples HTTP to API handlers. |
| Put FX issuance inside the pure Tax Engine | Rejected | Adds clock/network/signer dependencies to a deterministic domain library. |
| Queue FX issuance through an asynchronous broker | Rejected for the synchronous quote path | Adds delivery/idempotency ambiguity and threatens quote-expiry latency. |
| Require a concrete vendor during this implementation | Rejected | DEC-0251-01/03/09 remain open; inventing a provider, legal owner, or custody model would be unsafe. |
| **Server-owned API port backed by the bounded coordinator** | **Selected** | Completes the application composition boundary while preserving tenant scope, exact evidence binding, low-latency budgets, and explicit non-local release gates. |

## Invariants

- Authenticated tenant identity is the only merchant authority boundary.
- The browser cannot provide FX evidence, policy digest, provider identity,
  reference rate, or signer material.
- The envelope and projected conversion bind to the same pre-resolution facts
  digest used for jurisdiction evidence.
- Same-currency requests do not invoke the FX port.
- Missing, malformed, stale, disputed, or unavailable FX configuration fails
  closed without calculation persistence.
- Aggregate and document quote paths share one enrichment contract and one
  server-owned port; they cannot diverge in binding semantics.
- Idempotency uses the pre-resolution request identity and normalized source
  currency; server-issued provider evidence is persisted and immutable but is
  not used as a caller request digest, so retries cannot turn a refreshed
  server quote into a false conflict.

## Verification

Tests must prove same-currency bypass, missing-port fail-closed behavior,
source/target validation, caller-owned evidence rejection, exact merchant/facts
binding, aggregate/document parity, coordinator failure propagation, and
idempotency stability across server-issued quote replacement. Storage tests
prove that the durable envelope and outbox remain committed atomically by the
existing storage boundary.

This ADR does not claim provider approval, legal coverage, HSM/KMS custody,
regional SLO evidence, or production FX enablement. Those remain explicit
release gates in the decision register.
