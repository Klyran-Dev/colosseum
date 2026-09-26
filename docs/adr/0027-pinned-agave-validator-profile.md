# ADR-0027: Provide a pinned Agave validator Compose profile

- Status: Accepted
- Date: 2026-09-25

## Context

Solana integration and program tests must run against a reproducible local
ledger. Requiring a host-installed validator creates version drift, while
starting a validator for every UI/API edit wastes substantial resources.

## Decision

Provide an opt-in Docker Compose `solana` profile. It installs the stable
Agave 4.3.0 Linux release through its upstream installer artifact, verifies
the immutable SHA-256 digest, runs as a non-root user, persists only the
project-owned `solana-ledger` volume, exposes synthetic local RPC/faucet
ports, and has a health check. `make solana-up` is the documented entrypoint.

The default Compose path remains the API/UI/PostgreSQL feedback loop. The
profile is required for localnet program integration and settlement tests; it
must never be used as a production validator deployment template.

## Alternatives considered

- **Host-installed Agave/Anchor:** rejected because local version drift breaks
  reproducibility and clean-checkout onboarding.
- **Floating Solana Docker image:** rejected because `latest` images cannot
  prove the validator binary or protocol version.
- **Build Agave from source for every local run:** rejected for the default
  loop because build latency and resource use are excessive; source builds
  remain a separate release reproducibility track.

## Consequences

- The local validator profile currently targets the pinned x86_64 Linux release;
  Docker Desktop may use emulation on ARM hosts.
- Agave 4.3.0 requires Linux `io_uring` support for its accounts-directory
  path. Docker Desktop or CI runtimes that expose no usable `io_uring` syscall
  will build the image but cannot start this profile; they must run the
  validator on a compatible Linux host or use the separately governed legacy
  compatibility fixture when that track is added.
- The profile consumes significant CPU, memory, and disk, so it is not started
  by default.
- Program integration tests must declare their validator dependency and verify
  the reported binary/version before running.
