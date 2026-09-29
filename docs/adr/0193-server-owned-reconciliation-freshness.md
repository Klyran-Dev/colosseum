# ADR-0193: Server-owned reconciliation freshness contract

- Status: Accepted
- Date: 2026-09-29
- Deciders: Principal Low-Latency Architecture, Finance/Tax, Security, and SRE
- Scope: freshness semantics for the ADR-0128 vault-reconciliation read model

## Context

ADR-0127 schedules finalized vault observations every 30 seconds, but the
ADR-0128 API currently exposes only the latest run status. A run can therefore
remain matched in the read model while the indexer has stopped, the provider
is unavailable, or the scheduler is delayed. Rendering that row as healthy
would be an unsafe stale-evidence interpretation.

Freshness must be decided by the server using its clock and persisted evidence.
The browser must not compare timestamps with a device clock, accept a
caller-provided threshold, or infer current chain state from a previous
response.

## Decision

Add a server-owned stale boolean to every reconciliation run returned by the
tenant-scoped API. The API and storage adapters use the fixed policy
VAULT_RECONCILIATION_STALE_AFTER_SECONDS = 90, equal to three default
30-second scheduler periods. The policy is versioned in this ADR and is not
caller-configurable.

The database computes freshness with its UTC clock at read time:

- stale=false means the observation is within the bounded evidence window;
- stale=true means the observation is older than 90 seconds;
- no latest run remains No observation yet, which is not a matched state.

The browser treats stale observations as an operational stop condition,
regardless of whether the underlying run status is matched, mismatch, or
error. It displays the stale state and does not claim that balances are
currently reconciled. Run history retains the server-provided stale value for
the read timestamp; historical rows are evidence, not a live health feed.

The policy is intentionally conservative. A future deployment with a
different scheduler period must create a superseding ADR and change the
versioned API contract rather than silently changing the threshold.

## Alternatives considered

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Browser compares created_at with its local clock | No API change | Clock skew, offline tabs, timezone errors, and caller-controlled freshness interpretation | Rejected |
| Client supplies a freshness threshold | Flexible operations | Allows each caller to redefine financial evidence validity and creates inconsistent status | Rejected |
| Indexer mutates old rows to stale | Simple UI model | Violates append-only evidence and loses the distinction between observed status and read-time freshness | Rejected |
| API returns timestamps only and leaves status to operators | Minimal schema | Human interpretation is inconsistent and unsafe in an operational control surface | Rejected |
| **Database-clock server-owned stale boolean with fixed policy** | Deterministic, tenant-safe, append-only compatible, and explicit in the UI contract | Adds one bounded computed field and requires a superseding ADR for policy changes | **Selected** |

## Invariants and budgets

- Freshness is evaluated against the database UTC clock, never a browser clock.
- The stale threshold is 90 seconds and cannot be supplied in query
  parameters or request bodies.
- stale=true never renders as matched, settled, paid, or safe to release.
- No write, RPC request, transaction submission, or scheduler task is added to
  the read path.
- The read query remains bounded to 64 targets or runs and uses the existing
  target/run indexes.
- The API remains tenant-scoped and exact-decimal for financial values.

## Verification and rollout

- Add the field to Rust storage/API serialization and the TypeScript parser.
- Add storage SQL-contract tests for database-clock freshness and parser tests
  for the required boolean.
- Add browser evidence for a stale row and retain mismatch/unauthorized
  coverage under ADR-0192.
- Update the operator runbook and readiness matrix. Live-provider timing,
  production scheduler SLO, and incident-response evidence remain separate
  release gates.
