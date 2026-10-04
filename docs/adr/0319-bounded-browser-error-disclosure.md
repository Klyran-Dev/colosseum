# ADR-0319: Bounded browser error disclosure

- Status: Accepted
- Date: 2026-10-04
- Deciders: Principal UI Architecture and Principal Low-Latency Blockchain Architecture
- Scope: React browser API response-error boundary

## Context

The browser receives error payloads from authenticated and unauthenticated API
boundaries. React safely escapes the resulting text, but an error message is
still untrusted input. An unbounded message can overwhelm an operator surface,
carry control characters into support artifacts, or expose server detail that
was never intended to be a user-facing explanation. A whitespace-only message
must not hide the operation-specific fallback.

## Decision

The shared `responseError` boundary applies a dependency-free display policy
before any error enters React state:

1. accept only a string `error.message` from the bounded response shape;
2. replace C0 and DEL control characters with spaces;
3. collapse whitespace and trim the result;
4. use the trusted operation fallback when the normalized message is empty;
5. truncate the selected message to 512 characters with a single ellipsis; and
6. append a support reference only when the response header is a canonical
   UUIDv7.

The browser does not render raw response bodies, stack traces, headers, or
caller-controlled request identifiers. The API remains responsible for
authoritative error classification and audit evidence; this boundary controls
only user-facing disclosure.

## Alternatives considered

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Render the server message unchanged | Preserves every diagnostic detail | Unbounded disclosure, control characters, and accidental sensitive detail | Rejected |
| Show status code only | Small and predictable | Removes actionable operator guidance and support context | Rejected |
| Add a sanitizer dependency | Potentially broader policy surface | Extra supply-chain/runtime cost for a small deterministic boundary | Rejected |
| **Pure bounded normalizer plus strict UUIDv7 support-reference check** | Deterministic, dependency-free, auditable, and low-latency | Long diagnostics require server-side logs and correlation lookup | **Selected** |

## Invariants and failure policy

- No rendered API error message exceeds 512 characters before a support
  reference is appended.
- Control characters and line-breaking whitespace do not cross the display
  boundary.
- Empty or malformed server messages use the operation-specific fallback.
- An invalid request ID is never rendered as a support reference.
- The normalizer performs no network, storage, clock, or authorization work.

## Verification

- `apps/web/src/api.test.ts` covers valid support references, invalid IDs,
  long/control-character messages, and whitespace-only fallback behavior.
- The complete frontend suite, typecheck, production build, browser journeys,
  and public repository contract remain required after changes.

## References

- [ADR-0147: Lazy workspace error containment](0147-lazy-workspace-error-containment.md)
- [ADR-0271: Executable UI language contract](0271-executable-ui-language-contract.md)
