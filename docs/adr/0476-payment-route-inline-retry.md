# ADR-0476: Keep payment-route recovery in the checkout surface

## Status

Accepted

## Context

The payment-review workspace loaded the server-approved settlement route when
the authenticated checkout mounted. If that read failed, the route selector
became unavailable and the operator had no recovery control; changing unrelated
checkout state or reloading the page was the only way to try again. This is a
high-impact failure because route selection is a prerequisite for compiling a
wallet-bound settlement artifact.

## Decision

Add an explicit localized retry action to the route error alert. Retrying
increments a bounded client reload key, re-runs the existing authenticated
route read, clears stale route state, and leaves all settlement compilation and
wallet-signing controls unavailable until a verified route is loaded. The
split-plan loader keeps its separate retry lifecycle.

## Alternatives considered

- Require a full page reload: rejected because it discards the operator's
  current quote and wallet context.
- Retry automatically: rejected because route reads are an authority boundary
  and automatic retries can amplify an outage.
- Let the operator enter route accounts manually: rejected because the route
  registry is server-owned and browser input must not become an authority
  boundary.

## Consequences

- Route failures have an immediate recovery action without weakening the
  server-owned route boundary.
- The quote, wallet, and split-plan context remain intact during recovery.
- Compilation remains fail-closed until route evidence is available.

## Verification

- Checkout UI test covers a failed route read followed by an explicit retry.
- Frontend test suite, typecheck, `git diff --check`, and Docker browser gate.
