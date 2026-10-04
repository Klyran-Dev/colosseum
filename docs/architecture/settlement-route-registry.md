# Settlement Route Registry bounded context

The Settlement Route Registry owns tenant-scoped, versioned configuration for
the public Solana accounts used by a settlement route. It is a configuration
authority, not a funds ledger and not a substitute for Solana account
constraints.

## Aggregate and lifecycle

```text
pending_verification -> active -> suspended -> retired
                                  \-> active (only through an explicit,
                                      authorized re-verification)
```

Account fields are immutable after activation. A correction creates a new
route version and ID; suspension/retirement preserves the old row for audit.
The active state is only valid inside the effective time window and after a
chain verification workflow has recorded v2 evidence for the five-account
topology, including the enabled mint-scoped `AssetConfig` PDA. Legacy
four-account evidence remains historical and queryable but cannot authorize an
active route. Existing rows without v2 evidence are suspended by the
expand/contract migration and must complete explicit finalized re-verification
before returning to active.

## Hot-path read contract

`resolve(merchant_id, route_id, now)` performs one indexed, tenant-scoped
read. It returns the merchant account, mint, verified `mint_decimals`, tax
vault, net destination, token program, and verified asset-configuration PDA.
The route must carry `settlement-route-verification/v2`; legacy or missing
evidence returns a re-verification-required state rather than a fallback.
`mint_decimals` is bounded to `0..=9`, derived from finalized
route-verification evidence, and immutable for the route version. The source
token account is derived from the payer wallet, token program, and mint using
the associated-token-account PDA formula. No RPC, database write, ATA
creation, or dynamic rule evaluation is allowed in this read.

The quote remains the authority for gross, tax, net, rule digest, engine
version, and expiry. The route remains the authority for account selection.
The on-chain program remains the authority for account ownership, token
compatibility, policy attestation, conservation, replay, and movement of
funds.

## Failure behavior

Missing, suspended, retired, expired, malformed, aliased, or unverified route
configuration fails closed with a supportable error. The API does not fall
back to caller-supplied addresses or a global default route. A payer without
the derived source account must receive a wallet/setup remediation state; the
API must not create that account in the settlement hot path.
