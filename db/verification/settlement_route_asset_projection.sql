-- Disposable PostgreSQL contract for ADR-0325. Every mutation is rolled back
-- so this script never becomes production evidence.
BEGIN;

INSERT INTO settlement_routes (
    id, merchant_id, route_key, version, status,
    merchant_account, mint, tax_vault, net_destination, token_program
)
VALUES (
    '01992f1b-4a70-7e30-8ea2-b6e37a9d9d14',
    '01992f1b-4a70-7e30-8ea2-b6e37a9d9d11',
    'verification-asset-projection', 1, 'pending_verification',
    '11111111111111111111111111111111',
    '22222222222222222222222222222222',
    '33333333333333333333333333333333',
    '44444444444444444444444444444444',
    'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA'
);

DO $$
BEGIN
    BEGIN
        UPDATE settlement_routes
        SET status = 'active',
            mint_decimals = 6,
            verification_digest = decode(repeat('21', 32), 'hex'),
            verification_slot = 1,
            verification_commitment = 'finalized',
            verification_schema = 'settlement-route-verification/v2',
            verification_evidence = jsonb_build_object('mint_decimals', 6),
            verified_at = now()
        WHERE id = '01992f1b-4a70-7e30-8ea2-b6e37a9d9d14';
        RAISE EXCEPTION 'active route without typed asset proof unexpectedly succeeded';
    EXCEPTION WHEN OTHERS THEN
        IF POSITION('settlement_routes_active_asset_projection_check' IN SQLERRM) = 0 THEN
            RAISE;
        END IF;
    END;
END;
$$;

UPDATE settlement_routes
SET status = 'active',
    mint_decimals = 6,
    verification_digest = decode(repeat('22', 32), 'hex'),
    verification_slot = 1,
    verification_commitment = 'finalized',
    verification_schema = 'settlement-route-verification/v2',
    asset_config = '55555555555555555555555555555555',
    asset_kind = 1,
    asset_enabled = true,
    asset_config_bump = 1,
    verification_evidence = jsonb_build_object(
        'asset_config', '55555555555555555555555555555555',
        'asset_kind', 1,
        'asset_enabled', true,
        'asset_config_bump', 1,
        'mint_decimals', 6
    ),
    verified_at = now()
WHERE id = '01992f1b-4a70-7e30-8ea2-b6e37a9d9d14';

DO $$
BEGIN
    BEGIN
        UPDATE settlement_routes SET asset_kind = 2
        WHERE id = '01992f1b-4a70-7e30-8ea2-b6e37a9d9d14';
        RAISE EXCEPTION 'active asset projection mutation unexpectedly succeeded';
    EXCEPTION WHEN OTHERS THEN
        IF POSITION('active or retired route verification evidence is immutable' IN SQLERRM) = 0 THEN
            RAISE;
        END IF;
    END;
END;
$$;

ROLLBACK;
