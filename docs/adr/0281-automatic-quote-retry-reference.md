# ADR-0281: Automatic quote retry reference

## Status

Accepted — implemented and locally verified; retained staging/release evidence
remains open.

## Context

Quote and invoice-quote requests require an idempotency key so a retry of the
same facts returns the original durable calculation instead of creating a
second record. The prior UI exposed that generated UUID as an editable
“Request reference” field. This added protocol vocabulary and allowed an
operator to accidentally reuse a key after changing financial facts, causing a
conflict or an unsafe retry workflow.

The key is not an accounting fact, authorization input, or support identifier.
It is a browser-generated replay-control value. The operator needs to know
that retry protection exists, not to edit its implementation value.

## Decision

The aggregate and document quote workspaces generate a cryptographically secure
UUID idempotency key in memory. The key is regenerated whenever a semantic
quote input changes: facts, currency, jurisdiction, rule selection, regime,
location evidence, or invoice lines. It remains stable while the input set is
unchanged, including a retry after a transient response or timeout.

The request continues to send the unchanged `idempotency_key` API field. The
primary UI renders only: “Retry protection selected automatically” and a
plain-language explanation. The UUID is not an editable input, URL value,
financial fact, authorization value, or displayed technical identifier.

## Invariants and failure behavior

- Identical inputs in one workspace session reuse one idempotency key.
- Any semantic input mutation creates a fresh key before the next submission.
- A missing secure random source fails closed; timestamps and predictable
  fallbacks are never used.
- Server-side idempotency and conflict semantics remain authoritative.
- A retry after a timeout does not silently generate a second key unless the
  operator changed the quote facts.
- The request contract remains backward-compatible; only presentation and
  browser key lifecycle change.

## Alternatives considered

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Keep an editable visible UUID field | Explicit and easy to inspect | Technical cognitive load, accidental reuse after edits, and copy/paste errors | Rejected |
| Generate a new key for every submit | Simplest implementation | Retries create duplicate calculations and defeat idempotency | Rejected |
| Derive a key from financial facts | Stable for equivalent input | Facts may contain sensitive data; canonicalization changes become compatibility hazards | Rejected |
| Store the key in persistent browser storage | Survives reloads | Extends retention of a replay credential and complicates tenant/session isolation | Rejected |
| **In-memory CSPRNG key regenerated on semantic input change** | Safe retry stability, no persistence, low latency, simple API compatibility, and plain-language UX | Reload starts a new request identity | **Selected** |

## Verification

- Component tests prove the technical input is absent and the retry-protection
  explanation is present.
- API integration tests continue to prove exact replay and changed-input
  conflict behavior.
- Full web tests, typecheck, accessibility, browser journeys, and public UI
  language checks remain required.

