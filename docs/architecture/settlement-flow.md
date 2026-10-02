# Settlement transaction and state machine

## Canonical flow

1. Merchant creates or selects a versioned split plan and requests a quote.
2. API resolves a non-expired policy attestation for merchant, product, jurisdiction, and asset.
3. API validates the asset registry, recipient accounts, amount bounds, client nonce, and policy digest.
4. The authenticated API compiles a deterministic, wallet-reviewable
   `prepared_unsubmitted` `settle` instruction with fixed account metas,
   derived PDAs, and the server-owned versioned compute-budget policy. A later
   coordinator adds the blockhash and legacy/v0 transaction wrapping.
5. Payer reviews and signs the transaction with a wallet or bounded token delegate. A wallet may wrap the exact instruction in a legacy or governed v0 message; v0 lookup tables are resolved by the server at submission and never supplied as trusted client metadata.
6. API accepts only the wallet-signed bytes, binds their digest and native
   transaction signature to the durable UUIDv7 intent, and wins a `signed ->
   submitting` compare-and-set before submitting immutable bytes to a bounded
   RPC provider pool. A provider timeout is `unknown`, never an implicit
   retry/new intent; a process crash leaves a visible `submitting` intent for
   signature reconciliation.
7. Solana executes the entire instruction atomically: policy checks, gross transfer, tax-vault transfer, net split transfers, receipt creation, and events.
8. Indexer observes the signature, verifies the receipt and logs at the configured commitment, and updates projections.
9. Reconciliation confirms chain balances and transitions the external status to `finalized`.

The singleton `protocol-config` PDA is a mandatory read-only account on every
token-moving instruction, including governed `execute_vault_release`.
`ProtocolConfig.paused` is checked before the first token CPI; claim creation
and governance cancellation remain available for incident recovery. ADR-0242
defines the vault-release account-contract extension and its conformance
evidence.

## Target hot-path instruction shape

```text
ComputeBudget::set_compute_unit_limit
ComputeBudget::set_compute_unit_price
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

In the target policy-attested production path, the program recomputes all
amounts from canonical inputs and rejects caller-supplied values that do not
match. Caller-supplied expected values are useful for slippage/quote UX but
are not trusted accounting inputs. The current first vertical slice accepts
bounded tax/net values only to prove token movement and conservation; it is
explicitly not the policy-attested production path.

An eventual inline Ed25519 attestation-verification profile would require its
own instruction-shape ADR and wire/compute evidence; it is not silently added
to the three-instruction settlement profile governed by ADR-0162.

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

The first executable vertical slice uses Anchor's token interface and requires
a cached `PolicyAttestation` PDA. It accepts legacy SPL Token and extension-free
Token-2022 accounts, and validates policy
merchant/mint/sequence/digest/amount binding, expiry and revocation, amount
conservation, account ownership, mint identity, token-program consistency,
extension-free account layouts, vault PDA authority, distinct token accounts,
and deterministic receipt creation. ADR-0034 adds the merchant/mint-scoped
governed issuer registry used to authorize registration and revocation. Fresh
Ed25519 verification, extension-by-extension Token-2022 support, and
production multisig/timelock integration are still required before a deployed
settlement path can claim production readiness.

The deterministic compiler is intentionally non-custodial: it has no RPC,
clock, database, wallet, or private-key dependency. It validates the same
conservation and account-domain constraints before returning instruction bytes.
The returned artifact is never described as signed, submitted, simulated,
processed, confirmed, or finalized.

## State machine

```text
Prepared -> Signed -> Submitted -> Processed -> Confirmed -> Finalized
    |         |          |            |            |
    +---------+----------+------------+------------+--> Rejected
              |          |            |            |
              +----------+------------+------------+--> Unknown -> reconciled evidence
```

External states are projections. On-chain success is defined by a verified `SettlementReceipt` and transaction meta. A failed transaction may still consume network fees; retry uses the same client nonce only when the prior transaction is not known to have committed and the client can safely detect duplicate receipts.

## Replay and idempotency

- `client_nonce` is unique per merchant and settlement intent.
- `SettlementReceipt` is created before successful instruction completion can return.
- A second settlement with the same merchant and nonce fails with `AlreadySettled`.
- The API stores an idempotency record before returning a transaction and never maps one idempotency key to two different intents.
- Transaction signature, slot, blockhash, policy digest, and receipt address are persisted for audit.
- The API lifecycle endpoints never accept private keys. `tax:settlement:submit`
  and `tax:settlement:read` are separate scopes; durable intent creation and
  expected-status event fencing are required before submission is enabled.
