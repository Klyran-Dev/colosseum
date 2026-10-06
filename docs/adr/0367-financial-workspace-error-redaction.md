# ADR-0367: Financial-workspace error redaction boundary

- Status: Accepted
- Date: 2026-10-05
- Deciders: Principal UI Architecture, Security, and Settlement Integration
- Scope: Browser presentation of API, transport, parser, and wallet failures in financial workspaces

## Context

The wallet picker now maps arbitrary injected-provider exceptions to localized
copy, but other payment, refund, reconciliation, treasury, governance, quote,
revenue-streaming, and application-shell paths still use `Error.message` as a display value. That value may contain a provider
diagnostic, an HTTP/client implementation detail, a parser response, or text
that was never reviewed for operator language. A bounded string is not the
same thing as a safe string.

The browser must preserve the server and finalized chain as authorities while
giving an operator a recoverable next step. A raw exception must not become a
second API contract, a localization bypass, or a support-data disclosure.

## Decision

Introduce a typed presentation boundary for operator errors:

1. Only explicitly marked `OperatorFacingError` values may carry a detailed
   message into a primary financial-workspace alert.
2. Ordinary `Error`, DOM, wallet, parser, and transport exceptions map to the
   localized or bounded operation fallback supplied by the workspace.
3. API failures retain the validated UUIDv7 support reference, but the browser
   does not render the untrusted response `message` by default. HTTP status
   suffixes are removed from primary copy; status and correlation data remain
   available to support telemetry and technical evidence surfaces.
4. User-actionable validation and state guidance is created explicitly through
   the typed constructor. This does not authorize client-side financial
   decisions or retries.
5. Existing unknown-outcome, reconciliation-required, expiry, and authorization
   state machines remain unchanged; only their presentation boundary changes.

## Alternatives considered

- Render bounded `Error.message`: rejected because length and control-character
  limits do not establish provenance or safe operator language.
- Remove all detail and show one generic message: rejected because expiry,
  authorization, validation, and unknown settlement outcomes require distinct
  recovery guidance.
- Parse/classify arbitrary provider and server messages in the browser:
  rejected because wording and classification would be unstable, localized
  inconsistently, and could change the intended action.
- **Typed operator-facing errors plus localized fallback mapping:** selected for
  explicit provenance, deterministic rendering, accessibility, and fail-closed
  wallet/settlement behavior.

## Invariants

- An ordinary exception can never render its message in the primary financial
  workspace.
- A support reference is shown only when it matches the API UUIDv7 contract.
- No exception mapping authorizes, retries, signs, submits, reconciles, or
  declares completion of a financial action.
- Explicit validation copy remains actionable and is not replaced by technical
  diagnostics.
- Error rendering remains locale-aware, keyboard/screen-reader accessible,
  and safe for the existing Material 3 semantic state palette.

## Verification and rollout

- Add failing unit tests for ordinary exception redaction, explicit operator
  copy, timeout/provider errors, API support-reference retention, and HTTP
  status removal.
- Apply the boundary first to aggregate/document checkout and refund, then to
  tax-document simulation, rule governance, document quote, treasury/vault,
  adjustment, replay, revenue-streaming, and application-shell paths.
- Add real-browser assertions for a server diagnostic and a wallet/provider
  diagnostic in payment review; preserve unknown-outcome and reconciliation
  assertions.
- Keep live provider, staging identity, manual assistive-technology, and
  support-runbook evidence as separate release gates.
