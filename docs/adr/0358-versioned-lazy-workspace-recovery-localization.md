# ADR-0358: Versioned lazy-workspace and recovery-state localization

- Status: Accepted
- Date: 2026-10-05
- Deciders: Principal UI Architecture and Principal Security Architecture
- Scope: Static SPA lazy-load fallbacks and workspace error recovery state

## Context

The operational shell and workspaces are localized, but lazy-loaded workspace
fallbacks and the error boundary still displayed English-only copy. These
states are part of the production flow: they appear during normal chunk
loading, deployment faults, and recoverable frontend failures.

## Decision

Use the shell-operational typed catalog for localized lazy-load and recovery
copy in English, German, Arabic, and Japanese. The fallback receives the
localized workspace label from the existing shell catalog. The error boundary
localizes only bounded presentation text and never renders exception details,
stack traces, credentials, or inferred financial/authorization state.

The fallback remains a static client state and does not claim that a server
operation succeeded. Retry is a full application reload, preserving the
existing recovery behavior.

## Alternatives considered

| Alternative | Benefit | Risk | Decision |
| --- | --- | --- | --- |
| Keep loading/error copy in English | No implementation work | Mixed-language recovery flow | Rejected |
| Render raw chunk/error text | More diagnostic detail | Credential/implementation leakage and unsafe claims | Rejected |
| Duplicate catalogs in every workspace | Local ownership | Drift and inconsistent recovery semantics | Rejected |
| **Reuse the typed shell-operational catalog** | **Consistent, bounded, accessible, protocol-value preserving** | **Requires catalog maintenance and linguistic review** | **Selected** |

## Verification

- Error-boundary unit tests prove localized recovery and exception redaction.
- Production build/typecheck and the full Dockerized browser suite remain
  required.
- Existing failure-path browser checks must continue to assert no signing or
  settlement state is inferred.

