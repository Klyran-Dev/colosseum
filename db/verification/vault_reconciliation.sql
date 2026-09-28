-- Disposable PostgreSQL 18 contract for ADR-0127. The transaction is rolled
-- back so this never becomes production evidence.
BEGIN;

INSERT INTO chain_checkpoints (stream_key, next_slot, finalized_slot, generation)
VALUES ('verification/vault-reconciliation', 100, 100, 0)
ON CONFLICT (stream_key) DO NOTHING;

INSERT INTO indexed_receipts (
    stream_key, signature, slot, gross_atomic, tax_atomic, net_atomic,
    evidence_digest, receipt, payer, merchant, mint, nonce, policy_digest
)
VALUES (
    'verification/vault-reconciliation',
    'vault-reconciliation-synthetic-receipt',
    10, 250, 100, 150, decode(repeat('11', 32), 'hex'),
    decode(repeat('21', 32), 'hex'), decode(repeat('22', 32), 'hex'),
    decode(repeat('23', 32), 'hex'), decode(repeat('24', 32), 'hex'),
    10, decode(repeat('25', 32), 'hex')
)
ON CONFLICT (stream_key, signature) DO NOTHING;

INSERT INTO vault_release_claims (
    id, merchant_id, idempotency_key, request_digest, mint, token_program,
    tax_vault, destination, claim_pda, nonce, amount_atomic, reference_digest, status,
    proposer_subject, execute_after, expires_at, transaction_signature, slot
)
VALUES (
    uuidv7(),
    '01992f1b-4a70-7e30-8ea2-b6e37a9d9d11',
    'vault-reconciliation-synthetic-release',
    decode(repeat('12', 32), 'hex'),
    'SysvarC1ock11111111111111111111111111111111',
    'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA',
    'Sysvar1nstructions1111111111111111111111111',
    'SysvarRecentB1ockHashes11111111111111111111',
    'SysvarC1ock11111111111111111111111111111112',
    7, 100, decode(repeat('13', 32), 'hex'), 'executed',
    'synthetic-reconciliation-test', now(), now() + interval '1 day',
    'synthetic-vault-release-signature', 20
)
ON CONFLICT (merchant_id, idempotency_key) DO NOTHING;

CREATE TEMP TABLE verification_vault_target (
    id uuid NOT NULL
) ON COMMIT DROP;

INSERT INTO vault_reconciliation_targets (
    merchant_id, route_id, stream_key, mint, token_program, tax_vault,
    opening_balance_atomic, opening_balance_slot, opening_reference
)
VALUES (
    '01992f1b-4a70-7e30-8ea2-b6e37a9d9d11',
    '01992f1b-4a70-7e30-8ea2-b6e37a9d9d12',
    'verification/vault-reconciliation',
    'SysvarC1ock11111111111111111111111111111111',
    'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA',
    'Sysvar1nstructions1111111111111111111111111',
    1000, 1, 'synthetic-finalized-opening-balance'
);

INSERT INTO verification_vault_target (id)
SELECT id
FROM vault_reconciliation_targets
WHERE opening_reference = 'synthetic-finalized-opening-balance';

DO $$
DECLARE
    target_id uuid;
    expected numeric;
    mismatches bigint;
BEGIN
    SELECT id INTO target_id FROM verification_vault_target;
    SELECT opening_balance_atomic + receipt_tax_atomic - release_atomic
      INTO expected
      FROM (
          SELECT 1000::numeric AS opening_balance_atomic,
                 (SELECT SUM(tax_atomic) FROM indexed_receipts
                  WHERE stream_key = 'verification/vault-reconciliation') AS receipt_tax_atomic,
                 (SELECT SUM(amount_atomic) FROM vault_release_claims
                  WHERE status = 'executed'
                    AND tax_vault = 'Sysvar1nstructions1111111111111111111111111') AS release_atomic
      ) values;
    IF expected <> 1000 THEN
        RAISE EXCEPTION 'vault expected balance contract failed: %', expected;
    END IF;

    INSERT INTO vault_reconciliation_runs (
        target_id, route_id, merchant_id, stream_key, mint, token_program,
        tax_vault, opening_balance_atomic, receipt_tax_atomic, release_atomic,
        expected_balance_atomic, observed_balance_atomic, observed_slot, status
    )
    VALUES (
        target_id,
        '01992f1b-4a70-7e30-8ea2-b6e37a9d9d12',
        '01992f1b-4a70-7e30-8ea2-b6e37a9d9d11',
        'verification/vault-reconciliation',
        'SysvarC1ock11111111111111111111111111111111',
        'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA',
        'Sysvar1nstructions1111111111111111111111111',
        1000, 100, 100, 1000, 1000, 100, 'matched'
    );

    INSERT INTO vault_reconciliation_runs (
        target_id, route_id, merchant_id, stream_key, mint, token_program,
        tax_vault, opening_balance_atomic, receipt_tax_atomic, release_atomic,
        expected_balance_atomic, observed_balance_atomic, observed_slot, status
    )
    SELECT target_row.id,
        target_row.route_id, target_row.merchant_id, target_row.stream_key,
        target_row.mint, target_row.token_program, target_row.tax_vault,
        1000, 100, 100, 1000, 999, 101, 'mismatch'
    FROM vault_reconciliation_targets target_row
    WHERE target_row.id = target_id;

    SELECT COUNT(*) INTO mismatches
    FROM (
        SELECT DISTINCT ON (runs.target_id) runs.target_id, runs.status
        FROM vault_reconciliation_runs runs
        ORDER BY runs.target_id, runs.created_at DESC, runs.id DESC
    ) latest
    WHERE latest.status = 'mismatch';
    IF mismatches <> 1 THEN
        RAISE EXCEPTION 'vault mismatch latest-run contract failed: %', mismatches;
    END IF;
END
$$;

DO $$
BEGIN
    BEGIN
        UPDATE vault_reconciliation_runs SET observed_balance_atomic = 1001;
        RAISE EXCEPTION 'vault reconciliation run UPDATE unexpectedly succeeded';
    EXCEPTION WHEN OTHERS THEN
        IF POSITION('append-only' IN SQLERRM) = 0 THEN
            RAISE;
        END IF;
    END;
    BEGIN
        DELETE FROM vault_reconciliation_runs;
        RAISE EXCEPTION 'vault reconciliation run DELETE unexpectedly succeeded';
    EXCEPTION WHEN OTHERS THEN
        IF POSITION('append-only' IN SQLERRM) = 0 THEN
            RAISE;
        END IF;
    END;
END
$$;

ROLLBACK;
