# ADR-0467: Reflow narrow evidence summaries into readable rows

- Status: Accepted
- Date: 2026-10-10
- Owners: Product engineering, accessibility, and operations

## Context

Operational summaries use definition-list rows for amounts, slots, account
addresses, signatures, digests, and actor references. The default two-sided
flex row is efficient on wide panels but becomes a cramped label/value split at
narrow workspace widths. Long protocol identifiers can dominate the value
column and make the summary difficult to scan or localize.

## Decision

Keep two-sided summary rows at comfortable widths. Below the workspace's
34rem container threshold, switch each row to a single-column stack, place the
label above the value, and allow protocol values to wrap anywhere. This is
limited to summary evidence; form grids and action layouts retain their own
responsive contracts.

## Alternatives considered

1. **Keep the flex row at every width.** Rejected because long identifiers
   compete with labels and reduce readable line length.
2. **Hide or truncate protocol values.** Rejected because evidence references
   must remain inspectable and truncation can obscure the distinction between
   records.
3. **Stack labels and values at a container-bound threshold.** Selected because
   it preserves desktop scan speed while making narrow evidence readable and
   localization-safe.

## Invariants and acceptance criteria

- Labels and values remain present and associated in the same definition list.
- Long IDs, hashes, subjects, and signatures wrap without page-level overflow.
- Right-to-left summary alignment continues to follow the existing direction
  rules outside the narrow stacked treatment.
- No authority, financial value, or evidence data is removed or transformed.

## Verification

- The stylesheet contract requires the narrow container query and wrapping
  declarations.
- The frontend and Docker browser suites cover 320px reflow, RTL, themes,
  keyboard interaction, and accessibility.
