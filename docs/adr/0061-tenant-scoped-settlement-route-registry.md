# ADR-0061: Tenant-scoped settlement route registry

- Status: Accepted for implementation
- Date: 2026-09-26
- Deciders: Principal Low-Latency Blockchain Architecture
- Scope: Merchant settlement account configuration and quote-bound compilation

## Context

The quote-bound compiler currently accepts merchant, mint, tax-vault,
destination, and token-program public keys from the caller. The compiler
validates their syntax and account aliasing, but syntax validation is not an
authority boundary. A caller must not be able to select another merchant's
vault or destination, and a browser must not be the source of truth for the
merchant's settlement configuration.

The source token account is different: it belongs to the payer and varies by
wallet. Persisting it as merchant configuration would be incorrect. The
settlement program already proves source ownership and mint compatibility
on-chain. The low-latency checkout path must nevertheless choose a canonical
source account without an RPC lookup or an ATA-creation instruction.

## Decision

Create a tenant-scoped Settlement Route Registry bounded context backed by
PostgreSQL. A route is a versioned, public-account configuration containing:

- authenticated `merchant_id` scope;
- an immutable route key and positive configuration version;
- merchant system account;
- asset mint;
- tax-vault token account;
- net-destination token account;
- the selected SPL Token or compatible Token-2022 program;
- lifecycle status (`pending_verification`, `active`, `suspended`, or
  `retired`) and an effective window.

The quote-bound compiler accepts only `calculation_id`, an authenticated
tenant-scoped `route_id`, payer wallet public key, nonce, policy sequence, and
policy digest. It ignores and rejects caller-supplied merchant, mint, vault,
destination, token-program, or program identifiers. The API loads an active
route for the authenticated merchant, validates canonical public keys and
distinct token accounts, fixes the settlement program ID from the compiled
protocol configuration, and passes the resolved values to the deterministic
compiler.

The source account is derived locally as the canonical associated token
account PDA from payer, selected token program, and mint. The compiler does
not add an ATA-creation instruction and does not perform an RPC read in the
quote/compile critical path. A missing or non-standard payer account is a
recoverable wallet/setup failure; it is never silently substituted.

Published/active route account fields are immutable. Corrections create a new
route version and a new route ID. Suspension and retirement are explicit
state changes with audit evidence. Only a separately authorized chain
verification workflow may promote a route to `active`; this slice provides
the read/compile boundary and synthetic local fixtures, not production route
governance or chain verification.

The database is authoritative for tenant configuration and lifecycle state;
Solana remains authoritative for account ownership, token-account layout,
mint decimals, policy attestation, and funds. A route row is not proof that
the corresponding accounts currently exist on-chain.

## Alternatives considered

### Accept caller-supplied account tuples

Rejected. It permits account substitution and makes the browser/API request a
second authority for merchant funds.

### Discover accounts through RPC during compilation

Rejected for the hot path. It adds provider latency and ambiguity, couples
compilation to an external dependency, and creates a retry/finality problem.
Chain verification belongs in a bounded provisioning/activation workflow.

### Store source token accounts in the merchant registry

Rejected. Source ownership is payer-specific and a merchant route cannot
represent every payer wallet. The canonical ATA derivation provides a bounded
deterministic default; future explicit payer-account selection requires a
separate wallet-proof and account-discovery design.

### Put the registry on Solana and read it for every quote

Rejected as the primary configuration read path. It adds account locks,
transaction/RPC latency, migration complexity, and does not remove the need
for off-chain tenant administration. Solana account constraints remain the
final settlement authority.

### Environment variables or one global route

Rejected. It cannot provide tenant isolation, versioned lifecycle evidence,
or safe multi-merchant operation.

## Invariants

1. A route lookup is always scoped by authenticated `merchant_id` and route
   ID; a caller-supplied merchant ID is never used as an authority boundary.
2. Only an active, effective route can compile a production-bound quote.
3. Active route account fields cannot be mutated in place.
4. Merchant, tax vault, and destination accounts are distinct; the mint and
   token program are valid supported public keys.
5. The source account is deterministically derived from payer, mint, and
   token program; it is not accepted from the browser.
6. Route resolution never changes quote gross, tax, net, policy digest, or
   calculation evidence.
7. A route configuration failure fails closed before durable settlement intent
   creation or wallet signing.

## Consequences

Positive:

- Tenant account substitution is removed from the checkout request boundary.
- Route resolution is one bounded indexed PostgreSQL lookup and no RPC call.
- Route changes are versioned and auditable rather than destructive edits.
- The UI can present a route selector with public account evidence instead of
  unsafe free-form account fields.

Trade-offs and release gates:

- A route-management and chain-verification workflow is still required before
  production activation.
- Payers must have the selected associated token account already created;
  automatic ATA creation is deliberately outside this settlement transaction.
- The database configuration must be reconciled against finalized Solana
  state, and localnet tests must prove the route's accounts and extensions.

