# ADR-0477: Keep document-checkout route recovery in the payment surface

## Status

Accepted

## Context

Document checkout loads the server-approved settlement route before invoice
evidence can be checked or a wallet-bound payment can be prepared. When that
read failed, the route selector became unavailable and the operator had no
local recovery action; a page reload was the only practical retry and could
discard the current checkout context.

## Decision

Add an explicit localized retry action to the document-checkout route error
alert. Retrying increments a bounded client reload key, re-runs the existing
authenticated route read, and keeps evidence verification, compilation, and
wallet signing unavailable until a verified route is loaded. The route
registry remains server-owned and the existing fail-closed behavior is
unchanged.

## Alternatives considered

- Require a full page reload: rejected because it discards the operator's
  document reference and wallet context.
- Retry automatically: rejected because route reads are an authority boundary
  and automatic retries can amplify an outage.
- Let the operator enter route accounts manually: rejected because browser
  input must not become an authority boundary.

## Consequences

- A transient route-read failure has an immediate, understandable recovery
  action in the payment surface.
- Document evidence and wallet context remain intact during recovery.
- No payment can be prepared or signed until the server returns a valid route.

## Verification

- Document-checkout UI test covers a failed route read followed by an explicit
  retry.
- Frontend test suite, typecheck, `git diff --check`, and Docker browser gate.
