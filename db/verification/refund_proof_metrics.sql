-- Disposable PostgreSQL 18 contract for ADR-0159. The transaction is rolled
-- back so this never becomes production evidence.
BEGIN;

INSERT INTO chain_checkpoints (stream_key, next_slot, finalized_slot, generation)
VALUES ('verification/refund-proof-metrics', 1, 1, 0)
ON CONFLICT (stream_key) DO NOTHING;

INSERT INTO indexed_adjustment_refund_executions (
    stream_key, signature, slot, event_index, claim, adjustment,
    original_receipt, merchant, mint, nonce, gross_magnitude, tax_magnitude,
    net_magnitude, refund_funder, refund_destination, approver,
    reference_digest, evidence_digest
)
VALUES (
    'verification/refund-proof-metrics',
    'refund-proof-metrics-synthetic-signature',
    777, 0, decode(repeat('01', 32), 'hex'), decode(repeat('02', 32), 'hex'),
    decode(repeat('03', 32), 'hex'), decode(repeat('04', 32), 'hex'),
    decode(repeat('05', 32), 'hex'), 9, 75, 25, 50,
    decode(repeat('06', 32), 'hex'), decode(repeat('07', 32), 'hex'),
    decode(repeat('08', 32), 'hex'), decode(repeat('09', 32), 'hex'),
    decode(repeat('0a', 32), 'hex')
)
ON CONFLICT (stream_key, signature, event_index) DO NOTHING;

DO $$
DECLARE
    pending bigint;
BEGIN
    SELECT COUNT(*)
      INTO pending
      FROM indexed_adjustment_refund_executions execution
      LEFT JOIN refund_balance_proofs proof
        ON proof.stream_key = execution.stream_key
       AND proof.signature = execution.signature
       AND proof.event_index = execution.event_index
     WHERE execution.stream_key = 'verification/refund-proof-metrics'
       AND proof.id IS NULL;
    IF pending <> 1 THEN
        RAISE EXCEPTION 'expected one pending refund proof, found %', pending;
    END IF;
END;
$$;

INSERT INTO refund_balance_proofs (
    stream_key, signature, event_index, slot, claim, adjustment,
    original_receipt, merchant, mint, token_program, tax_vault, refund_source,
    payer_destination, vault_authority, refund_funder, payer, nonce,
    gross_atomic, tax_atomic, net_atomic, tax_vault_before_atomic,
    tax_vault_after_atomic, refund_source_before_atomic,
    refund_source_after_atomic, payer_destination_before_atomic,
    payer_destination_after_atomic, proof_digest
)
VALUES (
    'verification/refund-proof-metrics',
    'refund-proof-metrics-synthetic-signature',
    0, 777, decode(repeat('01', 32), 'hex'), decode(repeat('02', 32), 'hex'),
    decode(repeat('03', 32), 'hex'), decode(repeat('04', 32), 'hex'),
    decode(repeat('05', 32), 'hex'), decode(repeat('0b', 32), 'hex'),
    decode(repeat('0c', 32), 'hex'), decode(repeat('0d', 32), 'hex'),
    decode(repeat('0e', 32), 'hex'), decode(repeat('0f', 32), 'hex'),
    decode(repeat('06', 32), 'hex'), decode(repeat('10', 32), 'hex'), 9,
    75, 25, 50, 100, 75, 200, 150, 0, 75,
    decode(repeat('11', 32), 'hex')
);

DO $$
DECLARE
    pending bigint;
BEGIN
    SELECT COUNT(*)
      INTO pending
      FROM indexed_adjustment_refund_executions execution
      LEFT JOIN refund_balance_proofs proof
        ON proof.stream_key = execution.stream_key
       AND proof.signature = execution.signature
       AND proof.event_index = execution.event_index
     WHERE execution.stream_key = 'verification/refund-proof-metrics'
       AND proof.id IS NULL;
    IF pending <> 0 THEN
        RAISE EXCEPTION 'expected no pending refund proof after persistence, found %', pending;
    END IF;
END;
$$;

ROLLBACK;
