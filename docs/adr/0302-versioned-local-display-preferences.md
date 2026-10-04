# ADR-0302: Versioned local display preferences

## Status

Accepted — implementation in the static SPA with unit and browser evidence.

## Context

The operational shell exposes language, time-zone, and Material theme controls.
These are presentation preferences rather than account, authorization, tax,
wallet, or settlement state. Losing them on reload makes a production SaaS
console harder to operate, especially for RTL and regional time-zone users.
Persisting the wrong state would be a security and correctness defect: bearer
credentials, capability snapshots, financial facts, and wallet artifacts must
never be stored in browser persistence.

## Decision

Persist only a versioned `DisplayPreferences` object in `localStorage` under a
repository-owned key. The object contains the selected supported locale,
validated IANA time zone, and `system`/`light`/`dark` theme preference. Reads
are defensive: malformed JSON, unknown locales, invalid time zones, unknown
themes, oversized values, or storage exceptions resolve to safe defaults.
Writes are best effort and never affect authorization or financial behavior.

The preference schema is presentation-only. It is not tenant configuration,
an identity claim, an audit input, a tax-engine input, a transaction digest
input, or a security decision. Authentication tokens, PKCE state, capability
snapshots, quotes, calculations, signed bytes, and wallet keys remain outside
this store.

## Alternatives considered

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Reset preferences on every reload | No browser persistence | Poor operator continuity; breaks regional/RTL workflows | Rejected |
| Persist the complete React application state | Convenient reload behavior | Leaks credentials, stale authority, financial facts, and wallet state | Rejected |
| Server-saved user profile preferences | Cross-device continuity | Requires authenticated profile ownership, migrations, and a new network dependency | Deferred |
| **Versioned local presentation-only preferences** | Fast, private, bounded, no hot-path dependency, easy to invalidate | Device-local rather than cross-device | **Selected** |

## Invariants and failure policy

- Only the documented schema and allow-listed fields are read or written.
- Invalid or oversized persisted values fall back to safe defaults.
- Persistence failure never blocks rendering, sign-in, quote, tax, or settlement.
- The persisted object never contains bearer credentials, authorization data,
  financial values, signed transaction bytes, or wallet material.
- Theme, locale, and time zone cannot change capabilities, API authority,
  calculation inputs, transaction bytes, or settlement behavior.

## Verification

- Unit tests cover absent, valid, malformed, unknown, oversized, and storage-
  exception cases, including the no-sensitive-fields contract.
- The real-browser suite changes locale, time zone, and theme, reloads the
  NGINX-served SPA, and verifies the preferences are restored without
  restoring the local credential.
- Existing accessibility, color, language, CSP, and production-build gates
  remain required.
