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

The verifier requests exactly four accounts in a fixed order:

1. merchant system account;
2. configured mint;
3. configured tax-vault token account;
4. configured net-destination token account.

It validates the RPC context, account existence, owner program, executable
flag, data length, token-account mint fields, and the tax-vault authority PDA.
The account data is bounded before decoding. The resulting evidence contains
only public account metadata, data digests, mint decimals, commitment, and
slot; raw account bytes are not retained in the route registry.

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

