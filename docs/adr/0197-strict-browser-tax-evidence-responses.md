# ADR-0197: Strict browser tax-evidence response parsing

## Status

Accepted — calculation and replay response parsing is implemented before
financial evidence reaches the SPA state or rendering boundary.

## Context

The Rust API serializes calculation evidence as UUIDv7 identities, signed
decimal `i128` atomic amounts, fixed 32-byte digests, bounded component traces,
and optional quote expiry. The SPA previously cast decoded JSON directly into
TypeScript types. That left malformed, numeric, legacy, or structurally
unbounded responses capable of reaching formatters, checkout, or replay UI.

## Decision

Introduce a pure `taxResponses` parser at the browser/API boundary. It must:

- accept only the three published calculation statuses and the exact
  `replay_verified` status;
- validate UUIDv7 calculation identity and positive bounded versions/timestamps;
- preserve atomic amounts as canonical signed decimal strings within `i128`,
  without converting through `number` or `float`;
- validate fixed-size digest byte arrays, bounded text, component count, and
  component atomic fields;
- validate optional jurisdiction-resolution evidence when present; and
- return typed values only after validation, with callers rendering a redacted
  error and no financial fallback on failure.

Unknown additive fields remain forward-compatible, but required fields and
their wire semantics are never inferred. Server/API contract validation remains
authoritative; this parser is a client-side defense-in-depth boundary.

## Alternatives considered

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Direct TypeScript casts | Minimal code | Untrusted financial data reaches rendering and wallet workflows | Rejected |
| Runtime schema library for every response | Broad tooling and generated schemas | Added bundle/dependency cost and less explicit protocol semantics | Rejected for this bounded hot path |
| **Small pure hand-written parser** | Explicit bounds, no runtime dependency, easy unit/golden testing, low allocation | Must be maintained with the API contract | **Selected** |
| Validate only at the API | One validation point | Browser can still be fed stale/malformed intermediary responses | Rejected |

## Invariants and verification

- No invalid calculation or replay payload enters React state.
- No atomic value is converted through JavaScript `number`.
- Non-canonical signed decimals, numeric JSON amounts, invalid digests,
  unsupported statuses, and malformed identities fail closed.
- Unit tests cover valid evidence, bounds, malformed fields, and replay status.
- Web typecheck, unit tests, production build, and pinned NGINX browser
  journeys remain required after changes.

Verification recorded for this decision:

- `pnpm --dir apps/web test`: 82 passed, 6 skipped.
- `pnpm --dir apps/web typecheck`: passed.
- `pnpm --dir apps/web build`: passed.
- `make web-browser-check`: 10/10 Playwright journeys passed through the
  pinned NGINX edge, including expired-quote, wallet-rejection, accessibility,
  API-unavailable, and reconciliation failure states.
- `make public-docs-check`: passed.
