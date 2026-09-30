-- Local-development fixture only. The policy is synthetic and does not
-- establish tax coverage or legal authority.

INSERT INTO jurisdiction_policies
    (merchant_id, rule_set_id, policy_digest, policy_document, provenance,
     status, effective_from, author_subject, reviewer_subject, approver_subject,
     governance_revision)
SELECT
    '01992f1b-4a70-7e30-8ea2-b6e37a9d9d11',
    id,
    decode('77227d8bd8cf5703709f837df7fce025b0fbc05755052f6203fe1e40a58f3411', 'hex'),
    '{"id":"digital-commerce","version":1,"ordered_sources":["ShippingDestination","CustomerResidence","MerchantRegistration"],"conflict_mode":"HighestPrecedence"}'::jsonb,
    '{"source":"synthetic-local-fixture","owner":"development-only"}'::jsonb,
    'published',
    now() - interval '1 minute',
    'local-document-policy-author',
    'local-document-policy-reviewer',
    'local-document-policy-approver',
    1
FROM rule_sets
WHERE merchant_id = '01992f1b-4a70-7e30-8ea2-b6e37a9d9d11'
  AND external_id = 'operator-document-flat-rate'
  AND version = 1
ON CONFLICT (merchant_id, rule_set_id) DO NOTHING;
