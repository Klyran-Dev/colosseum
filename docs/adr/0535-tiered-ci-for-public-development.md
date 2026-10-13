# ADR-0535: Tiered CI for public development

- Status: Accepted
- Date: 2026-10-13
- Deciders: Principal Low-Latency Architecture, Application, and SRE
- Scope: GitHub Actions pull-request and main-branch validation

## Context

The repository is public and is still under active development. The default CI
workflow currently schedules a large set of Docker-backed load, recovery,
validator, failover, and operational-evidence drills on every push and pull
request. These checks are valuable release evidence, but several are sensitive
to runner architecture, Docker availability, timing, or shared-resource
contention. A red exploratory drill can obscure whether a change is safe to
review, while removing the checks would violate the repository's fail-closed
release contract.

The project therefore needs two different signals: a fast, deterministic
development signal for ordinary review and an explicit extended evidence
signal before a release candidate or when an engineer is investigating
operational behavior.

## Decision

Keep the normal `CI` workflow strict for source, authorization, security,
schema, Docker/Compose, API integration, accessibility, browser, and core
settlement correctness checks. Gate timing-sensitive operational drills behind
the `workflow_dispatch` input `run_extended`, which defaults to `false`.

The extended jobs remain in the repository, retain their failure semantics,
and can be run manually with `run_extended=true`. The dedicated Solana
conformance workflow and the release-candidate workflow remain authoritative
for validator-backed and release evidence; this change does not weaken either
workflow. Skipped extended jobs are not described as passed evidence.

The extended set is:

- Tax Engine, quote, document-quote, and persisted-rule load evidence;
- OTLP and Alertmanager delivery drills;
- validator-backed conformance;
- indexer replay/retry recovery drills;
- refund lease, funding, execute-contention, and proof drills;
- PostgreSQL backup, PITR, contention, pool, and index-plan drills;
- vault reconciliation and multi-provider RPC failover; and
- migration integrity and rollback rehearsals.

Core API authentication rotation, Docker quote/idempotency integration,
transaction-bound refund proof integration, migration bootstrap, and the
adversarial settlement gates remain required on ordinary CI.

## Alternatives considered

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Delete the extended jobs | Fastest and quietest PR signal | Loses recovery and production-evidence coverage; violates release-readiness mandates | Rejected |
| Mark every extended job `continue-on-error` | Evidence still runs on every change | Red failures become easy to ignore and branch protection cannot distinguish an unsafe result from an allowed experiment | Rejected |
| Keep every job required on every push | Strongest immediate signal | High runner cost and timing sensitivity obscure core review failures during active development | Rejected |
| **Manual `run_extended` gate with strict execution when enabled** | Deterministic review signal, retained evidence, explicit operator intent, no weakened release gates | Engineers must request extended evidence before release or operational review | **Selected** |

## Invariants and threat model

- No extended job is deleted or converted to a successful result after a
  failure.
- Release-candidate and dedicated conformance workflows still execute their
  authoritative checks without this development shortcut.
- Security, authentication, authorization, schema, settlement, and core
  integration checks cannot be bypassed by omitting the manual input.
- The input is only a scheduling control; it is not an authorization,
  deployment, signer, or settlement control.
- Workflow changes remain pinned and are validated by the repository's static
  workflow contracts.

## Verification and operational policy

The workflow contract verifies the input, the list of gated jobs, and that the
release/conformance workflows retain their required commands. Before a release
candidate, run the release-candidate workflow and retain its evidence bundle;
for focused operational testing, dispatch `CI` with `run_extended=true`.

