# ADR-0054: Run the NGINX edge as a non-root immutable container

- Status: Accepted
- Date: 2026-09-26
- Decision owners: Principal low-latency architecture and platform security
- Scope: `ops/docker/Dockerfile.web`, `ops/nginx/`, and the Compose `edge` service

## Context

The web edge is a security boundary that serves the static React application
and proxies API requests. Running the upstream image with its default root
master process would violate the repository least-privilege mandate and was
reported as a high-severity Docker misconfiguration by the pinned Trivy gate.
The edge must remain low overhead and must not require a privileged port or a
writable container filesystem.

## Decision

Run NGINX as the image's built-in `nginx` user on port 8080. Keep the external
developer-facing mapping at `${EDGE_PORT:-8088}:8080`. The runtime image:

- uses an explicitly owned temporary directory for NGINX temporary files and
  PID state;
- logs to stdout/stderr;
- runs with a read-only root filesystem, a bounded `/tmp/nginx` tmpfs, all
  Linux capabilities dropped, and `no-new-privileges`;
- retains the existing security headers, API proxy, SPA fallback, and health
  endpoint; and
- is validated in Compose with a UID/GID check, `nginx -t`, and the edge
  healthcheck.

## Alternatives considered

1. **Retain root NGINX on port 80:** rejected because it fails least privilege
   and the container security gate.
2. **Grant `CAP_NET_BIND_SERVICE` to a non-root NGINX on port 80:** rejected
   because it preserves an unnecessary capability and complicates immutable
   runtime hardening.
3. **Replace NGINX with a custom static server:** rejected because it expands
   the security and operational surface without improving this workload's
   latency budget.
4. **Use a separate privileged front proxy:** deferred to the production
   ingress platform; local development and the application edge must remain
   independently testable and least privileged.

## Invariants and release evidence

- The edge process runs as `nginx` and never as UID 0.
- No privileged port or Linux capability is required.
- `nginx -t` passes and `/healthz` returns `204` in the Compose network.
- Trivy critical/high vulnerability, secret, and Docker misconfiguration
  scans pass.
- External TLS termination, WAF policy, certificate rotation, and ingress
  capacity remain deployment-owned release gates.

