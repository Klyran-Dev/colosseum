-- Local-development fixture only. Never promote this file to a production
-- migration or treat it as legal/tax coverage.

INSERT INTO rule_sets
    (external_id, version, jurisdiction, regime, status, rule_digest, rule_document, provenance, effective_from)
VALUES
    (
      'operator-simulator-flat-rate',
      1,
      'US-CA',
      'sales-tax',
      'published',
      decode('f3b89c9c297c9f384e3ef1d86d90e5a89e60a5d66e57d2d29e478a00ec7523b0', 'hex'),
      '{"id":"operator-simulator-flat-rate","version":1,"components":[{"id":"sales-tax","label":"Sales tax","base":{"sources":["Net"]},"rate":{"Flat":{"numerator":"10","denominator":"100"}},"price_mode":"Exclusive","credit_atomic":"0","floor_atomic":null,"cap_atomic":null,"rounding":{"mode":"HalfUp"}}]}'::jsonb,
      '{"source":"synthetic-local-fixture","owner":"development-only"}'::jsonb,
      now() - interval '1 minute'
    )
ON CONFLICT (external_id, version) DO UPDATE
SET status = EXCLUDED.status,
    rule_digest = EXCLUDED.rule_digest,
    rule_document = EXCLUDED.rule_document,
    provenance = EXCLUDED.provenance,
    effective_from = EXCLUDED.effective_from;
