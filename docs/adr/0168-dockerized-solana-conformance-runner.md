# ADR-0168: Dockerized Solana conformance runner

## Status

Accepted.

## Context

ADR-0065 makes the pinned Agave validator and the deployed SBF artifact the
authoritative localnet settlement gate. The previous runner started the
validator in Compose but invoked the TypeScript/Vitest client with host
`pnpm`. That made a clean checkout dependent on a host Node.js toolchain and
allowed the conformance command to drift from the pinned Docker runtime used
by the web build and CI.

The validator remains an independent Linux capability gate. This decision does
not weaken its `io_uring` requirement, replace real RPC evidence with a mock,
or mark a failed validator startup as a pass.

## Decision

Add a dedicated `solana-conformance` Compose service using a pinned Node.js
24.21.0 image digest and pnpm 12.3.4. The image installs the locked web
dependencies at build time, runs as the unprivileged `node` user, and invokes
only `src/solana.localnet.test.ts` against the Compose `validator` hostname.
`scripts/run_solana_conformance.sh` starts the validator first, waits for its
health endpoint, and then runs the client through this container with
`--no-deps`. The runner shares the validator's Compose network but cannot
change validator state, mount the host Docker socket, or access private keys.

The conformance command therefore requires Docker/Compose and the pinned
images, but no host Rust, Node.js, pnpm, Solana CLI, or NGINX installation.

## Alternatives considered

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Host pnpm invocation | Fast to implement | Violates clean-checkout Docker mandate and permits runtime drift | Rejected |
| Reuse the browser Playwright image | Already contains pnpm and web dependencies | Pulls a browser stack into a non-browser chain gate and increases attack/resource surface | Rejected |
| `solana-program-test` in Rust | Fast and deterministic | Does not exercise the external Agave RPC boundary or exact TypeScript serialization | Rejected as release evidence |
| **Dedicated pinned Node conformance image** | Minimal runtime, reproducible lockfile, non-root, same Compose network, no host toolchain | One additional image and build cache | **Selected** |

## Invariants and failure policy

- the validator must become healthy before the runner starts;
- the runner must use `SOLANA_RPC_URL=http://validator:8899` and
  `SOLANA_LOCALNET_TESTS=true`;
- a missing image, failed dependency install, Vitest failure, RPC failure, or
  validator failure returns non-zero and retains evidence;
- the runner is read-only, uses a temporary writable home, drops capabilities,
  and runs as a non-root user; and
- local Docker success remains localnet evidence only; Linux CI or an
  equivalent Linux runner must still provide the authoritative Agave 4.3.0
  conformance result.

## Verification

- `docker compose --profile solana config --quiet` validates the service;
- `make compose-config` covers all profiles and pinned image references;
- `make solana-conformance` runs the test client inside the image;
- CI's Ubuntu `solana-conformance` job uses the same script and retains the
  validator, SBF, Compose, and Vitest evidence bundle; and
- the existing localnet test remains the real transaction contract for legacy
  SPL, extension-free Token-2022, duplicate receipts, Split Plan, policy
  revocation, and unsupported extensions.

## Consequences

The local conformance gate now honors the Docker-only bootstrap contract and
has one reproducible JavaScript runtime. The image adds build time and cache
storage, while the validator's Linux `io_uring` capability remains a genuine
environment prerequisite rather than something the runner can mask.
