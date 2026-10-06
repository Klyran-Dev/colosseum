# ADR-0359: Explicit fee-payer binding at the signed-message boundary

- Status: Accepted
- Date: 2026-10-05
- Deciders: Principal Low-Latency Blockchain Architecture
- Scope: settlement coordinator wallet-submission verification

## Context

The signed-message verifier already rejects an unexpected sponsored fee payer
through account-shape and privilege checks. That indirect rejection is fragile
as a security contract: a future transaction profile could accidentally admit a
known account as the fee payer while preserving the compiled instruction
domain. The compiled wallet handoff explicitly selects its first account as
the fee payer, so that relationship must be checked directly and before the
remaining account comparison.

## Decision

Every wallet-submission profile—settlement, adjustment, refund movement,
refund cancellation, and revenue-stream movement—must verify that:

1. the first static account in the signed Solana message is the first account
   in the server-compiled artifact;
2. that account is a required signer; and
3. that account is writable as required by Solana's fee-payer message rules.

The coordinator rejects an absent, substituted, or sponsored fee payer with a
stable `InvalidSignedTransaction` failure before profile-specific account-shape
comparison or any RPC call. This is an explicit defense-in-depth invariant; it
does not change the non-custodial wallet model or permit API-side signing.

## Alternatives considered

### Rely on indirect account-count and privilege rejection

Rejected as the sole contract. It currently fails closed, but the invariant is
not visible at the authority boundary and could regress when a new profile or
account topology is added.

### Permit a sponsored fee payer

Rejected. Sponsorship is a separate product and authorization contract. The
current compiled artifact, wallet review, replay evidence, and fee policy bind
the intended wallet as the fee payer.

### Reconstruct or re-sign with the intended payer

Rejected. The API must remain non-custodial and must verify immutable bytes
returned by the user's wallet.

## Consequences

- Fee-payer binding is explicit, testable, and auditable across all submission
  profiles.
- The hot path adds only bounded in-memory checks and no RPC or database work.
- A future sponsored-fee feature must define a new artifact, authorization,
  UI disclosure, and ADR rather than weakening this boundary.
- Local tests prove the invariant; Linux validator, staging wallet, and
  independent security-review evidence remain release gates.

## Verification

- A real two-key Solana transaction where the compiled payer signs but a
  different wallet pays fees is rejected with the fee-payer binding error.
- Existing settlement, adjustment, refund, cancellation, revenue-stream,
  lookup-table, signature, and account-policy tests remain green.
