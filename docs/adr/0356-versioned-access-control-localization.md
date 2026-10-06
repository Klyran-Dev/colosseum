# ADR-0356: Versioned access-control workspace localization

- Status: Accepted
- Date: 2026-10-05
- Deciders: Principal UI Architecture and Principal Security Architecture
- Scope: Access-control workspace presentation only

## Context

The access-control workspace explains RBAC, PBAC, and ABAC decisions for
operators, but its copy and policy descriptions were English-only while the
shell and operational workspaces already support versioned locale catalogs.
This creates an inconsistent production surface and makes high-consequence
authorization explanations harder to review for multilingual operators.

## Decision

Use immutable, typed presentation catalogs for English, German, Arabic, and
Japanese. Localize headings, access-state explanations, policy labels,
descriptions, groups, and risk labels. Keep authorization policy IDs,
capability decisions, reason codes, scope names, role names, tenant IDs,
subjects, and all server responses unchanged. The catalog is presentation
only; the API and server-owned capability snapshot remain the authority.

The active shell locale is passed into the workspace. Unknown locales fall
back to the English catalog. Arabic uses the shell's existing RTL direction;
the workspace must not infer authorization from color, language, or direction.

## Alternatives considered

| Alternative | Benefit | Risk | Decision |
| --- | --- | --- | --- |
| Keep authorization copy English-only | No implementation work | Inconsistent production UX and weaker operator comprehension | Rejected |
| Runtime machine translation | Broad language coverage | Unstable terminology and unsafe translation of policy semantics | Rejected |
| Translate policy IDs, scopes, or reason codes | Looks fully localized | Breaks protocol contracts, diagnostics, and audit correlation | Rejected |
| Separate localized application bundles | Strong isolation | Duplicated authorization UI and drift between bundles | Rejected |
| **Typed versioned presentation catalogs** | **Deterministic terminology, stable protocol values, testable RTL/accessibility behavior** | **Requires catalog maintenance and linguistic review** | **Selected** |

## Invariants and verification

- Authorization remains server-owned and fail-closed.
- Policy IDs, decision kinds, reason codes, scopes, roles, tenant IDs, and
  subjects are never translated or rewritten.
- Catalog completeness is tested for every supported locale.
- German and Arabic RTL browser journeys include axe checks.
- `make web-check`, `make web-browser-check`, and `make ui-language-contract`
  remain required gates.

