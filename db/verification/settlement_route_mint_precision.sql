-- Disposable PostgreSQL contract for ADR-0216. The transaction is rolled
-- back so this never becomes production evidence.
BEGIN;

INSERT INTO settlement_routes (
    id, merchant_id, route_key, version, status, merchant_account, mint,
    tax_vault, net_destination, token_program, mint_decimals
)
VALUES (
    '01992f1b-4a70-7e30-8ea2-b6e37a9d9d12',
    '01992f1b-4a70-7e30-8ea2-b6e37a9d9d11',
    'verification-mint-precision', 1, 'pending_verification',
    '11111111111111111111111111111111',
    '22222222222222222222222222222222',
    '33333333333333333333333333333333',
    '44444444444444444444444444444444',
    'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA', 6
);

DO $$
DECLARE
    route_id uuid;
    stored_precision smallint;
    column_type text;
BEGIN
    route_id := '01992f1b-4a70-7e30-8ea2-b6e37a9d9d12';
    SELECT data_type INTO column_type
    FROM information_schema.columns
    WHERE table_schema = 'public'
      AND table_name = 'settlement_routes'
      AND column_name = 'mint_decimals';
    IF column_type <> 'smallint' THEN
        RAISE EXCEPTION 'settlement route mint_decimals must be smallint, got %', column_type;
    END IF;

    SELECT mint_decimals INTO stored_precision
    FROM settlement_routes
    WHERE id = route_id;
    IF stored_precision <> 6 THEN
        RAISE EXCEPTION 'pending route precision did not persist: %', stored_precision;
    END IF;

    UPDATE settlement_routes
    SET status = 'active',
        verification_digest = decode(repeat('11', 32), 'hex'),
        verification_slot = 1,
        verification_commitment = 'finalized',
        verification_evidence = jsonb_build_object('mint_decimals', 6),
        verified_at = now()
    WHERE id = route_id;

    BEGIN
        UPDATE settlement_routes SET mint_decimals = 7 WHERE id = route_id;
        RAISE EXCEPTION 'active route precision mutation unexpectedly succeeded';
    EXCEPTION WHEN OTHERS THEN
        IF POSITION('verified settlement route mint precision is immutable' IN SQLERRM) = 0 THEN
            RAISE;
        END IF;
    END;

    BEGIN
        UPDATE settlement_routes
        SET verification_evidence = jsonb_build_object('mint_decimals', 8)
        WHERE id = route_id;
        RAISE EXCEPTION 'active route evidence substitution unexpectedly succeeded';
    EXCEPTION WHEN OTHERS THEN
        IF POSITION('active or retired route verification evidence is immutable' IN SQLERRM) = 0 THEN
            RAISE;
        END IF;
    END;
END;
$$;

DO $$
BEGIN
    BEGIN
        INSERT INTO settlement_routes (
            id, merchant_id, route_key, version, status, merchant_account, mint,
            tax_vault, net_destination, token_program, mint_decimals
        )
        VALUES (
            '01992f1b-4a70-7e30-8ea2-b6e37a9d9d13',
            '01992f1b-4a70-7e30-8ea2-b6e37a9d9d11',
            'verification-mint-precision-invalid', 1, 'pending_verification',
            '11111111111111111111111111111111',
            '22222222222222222222222222222222',
            '33333333333333333333333333333333',
            '44444444444444444444444444444444',
            'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA', 10
        );
        RAISE EXCEPTION 'out-of-range route precision unexpectedly succeeded';
    EXCEPTION WHEN OTHERS THEN
        IF POSITION('settlement_routes_mint_decimals_check' IN SQLERRM) = 0 THEN
            RAISE;
        END IF;
    END;
END;
$$;

ROLLBACK;
