# Contributing

Thank you for contributing to the Solana Tax Protocol. This is financial infrastructure: correctness, security, determinism, and clear evidence are more important than the size of a change.

## Before opening a pull request

1. Read [AGENTS.md](AGENTS.md).
2. Read the relevant architecture and ADR documents.
3. Write or update the design, alternatives analysis, invariants, threat model, and latency/resource budget before implementation.
4. Add tests first. Money, tax, authority, replay, and concurrency behavior must have executable specifications.
5. Update API/IDL/schema compatibility notes and public documentation.
6. Confirm the change does not add a demo-only path, hard-coded financial behavior, secret, unbounded work, or misleading UX.

## Pull request expectations

Every pull request should explain:

- the user and operational problem;
- the selected design and alternatives rejected;
- bounded contexts and invariants affected;
- security, privacy, and failure-mode impact;
- latency, throughput, compute, and storage impact;
- test evidence, including negative paths;
- migration, rollback, and observability changes;
- documentation and ADR updates.

Do not report a feature as production-ready unless the relevant production-readiness gates are complete.

## Local development

Docker Compose is the required development boundary. Host-installed tools may be used for investigation, but a contribution must be reproducible from the repository’s pinned containers and lockfiles.

The reproducible checks are:

- `make bootstrap` for the full Docker development topology, including the
  real NGINX-served SPA, PostgreSQL migrations/fixtures, validator, indexer,
  route verifier, and local observability. It requires the pinned validator's
  runtime capabilities and fails closed if they are unavailable;
- `make check` for formatting, linting, Rust and web tests, type checking,
  production web build, security scanning, Compose configuration,
  observability configuration, and public-documentation contracts;
- `make integration-check` for the isolated Docker API, PostgreSQL, NATS,
  NGINX, authentication, quote, attestation, governance, and persistence
  paths;
- `make solana-conformance` for the explicit validator-backed gate. It
  requires a Linux runtime that exposes the capabilities required by the
  pinned Agave validator and must not be replaced by a simulated transaction;
- `make public-docs-check` for the public repository contract.

Changes affecting money movement, tax arithmetic, authorization, replay,
serialization, migrations, or concurrency must add or update executable tests
before implementation and must identify the relevant ADR and readiness gate.

## Code and documentation style

- Prefer small, cohesive changes with explicit ownership.
- Keep domain logic independent of infrastructure.
- Use checked integer arithmetic and canonical serialization for financial values.
- Keep errors actionable and observable; never silently fall back.
- Write documentation for a public reader who has not seen the issue or conversation.
- Avoid unsupported claims about tax coverage, compliance, security, performance, or production status.

## Commit and review guidance

Use clear, imperative commit subjects. Keep unrelated formatting or dependency upgrades separate. Reviewers should be able to reproduce the claimed behavior and understand why the chosen design is safe.
