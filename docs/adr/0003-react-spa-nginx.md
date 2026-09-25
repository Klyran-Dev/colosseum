# ADR-0003: Use a React SPA served by NGINX; do not use Next.js

- Status: Accepted
- Date: 2026-09-25

## Context

The user explicitly excludes Next.js and requests React with NGINX. The checkout UI is client-side wallet software and does not require a server component framework.

## Decision

Use React 19.3 with TypeScript and Vite to build a static SPA. Use NGINX 1.30.x stable to serve immutable assets, handle SPA fallback, terminate TLS, set security headers, and proxy the Rust API.

## Consequences

- Deployment is simple, cacheable, and horizontally scalable.
- Wallet and browser APIs stay in the client boundary.
- SEO/server-rendered pages are not a primary requirement; any future public content gets a separate static publishing path.

