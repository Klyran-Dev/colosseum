# ADR-0210: Docker full-topology bootstrap

- Status: Accepted
- Date: 2026-09-30
- Owners: Platform / SRE
- Scope: Local developer and CI bootstrap orchestration

## Context

The repository already provides pinned Docker images, Compose health checks,
PostgreSQL migrations and fixtures, the React/Vite application behind NGINX,
the validator profile, the indexer, the route verifier, and local
observability. Starting only the default Compose services does not explicitly
prove that the complete local topology is healthy, and starting every profile
would also launch one-shot conformance or browser-test jobs unintentionally.

A production-grade development boundary therefore needs one documented command
that selects the long-running services, waits on their declared health
contracts, and fails closed when a required capability such as the pinned
Agave validator's Linux `io_uring` support is unavailable.

## Decision

Add `make bootstrap` backed by `scripts/bootstrap_local.sh`. The script uses
the repository's Compose file and starts the explicitly bounded long-running
topology:

- PostgreSQL and the migration/fixture runner;
- API, worker, NATS, Redis, and NGINX edge;
- indexer and finalized settlement-route verifier;
- OpenTelemetry Collector, Prometheus, Alertmanager, Grafana, and the local
  alert receiver; and
- the pinned Agave validator with the Docker-built SBF artifact.

Before Compose startup it invokes the pinned Dockerized SBF builder, so a
clean checkout does not depend on a host Solana toolchain or an artifact left
by a previous command.

It uses Compose `up --wait --wait-timeout` so health checks remain the service
readiness authority. The one-shot `solana-conformance` and `web-e2e` services
are intentionally excluded; they remain explicit release/test commands. A
`BOOTSTRAP_DRY_RUN=true` mode exposes the resolved command for contract tests
without contacting Docker. On failure the script preserves the named Compose
project for inspection and returns non-zero; it never changes the validator to
a mock or downgrades a health failure.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| `docker compose up` with default services | Rejected | Omits required profiles and gives no explicit full-topology contract. |
| Enable every Compose profile | Rejected | Starts one-shot conformance and browser jobs as if they were daemons. |
| A host shell supervisor with per-service polling | Rejected | Duplicates Compose health semantics and introduces host-toolchain drift. |
| Kubernetes-in-Docker | Rejected | Adds a second local orchestrator and weakens clean-checkout reproducibility. |
| **Explicit long-running services with Compose `--wait`** | **Selected** | Reuses pinned images, health checks, dependency ordering, and Docker-only development while keeping one-shot gates explicit. |

## Invariants and failure policy

- The command must select every required long-running service exactly once.
- `solana-conformance` and `web-e2e` must not be started by bootstrap.
- All required services must be health-checked by Compose before success is
  reported.
- A validator capability failure is a typed non-zero failure; it cannot be
  converted to a simulated or skipped path.
- The project name is bounded and overridable through
  `COMPOSE_PROJECT_NAME`, so cleanup and inspection remain scoped to this
  repository's named resources.
- No host Rust, Node, Solana CLI, PostgreSQL, Redis, NATS, or NGINX install is
  required.

## Consequences

`make bootstrap` becomes the canonical local full-topology entry point. It
improves onboarding and makes the required local architecture executable, at
the cost of starting more containers and requiring a Linux-capable Docker
runtime for validator-backed startup. It does not close the independent
Linux/amd64 validator conformance, production SLO, live-provider, security,
or legal release gates.
