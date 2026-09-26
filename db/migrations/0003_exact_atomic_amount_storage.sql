-- PostgreSQL bigint is signed and cannot represent the complete Solana u64
-- atomic-unit domain. Preserve exact values through numeric(20,0).

BEGIN;

ALTER TABLE tax_calculations
    ALTER COLUMN total_tax_atomic TYPE numeric(20, 0)
    USING total_tax_atomic::numeric;

ALTER TABLE tax_calculations
    ALTER COLUMN total_net_atomic TYPE numeric(20, 0)
    USING total_net_atomic::numeric;

ALTER TABLE rule_sets
    DROP CONSTRAINT IF EXISTS rule_sets_rule_digest_length_check;

ALTER TABLE rule_sets
    ADD CONSTRAINT rule_sets_rule_digest_length_check
    CHECK (octet_length(rule_digest) = 32);

ALTER TABLE tax_calculations
    DROP CONSTRAINT IF EXISTS tax_calculations_facts_digest_length_check;

ALTER TABLE tax_calculations
    ADD CONSTRAINT tax_calculations_facts_digest_length_check
    CHECK (octet_length(facts_digest) = 32);

COMMIT;
