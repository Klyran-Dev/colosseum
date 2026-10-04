# ADR-0152: Settlement-adjustment operator workspace

## Status

Accepted and implemented for the first UI slice. This ADR covers the browser
proposal, evidence review, and governed lifecycle surface; it does not
authorize token movement or represent `posted` state.

## Context

ADR-0150 defines exact immutable adjustment semantics and ADR-0151 exposes a
tenant-scoped proposal/approval API. Without a real operator surface, those
controls are difficult to operate safely: an operator cannot inspect the
server-owned record, distinguish a stale or unavailable API from an empty
result, or provide bounded evidence for an approval decision. A browser must
also avoid inventing financial state, accepting a caller-selected merchant, or
presenting approval as payment.

## Decision

Add a dedicated lazy-loaded `SettlementAdjustmentPanel` to the React/Vite SPA:

1. The workspace provides real forms for propose, lookup, and lifecycle
   transition. It calls only the documented API routes and never writes a
   receipt, signs a transaction, or exposes a `posted` option.
2. The authenticated bearer token remains memory-only and is sent only to the
   API. The UI does not decode roles or decide authorization; the server is the
   authority. Client validation improves feedback but is repeated by the API.
3. The proposal form uses a browser CSPRNG UUIDv7-shaped client idempotency key
   helper, exact decimal strings, fixed-length byte-array validation, and
   explicit conservation validation before submission. It performs no
   JavaScript floating-point money arithmetic.
4. The panel renders server-owned status, request/domain/evidence digests,
   proposer/approver subjects, and lifecycle evidence as immutable review
   material. Financial strings remain strings and are never reformatted into a
   different value.
5. Every operation has loading, success, validation error, authorization
   error, timeout, offline, malformed-response, and retry states. A timeout or
   unknown status never becomes success. Focus moves to the workspace heading
   when selected, and status/errors use semantic live regions.
6. The proposal form starts empty for settlement identifiers, amounts, asset
   and evidence digests, and source references. Test fixtures must supply
   synthetic values explicitly; the production workspace never invents a
   financial or evidence value.
7. The layout follows the existing product visual language, works at mobile
   widths and RTL direction, and labels the legal/operational boundary:
   approval is evidence and does not mean refund paid, filing complete, or tax
   compliance.

## Alternatives considered

See `docs/architecture/alternative-analysis.md`. A dedicated bounded-context
workspace is selected over embedding adjustments in vault releases, extending
rule governance, or a generic admin table because it preserves the financial
authority boundary and gives refund/correction evidence a first-class review
flow without implying chain execution.

## Acceptance criteria

- Unit tests reject malformed UUIDv7 IDs, byte arrays, signed decimal strings,
  non-conserving deltas, unsupported status transitions, and unsafe numeric
  coercion.
- Component tests cover propose, replay, read, four-eyes approval, rejection,
  timeout, API error with support reference, and no `posted` control.
- Component tests prove the proposal form does not prefill settlement,
  financial, asset, or evidence values.
- Automated accessibility tests cover the panel at desktop, mobile, and RTL
  direction with keyboard-reachable controls and no WCAG violations detected
  by the repository gate.
- The real browser journey loads the workspace through NGINX and proves that
  server responses drive the displayed state.

## Consequences

The operator can perform the governed proposal/review workflow from the real
product surface with clear evidence and failure handling. The application gains
another API-dependent workspace and must maintain its contract tests. On-chain
posting, wallet signing, finalized chain evidence, and reconciliation remain
separate release gates and are intentionally not hidden behind this UI.
