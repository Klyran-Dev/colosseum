-- Local-development fixture only. Never promote this file to a production
-- migration or treat it as legal/tax coverage.

INSERT INTO rule_sets
    (id, merchant_id, external_id, version, jurisdiction, regime, status, rule_digest, rule_document, provenance, effective_from,
     author_subject, reviewer_subject, approver_subject, governance_revision)
VALUES
    (
      '01992f1b-4a70-7e30-8ea2-b6e37a9d9d10',
      '01992f1b-4a70-7e30-8ea2-b6e37a9d9d11',
      'operator-simulator-flat-rate',
      1,
      'US-CA',
      'sales-tax',
      'published',
      decode('26aae04b4685bf7c07733bfc949ca1eede22a0f8199bc70944fcae0b5e750732', 'hex'),
      '{"schema":"solana-tax-protocol/tax-rule/v1","rule_set":{"id":"operator-simulator-flat-rate","version":1,"components":[{"id":"sales-tax","label":"Sales tax","base":{"sources":["Net"]},"rate":{"Flat":{"numerator":"10","denominator":"100"}},"price_mode":"Exclusive","credit_atomic":"0","floor_atomic":null,"cap_atomic":null,"rounding":{"mode":"HalfUp"},"threshold":null,"exemption_codes":[]}]}}'::jsonb,
      '{"schema":"tax-rule-provenance/v1","source_reference":"controlled-record://local/operator-simulator-flat-rate","source_digest_hex":"26aae04b4685bf7c07733bfc949ca1eede22a0f8199bc70944fcae0b5e750732","legal_tax_owner":"tax-owner:synthetic-local","jurisdiction_scope":"US-CA","reviewed_at_unix":1790000000,"review_reference":"review://local/operator-simulator-flat-rate"}'::jsonb,
      now() - interval '1 minute',
      'synthetic-rule-author',
      'synthetic-rule-reviewer',
      'synthetic-rule-approver',
      4
    )
-- Published rule releases are immutable by design. Re-running local bootstrap
-- must preserve the existing release rather than attempting an update that the
-- production immutability trigger correctly rejects.
ON CONFLICT (merchant_id, external_id, version) DO NOTHING;
