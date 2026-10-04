-- Synthetic route for the local integration stack only. These are public
-- system/sysvar addresses and are never valid production settlement config.
INSERT INTO settlement_routes (
    id, merchant_id, route_key, version, status,
    merchant_account, mint, tax_vault, net_destination, token_program,
    mint_decimals,
    verification_digest, verification_slot, verification_commitment,
    verification_schema, verification_evidence, verified_at
)
VALUES (
    '01992f1b-4a70-7e30-8ea2-b6e37a9d9d12',
    '01992f1b-4a70-7e30-8ea2-b6e37a9d9d11',
    'local-synthetic-usd',
    1,
    'active',
    '11111111111111111111111111111114',
    'SysvarC1ock11111111111111111111111111111111',
    'Sysvar1nstructions1111111111111111111111111',
    'SysvarRecentB1ockHashes11111111111111111111',
    'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA',
    2,
    decode(repeat('07', 32), 'hex'),
    0,
    'local_synthetic',
    'settlement-route-verification/v2',
    '{"source":"synthetic-local-fixture","warning":"not chain verified","verification_schema":"settlement-route-verification/v2","asset_config":"AL5PDSRbT5yyhtjQrTmh11rTShZXS9x6QMo9YCMs499d","accounts":[{}, {}, {}, {}, {}],"mint_decimals":2}'::jsonb,
    now()
)
ON CONFLICT (id) DO NOTHING;
