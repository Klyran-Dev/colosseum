# ADR-0305: One-command Tax Engine local gate orchestration

## Status

Accepted — implemented as an explicit Make target and contract.

## Context

The repository has deliberately separate gates for Rust/frontend/contracts,
authenticated Docker integration, real-browser UI journeys, and validator-backed
Solana conformance. Separate commands make isolation useful, but they also make
it easy for a reviewer or release operator to run only the fast checks and
mistake that result for end-to-end evidence.

The gates have different infrastructure lifecycles and must not be merged into
one opaque script. The validator gate also requires Linux/amd64 Docker with the
Agave `io_uring` capability; a local host that lacks it must fail with typed
evidence rather than silently skipping conformance.

## Decision

Provide `make tax-engine-local-check` as the canonical ordered orchestration:

1. `make check` — Rust, frontend, static contracts, security, Compose,
   migrations, and runtime contracts.
2. `make integration-check` — authenticated multi-replica Docker persistence,
   outbox, reconciliation, and failure-path evidence.
3. `make web-browser-check` — pinned NGINX/Playwright browser journeys and
   accessibility assertions.
4. `make solana-conformance` — pinned SBF build and real validator transactions.

Each underlying command remains independently runnable and owns its own cleanup
and evidence. The orchestration stops on the first failure, does not convert a
validator capability failure into a skip, and does not claim staging, mainnet,
legal, custody, or independent-audit readiness.

## Alternatives considered

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Keep only separate commands | Good isolation and faster targeted feedback | Reviewers can omit a required layer; no canonical end-to-end command | Rejected as the sole interface |
| One opaque shell script | Simple invocation | Hides ownership, cleanup, evidence, and failure boundaries | Rejected |
| Run every layer in parallel | Lower wall-clock time | Shared Docker resources and evidence races; weak failure attribution | Rejected |
| **Ordered Make orchestration over independently bounded gates** | Reproducible, fail-fast, reviewable, preserves gate ownership | Longer wall-clock time and requires Linux for validator conformance | **Selected** |

## Invariants and failure policy

- The four gates run in the documented order and all are required.
- A non-zero result stops the orchestration and propagates the failure.
- No gate is replaced by a mock, a skipped test, or a host-only shortcut.
- The command is a test/evidence gate, not a deployment or production approval.
- Local synthetic fixtures and Docker evidence remain explicitly labeled.

## Verification

- A Makefile contract asserts all four commands and their order.
- `make -n tax-engine-local-check` provides a no-side-effect review of the exact
  sequence.
- The individual commands remain the authoritative evidence producers.
- The validator step must be executed on the retained Linux/amd64 release
  runner before production enablement.
