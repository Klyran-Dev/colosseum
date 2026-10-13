# ADR-0479: Keep treasury route recovery in the release surface

## Status

Accepted

## Context

The treasury release workspace loads a server-approved payment route before a
release request can be created. A failed route read left the selector disabled
and displayed a warning, but provided no recovery action. The operator had to
reload the page, which discarded the request context and made the failure look
like a permanent configuration problem.

## Decision

Add an explicit localized retry action to the treasury route warning. The
action increments a bounded client reload key and re-runs the existing
authenticated route read. The release form remains unavailable until a valid,
server-owned route is returned; no route accounts can be entered manually and
no automatic retry is introduced.

## Alternatives considered

- Require a full page reload: rejected because it discards the release form
  context.
- Retry automatically: rejected because route reads are an authority boundary
  and retry storms can amplify an outage.
- Allow manual route account entry: rejected because browser input must not
  become an authority boundary.

## Consequences

- Treasury route failures have an immediate recovery action.
- Route verification and fail-closed release behavior are unchanged.
- Operators retain their request context while recovering from transient
  service failures.

## Verification

- Vault-release UI test covers a failed route read followed by an explicit
  retry.
- Frontend test suite, typecheck, `git diff --check`, and Docker browser gate.
