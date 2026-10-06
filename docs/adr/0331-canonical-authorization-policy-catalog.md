# ADR-0331: Canonical authorization policy catalog

- Status: Accepted
- Date: 2026-10-04
- Deciders: Principal Low-Latency Architecture, Security, and Product UI
- Scope: RBAC/PBAC/ABAC capability metadata shared by API and React affordances

## Context

The API capability snapshot and the React access-review/navigation affordance
catalog described the same RBAC, PBAC, and ABAC policy IDs in separate
handwritten lists. That duplication could make a protected workspace appear
with stale labels, risk language, required roles, or context requirements even
when the server decision was correct. The browser catalog is not an authority,
but drift still creates misleading product behavior and increases review cost.

## Decision

Maintain one versioned, repository-owned catalog at
`contracts/authorization-capabilities-v1.json`. It contains the stable policy
ID, plain-language product label and description, workspace/group, action-risk
classification, and the required scopes, roles, and resource attributes.

The API parses and validates the bounded catalog once during process startup,
before binding its listener, and retains the immutable result for capability
snapshots. A malformed catalog prevents startup; endpoint-level failure also
remains fail closed if the catalog becomes unavailable through an unexpected
runtime path. It never enables an action. The
React application imports the same versioned artifact as a typed affordance
projection. It may explain or hide navigation, but every protected API,
settlement, wallet, and Solana authority boundary repeats server-side checks.

The catalog is intentionally metadata, not a dynamic policy language. It does
not contain tenant data, tokens, credentials, financial values, account
addresses, or user-supplied expressions. Any authorization-semantic change
requires a catalog version/schema review, API/UI parity tests, and an ADR or
superseding ADR.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| Keep independent Rust and TypeScript lists | Rejected | Drift can mislead operators and requires duplicate review. |
| Let the browser fetch policy metadata at runtime | Rejected | Adds a dependency to navigation, exposes a mutable presentation path, and does not improve server authority. |
| Generate only TypeScript from the Rust list | Rejected | Keeps the API implementation as an implicit source and complicates clean Docker builds. |
| **Versioned JSON contract consumed by API and browser** | **Selected** | One reviewable source, deterministic builds, bounded parsing, and explicit server-authority boundaries. |

## Invariants and failure policy

- The schema is `authorization-catalog/v1`; policy IDs are unique and bounded.
- Every policy has non-empty bounded identity and product metadata.
- Scope, role, and attribute arrays are bounded, duplicate-free, and use the
  approved token grammar; attributes are restricted to the supported ABAC
  context keys.
- Group, workspace, and action-risk values are restricted to the approved
  product vocabulary, so a typo cannot silently create an unrendered or
  differently protected workspace.
- The browser projection must equal the canonical catalog in CI.
- The API capability response must expose every catalog policy exactly once.
- Catalog failure cannot turn an indeterminate or denied action into allowed.
- Catalog metadata never grants access and never replaces handler-level checks.
- API startup preflight must complete before listener binding; a failed
  preflight must expose no serving endpoint.

## Latency and operations

The catalog is parsed once per API process during startup and retained immutably. Capability
evaluation remains an in-memory bounded scan with no database, RPC, broker, or
filesystem request on the snapshot path. The catalog is small enough to remain
well below the documented request and memory budgets; a future increase must
be bounded and benchmarked before acceptance.

## Verification

- Rust validates the catalog shape, product vocabulary, token grammar, and
  capability-snapshot coverage; semantic drift fails closed before the first
  snapshot is issued.
- The API startup preflight is invoked before `TcpListener::bind`, so an
  invalid catalog cannot produce a healthy-looking listener.
- React tests prove the access-review projection is semantically equivalent to
  the canonical policy entries.
- Docker API and NGINX builds copy the same contract into their build contexts.
- `make check`, `make web-browser-check`, and `make integration-check` remain
  required. The catalog does not close external IdP, legal, staging, or
  production authorization evidence gates.
