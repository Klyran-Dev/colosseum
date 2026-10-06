# ADR-0329: Approval-bound route-verification backfill mode

- Status: Accepted
- Date: 2026-10-04
- Deciders: Principal Low-Latency Blockchain Architecture, Settlement Domain, SRE
- Scope: route-verifier operator execution and route-verification evidence

## Context

ADR-0325 requires non-local settlement routes to be suspended and re-verified
when typed `AssetConfig` evidence is missing. ADR-0328 records scheduled and
backfill verification runs, but the route-verifier daemon currently exposes
only its recurring scheduled loop. A runbook instruction without an executable
approval-bound entry point is not sufficient for a production backfill.

The backfill must be deliberately invoked, bounded to one candidate batch,
auditable, and unable to alter the recurring service's behavior. It must not
become a second settlement authority or add work to quote, compile,
wallet-signing, or submit paths.

## Decision

Add an explicit `ROUTE_VERIFIER_MODE=backfill` one-shot mode. It requires a
UUIDv7 `ROUTE_VERIFIER_BACKFILL_APPROVAL_ID`; the mode is rejected when the
approval reference is absent, malformed, or supplied in daemon mode. The
one-shot process verifies one bounded settlement-route batch, persists a
`backfill` run with the approval ID, emits its summary, and exits. It does not
run refund-funding or revenue-destination loops and does not bind the health
listener.

The default mode remains `daemon` and creates only `scheduled` run records.
Backfill invocation is still subject to the existing environment, provider,
batch, deadline, finalized-evidence, and route-lifecycle bounds. A clean empty
batch is successful but creates no run row; a per-route failure completes as
`completed_with_failures`; storage or process failure is non-zero and never
reported as successful.

Persist the approval ID only on `backfill` run rows. Scheduled rows must have
no approval ID. The approval reference identifies an external change record;
it is not a credential and never authorizes token movement.

## Alternatives considered

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Run the daemon and rely on operator timing | No new binary mode | Ambiguous scope, also runs unrelated verifiers, weak evidence | Rejected |
| Add an unauthenticated `run once` flag | Simple invocation | Cannot prove approved backfill intent; unsafe in production | Rejected |
| Build a separate backfill service | Isolation | Duplicates provider, lifecycle, and evidence logic; more deployment drift | Rejected |
| **Approval-bound one-shot mode in the existing verifier** | Reuses proven bounded path, explicit scope, durable approval evidence, no hot-path impact | Requires operator change reference and migration | **Selected** |

## Invariants and failure behavior

- Daemon mode always uses `scheduled` and a null approval ID.
- Backfill mode always uses `backfill` and a valid UUIDv7 approval ID.
- A backfill process performs at most one bounded settlement-route batch.
- Backfill mode never executes refund-funding or revenue-destination loops.
- Missing, malformed, or mode-inconsistent approval configuration fails before
  database connections, RPC transports, or listeners are created.
- A route-verification run is not authority for funds; finalized Solana
  evidence and governed route activation remain authoritative.
- A process crash or persistence failure cannot produce a completed run.

## Verification

- Add configuration unit tests for default daemon mode, approval binding,
  malformed UUIDv7 values, and mode mismatch.
- Add storage and direct-SQL tests for scheduled-null/backfill-required
  approval invariants and immutable terminal rows.
- Add an executable one-shot contract that proves the backfill mode invokes
  only one settlement-route pass and exits without starting a health listener.
- Retain the approval ID, run ID, exact route evidence, finalized slots, and
  independent review in the release bundle before paid-state enablement.

## References

- [ADR-0325: Typed asset-admission projection](0325-typed-asset-admission-projection.md)
- [ADR-0328: Durable route-verification run evidence](0328-route-verification-run-evidence.md)
- [Settlement-route verification runbook](../runbooks/settlement-route-verification.md)
