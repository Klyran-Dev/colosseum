# ADR-0060: Bind settlement compilation to an expiring tenant-scoped quote

- Status: Accepted for implementation
- Date: 2026-09-26
- Decision owners: Principal low-latency architecture, Tax Domain, Settlement Domain

## Context

The wallet checkout must authorize the exact financial result that the server
calculated. A client-supplied `gross_atomic`, `tax_atomic`, or `net_atomic`
can otherwise diverge from the persisted quote between review and signing.
Settlement preparation also must not remain valid indefinitely: published tax
rules, policy attestations, and checkout context are time-bounded evidence.

## Decision

Add a quote-bound settlement compilation endpoint. The authenticated merchant
and `calculation_id` identify the persisted quote; the API loads the quote from
PostgreSQL, rejects an expired or non-settleable quote, and derives all three
settlement amounts from its server-owned `CalculationResult`. The request may
provide account and policy inputs needed by the current bounded compiler, but
it may not provide financial amounts or override the quote identity.

Persist `merchant_id` on every calculation and return an explicit UTC Unix
expiry in the quote contract. The database is authoritative for expiry and the
API performs the check before creating a settlement intent. Existing orphaned
calculation rows must fail the migration rather than receive a guessed tenant
identity.

The first implementation retains the low-latency compiler as a pure function
and adds only one bounded storage lookup. A later account-registry slice will
replace caller-provided merchant vault/destination configuration with an
authenticated merchant Solana account mapping; this ADR does not authorize
free-form merchant authority in production.

## Alternatives considered

1. Keep amounts in the browser request: rejected because the signed transfer
   would not be cryptographically or durably bound to the quoted tax result.
2. Recalculate from the browser request at compile time: rejected because it
   loses the immutable published-rule evidence and can select different facts.
3. Use an unbounded quote: rejected because stale tax and policy evidence can
   be authorized after its operational validity window.
4. Put quote state in an in-memory cache: rejected because failover, replay,
   and audit require PostgreSQL-backed tenant-scoped evidence.

## Consequences

- The checkout can display a deterministic expiry and must ask the user to
  obtain a new quote after expiry.
- The compile endpoint has one bounded database read before intent creation;
  the hot tax calculation path remains pure and cacheable.
- The migration intentionally fails on legacy orphan rows, making remediation
  explicit before production rollout.
- Merchant account registry and policy-registry resolution remain separate
  release gates and must be completed before mainnet settlement enablement.

## Verification

- Storage tests prove quote JSON retains exact atomic values and expiry.
- API tests prove a request cannot substitute amounts for a persisted quote.
- Integration tests prove expired quotes are rejected and valid quotes create a
  durable intent with the server-owned amounts.
