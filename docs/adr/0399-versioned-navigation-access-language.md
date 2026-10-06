# ADR-0399: Versioned navigation access language

- Status: Accepted
- Date: 2026-10-05
- Owners: Web Platform and Authorization UX

## Context

The shell navigation exposes authorization context through accessible names and
descriptions, but its access-state labels were hard-coded in English. That
created mixed-language output in German, Arabic, and Japanese shells and made
the navigation feel noisier than the localized workspace content. The access
decision itself is server-owned and must remain exact; only surrounding
presentation language is eligible for localization.

## Decision

1. Add a typed, immutable access-status catalog to the existing shell message
   catalog for every supported locale: available, unavailable, access check
   pending, sign-in required, and unknown workspace policy.
2. Pass the active catalog into navigation access-label resolution. No
   component may hard-code these labels or invent a locale fallback.
3. Preserve server-owned decision reasons, reason codes, capability IDs,
   tenant IDs, roles, scopes, and risk classifications exactly. They remain
   evidence and authorization values, not translated UI copy.
4. Keep access status in the accessible description and hover title only. The
   visible navigation remains task-oriented and calm; status is progressive
   disclosure rather than a persistent badge.

## Alternatives considered

### Keep English status labels in the shell

Rejected. It violates the shell localization contract and creates ambiguous
mixed-language recovery context.

### Translate server-owned decision reasons at the browser boundary

Rejected. Reasons are policy evidence and may be required for support,
replay, or authorization review. Translation would risk semantic drift.

### Remove access status from navigation entirely

Rejected. Operators and assistive-technology users need to know why a protected
workspace cannot be used, even when the visible rail stays quiet.

### Runtime machine translation

Rejected. It is nondeterministic, not reviewable, and inappropriate for
authorization-sensitive language.

## Invariants

- All supported locales have non-empty, bounded access-status messages.
- Visible task labels do not change based on authorization state.
- Access status cannot grant, deny, or mutate server authorization.
- Server-owned reasons and identifiers are not translated or rewritten.
- The navigation remains accessible, keyboard-operable, RTL-safe, and free of
  persistent status badges.

## Verification

- Failing-first catalog completeness tests for all supported locales.
- German, Arabic RTL, and Japanese rendered shell assertions proving status
  language comes from the active catalog.
- Frontend, browser, accessibility, localization, color, and repository gates.

## Consequences

Navigation status language is consistent and reviewable across locales while
authorization evidence remains exact. Future access-state copy must be added
to the typed catalog and its tests before it is used by the shell.
