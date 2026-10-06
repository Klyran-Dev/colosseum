# ADR-0357: Versioned shell operational localization

- Status: Accepted
- Date: 2026-10-05
- Deciders: Principal UI Architecture and Principal Security Architecture
- Scope: Static shell diagnostics, account context, protection disclosure, and footer presentation

## Context

The navigation and workspaces use versioned locale catalogs, but the shell's
system-status disclosure, account menu, access context, protection details,
and footer still contained English-only copy. These surfaces are part of every
high-consequence workflow and must not switch language midway through an
operator's review.

## Decision

Add a small immutable typed catalog for shell-operational presentation in
English, German, Arabic, and Japanese. Pass the active locale to the system
status disclosure and use the catalog for account/session/access/protection
copy. Preserve runtime environment and network labels, subject/tenant values,
capability decisions, reason text, and configuration warnings exactly as
server/build-owned values. Unknown locales fall back to English.

This catalog is not an authority boundary and does not translate or interpret
status values. It changes presentation only and keeps the existing Material
3 semantic state roles, RTL direction, and accessibility structure.

## Alternatives considered

| Alternative | Benefit | Risk | Decision |
| --- | --- | --- | --- |
| Leave shell diagnostics in English | No code change | Mixed-language high-consequence workflows | Rejected |
| Add locale conditionals throughout `ProductShell` | Fast first change | Duplicated strings and drift across states | Rejected |
| Runtime machine translation | Broad coverage | Unstable operational/security terminology | Rejected |
| **Typed shell-operational presentation catalog** | **Deterministic, testable, protocol-value preserving** | **Requires catalog maintenance and linguistic review** | **Selected** |

## Verification

- Catalog tests cover all supported locales and fallback behavior.
- Existing shell localization and RTL browser journeys remain green.
- `make web-check`, `make web-browser-check`, and `make ui-language-contract`
  remain required gates.

