# ADR-0343: Versioned document-quote localization

- Status: Accepted
- Date: 2026-10-05
- Deciders: Principal UI Architecture and Principal Low-Latency Blockchain Architecture
- Scope: Invoice tax quote workspace presentation

## Context

ADR-0342 localized the aggregate quote and tax-simulator workspaces, but the
invoice quote workspace still rendered English-only form, line-item, result,
and evidence guidance. This creates an inconsistent high-consequence workflow
for operators who have selected another supported locale and makes the invoice
entry boundary harder to review.

## Decision

Add a typed, immutable, versioned document-quote catalog for `en-US`, `de-DE`,
`ar-EG`, and `ja-JP`. The catalog covers the invoice quote form, line-item
controls, location-evidence disclosure, retry protection, service/session
states, result summary, persisted-evidence notice, and checkout handoff.

The catalog is presentation-only. Document IDs, amount strings, currencies,
jurisdictions, rule references, calculation identifiers, digests, line IDs,
server component values, and evidence explanations remain authoritative data
and are never translated or normalized by the browser. Validation and request
payloads remain unchanged. The document checkout workspace is a separate
slice because its signing and route-review vocabulary has a distinct authority
boundary.

## Alternatives considered

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Keep invoice quote in English | No code change | Inconsistent financial workflow and higher operator input risk | Rejected |
| Runtime machine translation | Broad apparent coverage | Nondeterministic tax/legal terminology, network/privacy cost, no reviewable contract | Rejected |
| Reuse aggregate quote catalog | Less code | Invoice line, evidence, and persisted-record semantics would be conflated | Rejected |
| **Typed bounded document-quote catalog** | Deterministic, reviewable, local, explicit value boundary | Requires deliberate locale and legal-copy review | **Selected** |

## Invariants and failure behavior

- Locale selection cannot change document-quote request payloads, amounts,
  currency, jurisdiction, rule references, line identity, authorization, or
  evidence bytes.
- Every supported locale has non-empty primary form and result copy and a
  deterministic English fallback for unknown locale values.
- No sample invoice, line amount, currency, rule, or jurisdiction is introduced
  by localization.
- Signed location evidence remains an opaque user input and server-verified
  record; the browser does not interpret or rewrite it.
- Withholding remains distinct from filing, reporting, legal compliance, and
  tax advice in every locale.
- Missing service, missing session, validation, timeout, and unattested states
  retain explicit fail-closed semantics in every locale.

## Latency and resource budget

- Static in-bundle lookup is O(1); no network, dynamic import, timer, or
  financial-path dependency is introduced.
- The document-quote catalog must remain under 64 KiB before compression.
- Locale changes do not recalculate, persist, submit, or alter a quote.

## Verification

- Failing-first catalog tests cover every supported locale, required keys,
  deterministic fallback, and protection against protocol-value copy.
- Existing document quote request, validation, retry, and evidence tests remain
  green without changing their wire assertions.
- Component tests cover translated labels and empty financial inputs.
- Docker browser coverage verifies German/Arabic invoice form presentation, RTL,
  axe, and unchanged document-quote route behavior through NGINX.
- Manual tax/legal terminology, screen-reader, and operator review remain
  release gates.

## References

- [ADR-016: Explainable, accessible tax UI/UX](0016-tax-ui-ux.md)
- [ADR-0273: Explicit financial-input boundary](0273-explicit-financial-input-boundary.md)
- [ADR-0342: Versioned quote workspace localization](0342-versioned-quote-workspace-localization.md)
