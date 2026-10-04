# Settlement route verification architecture

The route verifier is a provisioning bounded context. It is intentionally
separate from the checkout compiler and from the finalized receipt indexer.

```text
pending/suspended route
        |
        v
bounded provider pool -- finalized getMultipleAccounts --> account proof
        |                                                     |
        +---------------- fail closed ------------------------+
                                                              v
                                      serializable evidence + activation
```

The verifier requests exactly five accounts in a fixed order:

1. merchant system account;
2. configured mint;
3. configured tax-vault token account;
4. configured net-destination token account;
5. the mint-scoped `AssetConfig` PDA derived from
   `["asset-config", mint]`.

It validates the RPC context, account existence, owner program, executable
flag, data length, token-account mint fields, and the tax-vault authority PDA.
The asset configuration must be owned by the settlement program, use the
canonical Anchor account discriminator and exact layout, bind the mint, token
program, and decimals, identify the governed stablecoin kind, remain enabled,
and carry the derived PDA bump. The account data is bounded before decoding.
The resulting evidence contains only public account metadata, data digests,
mint decimals, commitment, and slot; raw account bytes are not retained in
the route registry. The topology digest is versioned to `v2` so pre-asset-
admission evidence cannot be replayed as current evidence.

Activation is an atomic PostgreSQL operation. The evidence row is append-only,
and the route row stores the selected evidence digest and verification
coordinates. A duplicate digest is idempotent. A different verification result
cannot overwrite an active route; a corrected configuration requires a new
route version. Suspended routes require a fresh explicit verification before
reactivation.

The verifier has no authority to move funds, sign transactions, publish tax
rules, or change program governance. It only establishes that a configured
route's public account topology matches the settlement program's bounded
interface at finalized chain state.

## Revenue payout destinations

Revenue-stream payout destinations use a separate registry and verifier pass;
the merchant settlement route's net destination is never reused. The revenue
verifier requests exactly two accounts in a fixed order: the configured mint
and the configured partner token account. It requires the supported SPL or
extension-free Token-2022 owner, canonical 82-byte mint/165-byte token-account
layouts, matching mint, and the configured recipient owner in the token account
authority field. It records a distinct evidence digest and activates only a
tenant-scoped route version with finalized commitment and provider agreement.
