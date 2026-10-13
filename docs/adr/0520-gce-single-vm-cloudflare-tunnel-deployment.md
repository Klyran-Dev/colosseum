# ADR-0520: GCE single-VM deployment with Cloudflare Tunnel

## Status

Accepted for the controlled hackathon deployment path. This is not a high-availability or mainnet-production approval.

## Problem

The repository needs a reproducible public deployment for the Colosseum hackathon without exposing PostgreSQL, NATS, Rust service ports, or database credentials to the internet. The initial deployment target is Google Cloud project `project-7dfff051-32d5-4721-88d` and the public application hostname is `colosseum.klyran.com`.

## Decision

Run the pinned Docker Compose application on one Google Compute Engine `e2-standard-4` VM with a 100 GiB balanced Persistent Disk. PostgreSQL remains in the same Compose project for the hackathon path and is persisted on a named volume. All non-edge host ports bind to loopback. A token-based Cloudflare Tunnel connector runs on the VM and routes:

- `colosseum.klyran.com` → `http://127.0.0.1:8088` (NGINX/static SPA and `/api/`)
- `auth.klyran.com` → `http://127.0.0.1:18085` (email-OTP service)

The GCE firewall exposes no application or database port. SSH is restricted to the operator's source range or accessed through the provider's identity-aware mechanism. Runtime secrets are injected on the VM with mode `0600`; they are never committed, copied into the frontend, or placed in chat.

## Scope and non-goals

This decision covers the hackathon public web deployment, Docker startup, SMTP-backed OTP, and Cloudflare ingress. It does not approve mainnet settlement, production legal/tax compliance, HA, Cloud SQL migration, multi-zone failover, or a Solana validator on the VM.

## Invariants

1. PostgreSQL, NATS, API, and auth ports are loopback-only from the VM host.
2. The Cloudflare Tunnel is outbound-only; the origin has no public application ingress.
3. `AUTH_ENV`, `API_ENV`, and `TAX_ENGINE_ENV` are explicit and may not retain local defaults.
4. Simulation mode and insecure RPC are disabled outside local development.
5. The frontend contains only public build-time configuration; database, SMTP, signing, evidence, and service tokens remain server-side.
6. The first administrator enrollment is constrained by `AUTH_INITIAL_ADMIN_EMAIL` and that setting is removed after successful enrollment.
7. PostgreSQL backups are scheduled and restore evidence is retained before the deployment is called submission-ready.

## Alternatives

| Option | Benefits | Costs/risks | Decision |
| --- | --- | --- | --- |
| GCE VM with PostgreSQL in Compose | Lowest setup complexity, preserves the tested stack, predictable cost | Single failure domain, operator-managed database backup/restore | **Selected for hackathon only** |
| GCE Rust VM plus Cloud SQL PostgreSQL | Database backups, patching, monitoring, and HA options are managed | Higher cost, private connectivity and migration work, more moving parts | Reserved for production hardening |
| Cloud Run for the Rust services | Autoscaling and managed deployment | Compose/NATS/PostgreSQL topology does not map directly; connection and background-worker changes required | Rejected for this deployment |
| Public GCE IP plus direct NGINX exposure | Simple DNS | Expands attack surface and exposes origin identity | Rejected |
| Cloudflare Pages plus public API VM | Managed static delivery | Adds a separate frontend release and origin/API CORS boundary | Deferred; current NGINX artifact is the tested product surface |

## Budgets

- VM: 4 vCPU / 16 GiB RAM; target sustained application utilization below 70% CPU and 75% memory.
- Disk: 100 GiB balanced Persistent Disk; PostgreSQL and evidence retention are capped and monitored.
- Public request path: Cloudflare edge → Tunnel → NGINX → Rust service; no direct origin path.
- Initial deployment: low traffic/hackathon workload only. Load and failure evidence are required before increasing scope.

## Threat model and failure behavior

- A leaked tunnel token is treated as an origin-access credential: rotate it in Cloudflare and restart the connector.
- A leaked SMTP or signing secret requires immediate rotation; logs and frontend bundles must be checked for disclosure.
- If PostgreSQL is unavailable, Compose health ordering prevents API/auth startup from claiming readiness.
- If the tunnel has zero healthy connectors, Cloudflare returns an origin error; the database remains private.
- If email delivery is unavailable, OTP requests fail with an explicit delivery error and are not treated as successful authentication.

## Rollout and rollback

1. Validate the exact source revision, Compose configuration, production preflight, and image digests.
2. Create the VM and encrypted persistent disk; apply firewall and service-account least privilege.
3. Install Docker and `cloudflared`; place the runtime environment with `0600` permissions.
4. Start migrations and services; verify health endpoints locally from the VM.
5. Start the tunnel; create/verify both public hostnames and HTTPS behavior.
6. Enroll the first administrator, remove the initial-admin setting, and rotate temporary bootstrap material.
7. Roll back by stopping the Compose project and restoring the previous immutable source/image revision; never delete the PostgreSQL volume during a routine rollback.

## Follow-up

Before real-money production, migrate PostgreSQL to Cloud SQL with private IP, enable automated backups/PITR and HA as required, run the backup/restore drill, and produce independent Solana/RPC/security evidence.
