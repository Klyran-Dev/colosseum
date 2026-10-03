# ADR-0267: Honest runtime and account context in the product shell

## Status

Accepted. This decision removes fixed tenant, operator, environment, and
network labels from the browser shell. It is a display-integrity boundary; it
does not authorize an action or replace server-side tenant checks.

## Context

The shell is shared by local, staging, and production deployments and may be
used by many accounts. Fixed labels such as `Acme Treasury`, `Staging`,
`Test network`, and operator initials can look credible while describing a
different deployment or account. That is unacceptable for a product that
prepares or reviews money movement.

The static React/NGINX application cannot discover its deployment safely from
an RPC URL or from a browser-supplied tenant value. Account identity must come
from the verified server capability snapshot. Deployment and Solana cluster
labels are immutable, non-authoritative build context supplied by the image
build; an absent or invalid value must be visible as unconfigured rather than
silently defaulting to staging or a test network.

## Decision

1. Remove all hard-coded account, operator, environment, and network labels
   from the shell.
2. Derive account context only from the server-owned capability snapshot. Until
   the snapshot is available, show `Sign-in required`; after it is available,
   show a bounded non-secret account reference and keep the full identifier in
   the technical disclosure area.
3. Derive deployment and Solana-cluster badges from explicit build variables:
   `VITE_DEPLOYMENT_ENVIRONMENT` and `VITE_SOLANA_NETWORK`. Accepted values are
   `local`, `staging`, `production` and `localnet`, `devnet`, `testnet`,
   `mainnet-beta`, respectively.
4. Invalid or missing build context renders `Not configured` with a visible
   warning. It never falls back to `staging`, `devnet`, or another plausible
   value. The context is display-only and cannot enable an action.
5. The Docker edge build supplies explicit local defaults for the local
   Compose profile. A staging or production release build must override both
   values and retain them in its immutable release manifest.
6. A future tenant-directory capability may add a server-owned display name;
   this ADR does not treat an unverified browser label or a caller-supplied
   tenant name as authoritative.

## Alternatives considered

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Keep fixed demo labels | No implementation work | Can misrepresent account, environment, and chain; unsafe for financial review | Rejected |
| Read labels from browser storage or query parameters | Easy to customize | Caller-controlled and can create a false authority boundary | Rejected |
| Infer environment from RPC URL or hostname | Avoids build configuration | Proxies and failover make inference unreliable; URLs are not authority | Rejected |
| Fetch deployment context from a new runtime endpoint | Central server ownership | Adds a request and availability dependency to static shell rendering | Deferred for a future signed deployment-manifest contract |
| **Immutable build context plus verified account snapshot** | Fast, deterministic, honest when absent, compatible with static NGINX delivery | Release pipeline must pass and record context | **Selected** |

## Invariants and failure behavior

- The browser never accepts a tenant identifier from a URL, form, local
  storage, or environment variable as authorization input.
- A missing capability snapshot cannot produce a named account or an allowed
  protected action.
- A missing or invalid deployment label is rendered as `Not configured` and
  emits no misleading network or production claim.
- Deployment and network labels do not select RPC endpoints, signing keys,
  token mints, or settlement routes.
- Account identifiers are bounded and displayed as opaque references; bearer
  tokens, private keys, and raw tax evidence never appear in the shell.

## Verification

- Unit tests cover every accepted value and invalid/missing build value.
- Shell tests prove fixed labels are absent, verified account context is
  displayed only after a server snapshot, and unconfigured context is visible.
- The real-browser suite checks the badges and narrow responsive shell.
- Docker build and public-repository checks prove local context is explicit and
  release documentation does not claim a production environment by default.

## Consequences

Operators get an honest deployment boundary and cannot mistake synthetic local
fixtures for a live account. The shell is slightly less visually polished
when release configuration is missing, by design: an explicit warning is safer
than a plausible lie. Production rollout still requires a release pipeline to
provide and record its environment and cluster values.
