# ADR-0361: Distinct tax-base sources

- Status: Accepted
- Date: 2026-10-05
- Deciders: Principal Low-Latency Blockchain Architecture
- Scope: Tax Engine rule validation

## Context

`TaxBase.sources` is an additive list. It intentionally supports composite
bases such as `Net + Component(environmental-levy)` for tax-on-tax rules, but
the previous validator also accepted repeated entries such as `Net + Net`.
That representation is indistinguishable from a malformed duplicate and can
silently double a taxable base. A global engine must reject ambiguous rule
semantics before a rule can be compiled or cached.

## Decision

Every source in a component tax base must be distinct. The compiler rejects a
repeated `Gross`, `Net`, or component reference with a stable invalid-rule
error. Distinct additive sources remain supported, and component references
continue to require an acyclic dependency graph.

This validation is part of rule compilation, before canonical digesting and
before a rule can enter the warm compiled-rule cache. It therefore applies
equally to API-created rules, persisted rule documents, golden vectors, and
replay.

## Alternatives considered

### Permit repeated sources as intentional weighting

Rejected. Weighted bases need an explicit, versioned operation in the DSL; a
repeated identifier is too easy to create through authoring or serialization
error and is not self-describing.

### Deduplicate silently

Rejected. Silent normalization would change the author’s requested rule while
preserving an apparently valid document and would make evidence replay harder
to explain.

### Replace the source list with a general expression language

Rejected for this boundary. It expands the attack surface and validation
surface without being required for additive bases. A future expression feature
must be separately bounded, versioned, and reviewed.

## Consequences

- Malformed additive bases fail closed before calculation, persistence, or
  settlement attestation.
- Composite tax-on-tax bases remain available when their sources are explicit
  and distinct.
- A future weighted or subtractive base must introduce a new DSL operation and
  golden vectors rather than overloading duplicate list entries.
- The hot calculation path is unchanged; the check is paid once during rule
  compilation.

## Verification

- `rejects_duplicate_tax_base_sources_before_calculation` proves `Net + Net`
  is rejected before graph compilation.
- Existing cascading vectors and dependency-order tests prove distinct
  `Net + Component(...)` bases continue to calculate correctly.
- The full repository gate must remain green before release-candidate review.
