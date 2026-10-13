# ADR-0316: Versioned rule-release provenance readiness

## Status

Accepted — tax-rule publication now requires a bounded, versioned provenance
envelope at the storage and database boundaries. The envelope records
provenance and responsibility; it is not legal approval and does not replace
counsel or jurisdiction-owner sign-off.

## Context

The rule lifecycle already provides immutable published versions, effective
windows, four-eyes governance, and generic provenance JSON. Generic JSON is
not enough for a Tax Engine release gate: a rule could otherwise be published
without a source reference, source digest, legal/tax owner, review record, or
an explicit jurisdiction scope. The browser must not be the authority for
this decision, and direct database writes must not bypass it.

## Decision

Require this exact object when a rule changes to `published`:

```json
{
  "schema": "tax-rule-provenance/v1",
  "source_reference": "controlled-record-or-uri",
  "source_digest_hex": "64 lowercase-or-uppercase SHA-256 hex characters",
  "legal_tax_owner": "approved owner reference",
  "jurisdiction_scope": "US-CA",
  "reviewed_at_unix": 1790000000,
  "review_reference": "review-record-reference"
}
```

The server validates bounded lengths, positive/bounded review time, exact
SHA-256 shape, and that `jurisdiction_scope` equals the rule's persisted
jurisdiction. The existing database columns remain authoritative for the
effective period and the lifecycle status. PostgreSQL applies the same
fail-closed shape check for inserts and transitions to `published`; the Rust
storage boundary validates before the transaction updates the row.

Drafts may retain incomplete working provenance so authoring can proceed, but
they cannot be published until the complete envelope is present. The
provenance is already part of the immutable rule request digest and published
row mutation guard, so changing it requires a new rule version.

## Alternatives considered

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Keep arbitrary provenance JSON | Backward-compatible and flexible | Published rules can omit ownership, review, or source evidence | Rejected |
| Validate only in the React workbench | Good operator feedback | API, scripts, and direct SQL can bypass the browser | Rejected |
| Add separate relational owner tables first | Strong relational querying | More migration and lifecycle coupling before the envelope semantics are stable | Deferred |
| **Versioned envelope plus Rust and PostgreSQL publication gates** | Fail-closed authority, immutable digest binding, bounded evolution, clear diagnostics | Requires approved metadata before publication and fixture updates | **Selected** |

## Invariants and failure policy

- A `published` rule has a valid `tax-rule-provenance/v1` envelope.
- Source reference, source digest, legal/tax owner, jurisdiction scope, review
  time, and review reference are all present and bounded.
- The provenance jurisdiction matches the persisted rule jurisdiction.
- Draft authoring remains possible without publication-ready metadata.
- Missing, malformed, future/out-of-range, mismatched, or unknown provenance
  fails closed and never changes lifecycle state.
- This metadata records accountability and evidence; it does not claim that
  the product filed taxes or achieved legal compliance.

## Verification

- Storage unit tests cover valid, missing, unknown-schema, malformed-digest,
  mismatched-jurisdiction, and out-of-range review metadata.
- The local integration gate publishes a valid envelope and proves malformed
  publication is rejected without a state change.
- PostgreSQL migration/trigger tests and the Rust storage boundary provide
  defense in depth.
- The UI presents the fields as release evidence, while the server remains
  authoritative.
- Counsel, legal/tax owner approval, source provenance verification, and
  production migration/rollback evidence remain external release gates.

## References

- [ADR-0016: Legal and product readiness](0016-legal-and-product-readiness.md)
- [ADR-0143: Structured Tax Engine rule workbench](0143-structured-tax-engine-rule-workbench.md)
- [ADR-0144: Server-owned persisted rule simulation](0144-server-owned-persisted-rule-simulation.md)
- [ADR-0315: Executable tax-responsibility language boundary](0315-executable-tax-responsibility-language.md)
