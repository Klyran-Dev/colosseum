# ADR-0524: Bind Hosted Browser Wallets to a Verified Solana Network

- Status: Accepted
- Date: 2026-10-13
- Owners: Solana settlement and release engineering
- Related: ADR-0099, ADR-0139, ADR-0200, ADR-0520

## Context

The GCE/Cloudflare hackathon deployment is intended to use Solana Devnet, but
the repository only declared the settlement program for localnet. The hosted
SPA also did not receive `VITE_SOLANA_RPC_URL` during its Docker build, so a
connected wallet could not construct a transaction that reached the selected
cluster. A network label without an RPC endpoint or finalized program evidence
can make the UI appear configured while settlement is impossible or pointed at
the wrong cluster.

## Decision

1. Hosted staging/hackathon releases must declare both
   `VITE_SOLANA_NETWORK` and `VITE_SOLANA_RPC_URL`.
2. Non-local browser RPC endpoints must use HTTPS and must not be loopback,
   Docker, validator, or other local-development addresses. The RPC URL is
   public build-time configuration; provider credentials must not be embedded
   in the SPA.
3. The Docker edge build passes the explicit RPC URL into Vite. Wallet signing
   continues to fail closed when the URL is absent or invalid.
4. `Anchor.toml` declares the canonical settlement program for Devnet as well
   as localnet. A release must separately prove at finalized commitment that
   the program account is executable and owned by the upgradeable BPF loader.
5. The API, indexer, and route verifier retain their own server-side RPC
   endpoint lists. Browser configuration never becomes authority for merchant
   accounts, mints, vaults, routes, program IDs, or settlement amounts.
6. No GCE, Cloudflare, API, or browser component generates or stores user,
   governance, upgrade, or vault-release private keys.

## Alternatives considered

| Alternative | Benefit | Risk | Decision |
| --- | --- | --- | --- |
| Infer the RPC from the network label | Smallest browser configuration | A label cannot select a provider, and signing can silently remain unavailable | Rejected |
| Proxy wallet RPC through the API | Keeps the endpoint out of the bundle | Adds a hot-path service dependency and makes the API an RPC authority | Rejected |
| Hard-code a public Devnet RPC in source | Easy local setup | Provider coupling, accidental mainnet reuse, and no release-specific evidence | Rejected |
| **Explicit build-time RPC plus non-local preflight and finalized program verification** | Auditable cluster binding, usable wallet signing, and fail-closed release behavior | Requires deployment configuration and a separate chain check | **Selected** |

## Invariants and acceptance criteria

- A staging or production preflight fails when either the Solana network or
  browser RPC URL is missing, invalid, local, or insecure.
- The built SPA contains the configured public RPC URL and network label.
- Devnet and localnet use the same canonical program ID unless a separately
  approved program-ID migration is recorded.
- A deployment is not called Solana-ready until finalized RPC evidence proves
  the canonical program account is executable and owned by the upgradeable BPF
  loader, and the deployed artifact digest is retained.
- The browser never receives private keys or server-owned account topology.

## Rollback and revisit conditions

Rollback is a static-asset/service rollback to the previously verified
network-matched release. Do not switch the network label or RPC endpoint alone
to recover a failed deployment. Revisit this decision if the release platform
provides a governed, equivalent public-RPC configuration service or if the
settlement program uses a network-specific program ID.
