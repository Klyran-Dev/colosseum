-- Synthetic local-development Split Plan only. It is not chain-activated
-- production configuration and must never be copied into a release migration.

INSERT INTO settlement_split_plans (
    id, merchant_id, plan_key, version, status,
    merchant_account, mint, plan_digest, recipients,
    activation_digest, activation_slot, activation_commitment,
    activation_evidence, activated_at, effective_from
)
VALUES (
    '01992f1b-4a70-7e30-8ea2-b6e37a9d9d13',
    '01992f1b-4a70-7e30-8ea2-b6e37a9d9d11',
    'local-synthetic-split',
    1,
    'active',
    '11111111111111111111111111111114',
    'SysvarC1ock11111111111111111111111111111111',
    decode('f838026ea1a77c87110fb6bf098a15f79daaea653aa0c5bc94c221ed36de993b', 'hex'),
    '[
      {"token_account":"SysvarRent111111111111111111111111111111111","basis_points":7000},
      {"token_account":"ComputeBudget111111111111111111111111111111","basis_points":3000}
    ]'::jsonb,
    decode(repeat('09', 32), 'hex'),
    0,
    'local_synthetic',
    '{"source":"synthetic-local-fixture","warning":"not chain verified"}'::jsonb,
    now(),
    now() - interval '1 minute'
)
-- Active split plans are immutable after activation. Re-running local
-- bootstrap must not mutate chain activation evidence or timestamps.
ON CONFLICT (id) DO NOTHING;
