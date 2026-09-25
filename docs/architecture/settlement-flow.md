# Settlement transaction and state machine

## Canonical flow

1. Merchant creates or selects a versioned split plan and requests a quote.
2. API resolves a non-expired policy attestation for merchant, product, jurisdiction, and asset.
3. API validates the asset registry, recipient accounts, amount bounds, client nonce, and policy digest.
4. API compiles a versioned transaction with compute budget, optional Ed25519 verification instruction, and one `settle` instruction.
5. Payer reviews and signs the transaction with a wallet or bounded token delegate.
6. API or client submits the signed transaction to the selected RPC path.
7. Solana executes the entire instruction atomically: policy checks, gross transfer, tax-vault transfer, net split transfers, receipt creation, and events.
8. Indexer observes the signature, verifies the receipt and logs at the configured commitment, and updates projections.
9. Reconciliation confirms chain balances and transitions the external status to `finalized`.

## Hot-path instruction shape

```text
ComputeBudget::set_compute_unit_limit
ComputeBudget::set_compute_unit_price
[Ed25519Program::verify(policy_digest)]  # only for fresh attestation path
settlement_program::settle(
  gross_amount,
  client_nonce,
  policy_digest,
  expected_tax_amount,
  expected_protocol_fee,
  split_plan_version
)
  -> CPI Token/Token-2022 transfer to tax vault
  -> CPI Token/Token-2022 transfers to recipients
  -> write SettlementReceipt PDA
  -> emit SettlementExecuted event
```

The program recomputes all amounts from canonical inputs and rejects caller-supplied values that do not match. Caller-supplied expected values are useful for slippage/quote UX but are not trusted accounting inputs.

## Policy attestation

The canonical payload includes:

```text
schema_version
merchant_id
policy_id
policy_sequence
jurisdiction_code
product_tax_code
asset_mint
tax_rate_bps
tax_cap_atomic
effective_from
effective_until
issuer_key_id
policy_digest
```

The on-chain program validates the digest and bounds. A fresh attestation path includes an Ed25519 verification instruction immediately before settlement; the program validates the instruction data and signer against the registered issuer set through the instructions sysvar. A low-latency cached path references a previously registered PolicyAttestation PDA whose digest, sequence, and window are checked in O(1) account reads.

The protocol never accepts an unsigned “rate” sent by the browser or API.

## State machine

```text
Prepared -> Signed -> Submitted -> Processed -> Confirmed -> Finalized
    |         |          |            |            |
    +---------+----------+------------+------------+--> Rejected
                                             |
                                             +-------> ReconciliationRequired
```

External states are projections. On-chain success is defined by a verified `SettlementReceipt` and transaction meta. A failed transaction may still consume network fees; retry uses the same client nonce only when the prior transaction is not known to have committed and the client can safely detect duplicate receipts.

## Replay and idempotency

- `client_nonce` is unique per merchant and settlement intent.
- `SettlementReceipt` is created before successful instruction completion can return.
- A second settlement with the same merchant and nonce fails with `AlreadySettled`.
- The API stores an idempotency record before returning a transaction and never maps one idempotency key to two different intents.
- Transaction signature, slot, blockhash, policy digest, and receipt address are persisted for audit.

