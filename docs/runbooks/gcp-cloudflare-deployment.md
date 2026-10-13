# GCP + Cloudflare deployment runbook

This runbook describes the controlled hackathon deployment topology documented in
[ADR-0520](../adr/0520-gce-single-vm-cloudflare-tunnel-deployment.md).

## Topology

- GCE VM: `colosseum-app`, zone `asia-south1-a`
- VM shape: `e2-standard-4`, 100 GiB balanced persistent disk
- Runtime: pinned Docker Compose services with PostgreSQL on the same VM
- Public web hostname: `https://colosseum.klyran.com`
- Public auth hostname: `https://auth.klyran.com`
- Cloudflare Tunnel origin services: `127.0.0.1:8088` and `127.0.0.1:18085`
- Solana environment: devnet for the hackathon deployment
- Solana program: `4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi`

PostgreSQL, NATS, API, auth, and NGINX ports are loopback-bound on the VM. The
Cloudflare Tunnel is outbound-only; no application port is exposed directly to
the Internet.

## Deploy the application

The hackathon deployment is a staging release, even if the operator names its
untracked runtime file `.env.production`. Before starting Compose, it must
contain the staging/Devnet contract below and the complete non-local service
configuration required by `scripts/verify_deployment_config.sh`:

```text
DEPLOYMENT_ENV=staging
AUTH_ENV=staging
API_ENV=staging
INDEXER_ENV=staging
ROUTE_VERIFIER_ENV=staging
TAX_ENGINE_ENV=staging
VITE_DEPLOYMENT_ENVIRONMENT=staging
VITE_SOLANA_NETWORK=devnet
VITE_SOLANA_RPC_URL=https://<approved-devnet-rpc>
API_SETTLEMENT_RPC_ENDPOINTS=https://<approved-devnet-rpc-1>,https://<approved-devnet-rpc-2>
API_INDEXER_RPC_ENDPOINTS=https://<approved-devnet-rpc-1>,https://<approved-devnet-rpc-2>
INDEXER_RPC_ENDPOINTS=https://<approved-devnet-rpc-1>,https://<approved-devnet-rpc-2>
ROUTE_VERIFIER_RPC_ENDPOINTS=https://<approved-devnet-rpc-1>,https://<approved-devnet-rpc-2>
```

`VITE_SOLANA_RPC_URL` is public browser configuration, not a place for an RPC
provider secret. Provider credentials remain server-side. Run the preflight on
the exact source bundle before starting services; it must pass without
printing secret values:

```bash
docker compose --env-file .env.staging run --rm --no-deps api \
  /usr/local/bin/verify_deployment_config
```

Verify the canonical program separately at finalized commitment with the
release RPC evidence procedure. A Devnet label alone is not deployment proof;
the program account must be executable, owned by the upgradeable BPF loader,
and the retained SBF artifact digest must be recorded for release review:

```bash
SOLANA_NETWORK=devnet \
SOLANA_RPC_URL=https://<approved-devnet-rpc> \
SOLANA_ARTIFACT_PATH=target/deploy/settlement.so \
make solana-devnet-program-check
```

From a workstation with access to the project:

```bash
gcloud compute ssh colosseum-app \
  --project=project-7dfff051-32d5-4721-88d \
  --zone=asia-south1-a \
  --command='cd /opt/colosseum && docker compose --env-file .env.staging --profile indexer --profile route-verifier up --build -d postgres migrator nats tax-engine api worker auth-inbox-init auth-service indexer route-verifier edge'
```

Check service state and health:

```bash
gcloud compute ssh colosseum-app \
  --project=project-7dfff051-32d5-4721-88d \
  --zone=asia-south1-a \
  --command='cd /opt/colosseum && docker compose --env-file .env.staging --profile indexer --profile route-verifier ps && curl -fsS http://127.0.0.1:8088/healthz'
```

## Tenant administrator lifecycle

`AUTH_INITIAL_ADMIN_EMAIL` is a one-time bootstrap mailbox for the tenant
created by that enrollment. It is not a platform administrator. With
`AUTH_SELF_SIGNUP=true`, each new self-signup creates its own tenant and makes
the first verified user that tenant's administrator. Invited users join the
inviter's tenant with only the selected merchant responsibility; a tenant
administrator promotes another active member from **Access & approvals**.

The promotion and workspace-name mutations are tenant-derived, UUIDv7
idempotent, and audited. Never edit role or tenant rows manually on the VM.
The current release intentionally has no platform-global administrator; adding
one requires a separate global authority model and security review.

## Cloudflare Tunnel

The tunnel is configured with these ingress routes:

```text
colosseum.klyran.com      -> http://127.0.0.1:8088
auth.klyran.com -> http://127.0.0.1:18085
```

The tunnel credentials are stored only on the VM under `/etc/cloudflared` with
root-only permissions. Never commit them, print them, or paste them into an
issue, chat, or CI log.

## Email prerequisite

For Google Workspace SMTP relay mode, allowlist the VM's reserved egress IP in
Google Admin and ensure `noreply@klyran.com` is an approved sender. The runtime
configuration uses relay/IP-allowlist mode and does not store the previously
exposed app password. If an app-password fallback is ever required, revoke the
old credential first and provision a newly rotated secret through a secure
secret-management path.

## Rollback

Re-deploy the previously verified source bundle and restart only the affected
Compose services. Do not remove the PostgreSQL volume during an application
rollback. Use `make reset-local` only for intentionally discarding synthetic
local state, never for the shared VM.

## Scope

This is a staging/hackathon deployment, not a high-availability or mainnet
production deployment. Before handling real funds, move PostgreSQL to a managed
HA service, use approved KMS/HSM and multisig controls, add backup/restore
verification, remove temporary operator SSH access, and complete the release
security and validator-backed conformance gates.
