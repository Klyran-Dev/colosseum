# ADR-0282: Plain-language primary UI with progressive technical disclosure

- Status: Accepted
- Date: 2026-10-04
- Owners: Product and UI architecture

## Context

The product serves payment operators, tax reviewers, finance teams, and
incident responders. Those users need exact rule, authorization, and evidence
references, but exposing implementation vocabulary in every first-view label
makes a high-consequence workflow harder to understand and increases input
mistakes. The product must remain auditable without turning the primary UI into
an API console.

## Decision

Primary navigation, headings, labels, empty states, and action buttons use
plain task language: “Approved tax rule”, “Location verification”, “Review the
tax calculation”, “Permission status”, and “Reason for this decision”.

Exact identifiers, fingerprints, engine versions, account keys, policy
references, and protocol details remain available through clearly labelled
“Record details”, “Technical details”, or administrator-only disclosures.
No disclosure may alter authorization, change a server-owned value, or imply
that a calculation, signature, or provider response is final settlement.

Guided controls are the default for rule authoring. Exact JSON and protocol
fields remain available only where the API contract requires an advanced
exchange, with an explanation of ownership and validation.

Provider-signed location records follow the same boundary. The quote form
groups the raw signed record under “Location verification” and labels it as an
advanced exchange. The service still validates the provider signature, tenant
binding, facts binding, validity window, and policy; the browser does not parse
or approve the record.

## Alternatives considered

| Alternative | Decision |
| --- | --- |
| Expose API and blockchain vocabulary in all labels | Rejected: increases cognitive load and copy/paste errors |
| Hide all identifiers and evidence | Rejected: prevents audit, support, and incident work |
| Build separate operator and administrator applications | Deferred: duplicates navigation and policy surfaces |
| **Plain primary workflow plus progressive technical disclosure** | **Selected: preserves auditability while keeping the task understandable** |

## Consequences

- Product copy must be tested for both visible task language and retained
  administrator detail.
- Every technical disclosure needs a stable accessible name and must work with
  keyboard and screen readers.
- The service remains authoritative: this decision changes presentation only;
  it does not move validation or authorization into the browser.
- New user-facing terminology requires review against the product glossary and
  the UI language contract.

## Verification

- Component tests assert that primary invoice and rule-authoring labels do not
  expose internal rule-set, regime, location-evidence, or serialization
  vocabulary.
- UI-language contract checks prohibit example, fake-success, and authority
  claims in the first-view product surfaces.
- Accessibility and browser journey tests cover navigation, disclosure, and
  error states.
