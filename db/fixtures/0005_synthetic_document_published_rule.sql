-- Local-development fixture only. This rule exists solely to exercise the
-- production document quote path with explicit line-scope rounding.

INSERT INTO rule_sets
    (id, merchant_id, external_id, version, jurisdiction, regime, status,
     rule_digest, rule_document, provenance, effective_from,
     author_subject, reviewer_subject, approver_subject, governance_revision)
VALUES
    (
      '01992f1b-4a70-7e30-8ea2-b6e37a9d9d13',
      '01992f1b-4a70-7e30-8ea2-b6e37a9d9d11',
      'operator-document-flat-rate',
      1,
      'US-CA',
      'sales-tax',
      'published',
      decode('9364c26c91735950f09995028e11074006ea04d6493dfa6bfce1feb773c0f649', 'hex'),
      '{"schema":"solana-tax-protocol/tax-rule/v1","rule_set":{"id":"operator-document-flat-rate","version":1,"components":[{"id":"sales-tax","label":"Sales tax","base":{"sources":["Net"]},"rate":{"Flat":{"numerator":"10","denominator":"100"}},"price_mode":"Exclusive","credit_atomic":"0","floor_atomic":null,"cap_atomic":null,"rounding":{"mode":"HalfUp","scope":"Line"},"threshold":null,"exemption_codes":[]}]}}'::jsonb,
      '{"schema":"tax-rule-provenance/v1","source_reference":"controlled-record://local/operator-document-flat-rate","source_digest_hex":"9364c26c91735950f09995028e11074006ea04d6493dfa6bfce1feb773c0f649","legal_tax_owner":"tax-owner:synthetic-local","jurisdiction_scope":"US-CA","reviewed_at_unix":1790000000,"review_reference":"review://local/operator-document-flat-rate"}'::jsonb,
      now() - interval '1 minute',
      'synthetic-document-rule-author',
      'synthetic-document-rule-reviewer',
      'synthetic-document-rule-approver',
      1
    )
ON CONFLICT (merchant_id, external_id, version) DO NOTHING;
