# ADR-0307: Project-scoped local reset for persistent Docker state

## Status

Accepted — implementation is an explicit local-development command.

## Context

The default Compose topology uses named PostgreSQL and NATS volumes so that a
normal restart does not discard local evidence. Migration files are immutable
after application; the migration runner correctly fails closed when an
existing volume contains a checksum from a different source revision. That is
the safe production behavior, but it leaves a developer who intentionally
changed or replaced a local checkout with a stale synthetic volume without a
clear recovery path.

`make down` must remain non-destructive. A reset operation must never delete a
host directory, an arbitrary Docker volume, or another Compose project.

## Decision

Add `make reset-local`, backed by `scripts/reset_local.sh`. The command:

1. validates the optional `COMPOSE_PROJECT_NAME` using the same restricted
   Compose-project grammar as bootstrap;
2. prints an explicit warning naming the target project;
3. runs `docker compose -p <validated-project> down --volumes --remove-orphans`;
4. removes only volumes and containers owned by that exact Compose project;
5. leaves the source tree and all other Docker projects untouched.

The command does not automatically bootstrap, migrate, or remove images. The
operator must explicitly run `make bootstrap` or `docker compose up --build`
after the reset. CI and production deployment paths never call this command.

## Alternatives considered

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Make `make down` remove volumes | Simple recovery | Surprising data loss on every shutdown; violates local evidence continuity | Rejected |
| Ask operators to run raw Docker commands | No repository code | Easy to target the wrong project or broad volume set; not reproducible | Rejected |
| Automatically repair migration checksums | Keeps the volume running | Hides source/schema drift and weakens the fail-closed migration boundary | Rejected |
| **Explicit project-scoped reset command** | Recoverable, auditable, bounded, and easy to discover | Intentionally destructive to that project's synthetic local state | **Selected** |

## Invariants and safety

- The default target is the repository's declared local Compose project,
  `solana-tax-protocol-local`, not an inferred host-wide scope.
- A project name containing unsupported characters is rejected before Docker is
  invoked.
- The command uses Compose's project-scoped volume ownership rather than
  globs, shell expansion, or host-path deletion.
- `make down` remains non-destructive.
- No staging or production environment can invoke the command through the
  repository's deployment workflow.

## Verification

- A shell contract checks validation, warning, exact project-scoped Compose
  invocation, and non-use of broad deletion commands.
- `sh -n` validates the script syntax.
- A fresh Compose bootstrap proves the post-reset target can apply all current
  migrations and start the API/NGINX path.
