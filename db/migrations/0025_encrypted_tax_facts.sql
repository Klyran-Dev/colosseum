-- Expand phase for privacy-preserving tax-calculation replay.
-- New calculations persist normalized facts only as authenticated ciphertext.
-- Legacy facts_document rows remain readable during the compatibility window.
ALTER TABLE tax_calculations
    ADD COLUMN IF NOT EXISTS facts_ciphertext bytea,
    ADD COLUMN IF NOT EXISTS facts_nonce bytea,
    ADD COLUMN IF NOT EXISTS facts_key_id text;

ALTER TABLE tax_calculations
    DROP CONSTRAINT IF EXISTS tax_calculations_encrypted_facts_shape_check;

ALTER TABLE tax_calculations
    ADD CONSTRAINT tax_calculations_encrypted_facts_shape_check
    CHECK (
        (facts_ciphertext IS NULL AND facts_nonce IS NULL AND facts_key_id IS NULL)
        OR (
            octet_length(facts_ciphertext) BETWEEN 17 AND 65552
            AND octet_length(facts_nonce) = 12
            AND length(facts_key_id) BETWEEN 1 AND 128
        )
    );

COMMENT ON COLUMN tax_calculations.facts_ciphertext IS
    'Authenticated encrypted normalized TaxFacts evidence; plaintext facts_document is legacy-only.';
COMMENT ON COLUMN tax_calculations.facts_nonce IS
    'Per-record 96-bit AEAD nonce for facts_ciphertext.';
COMMENT ON COLUMN tax_calculations.facts_key_id IS
    'KMS/HSM key identifier required to decrypt facts_ciphertext.';
