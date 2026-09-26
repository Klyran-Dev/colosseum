# ADR-0038: Make the authenticated quote workspace the primary UI boundary

- Status: Accepted for the first production UI slice
- Date: 2026-09-25
- Owners: Product / API / Security
- Scope: React/Vite merchant and operator workspace

## Context

The initial SPA only called the explicitly simulated `/v1/tax/calculate` route.
That is useful for rule exploration but cannot be the primary product workflow:
it accepts caller-supplied rules and does not prove authentication, published
rule selection, idempotency, or persisted evidence.

The product must not fabricate wallet, attestation, settlement, or finalized
states. The browser also must not persist bearer tokens or custody wallet keys.

## Decision

Make an authenticated `/v1/tax/quote` workspace the default experience. It:

- requires an operator-supplied OIDC access token with `tax:quote` scope;
- sends only the versioned quote contract to the API;
- displays the server's `calculated_unattested` status, calculation ID, rule
  selector, tax trace, and evidence digests;
- keeps the access token in React memory only and never writes it to storage;
- preserves a separate, visibly labeled preview mode for design/test use;
- disables actions when the service is unavailable or required authorization is
  absent and reports 401/403/409/5xx states without pretending success.

Wallet signing, attestation issuance, transaction compilation, and finalized
settlement remain separate product states. The UI presents those states only
when a real backend contract supplies them.

## Alternatives considered

See `docs/architecture/alternative-analysis.md` under “Product UI data
boundary”. Client-side calculation and fabricated dashboards were rejected;
full-stack/server-rendered delivery violates the selected SPA architecture.

## Consequences

Positive:

- The primary UI exercises the production authentication and evidence path.
- Users can distinguish preview, calculated, attested, submitted, and finalized
  states.
- No bearer token is persisted and no client-side result is treated as funds
  authority.

Costs:

- The local operator must provide a valid test access token for persisted
  quotes.
- Wallet and settlement screens require subsequent real API/wallet adapters.
- UI contract tests must cover authorization, idempotency, stale evidence, and
  degraded service states.
