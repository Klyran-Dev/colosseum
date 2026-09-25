# ADR-0017: Use Docker Compose with pinned stable images for local development

- Status: Accepted
- Date: 2026-09-25

## Context

The system contains a Solana validator, Rust services, a React build, NGINX, databases, cache, observability, and possibly an event stream. Host-installed tools create version drift and make concurrency/integration tests non-reproducible.

## Decision

Provide Docker Compose profiles for localnet development, integration testing, observability, and production-like edge behavior. Use newest compatible stable images at bootstrap, record exact tags and immutable digests, scan them, and update them through reviewed maintenance changes. Never use a floating `latest` tag in CI or production.

## Consequences

- A clean checkout can reproduce the relevant topology.
- Docker resource usage and local image maintenance become explicit costs.
- Production orchestration remains a separate deployment concern; Compose is the local contract, not the production scheduler.

