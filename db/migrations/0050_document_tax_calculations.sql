-- ADR-0204: dedicated authoritative document quote evidence.
-- Document calculations must never be silently decoded as aggregate quotes.

BEGIN;

CREATE TABLE IF NOT EXISTS tax_document_calculations (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    correlation_id uuid NOT NULL DEFAULT uuidv7(),
    merchant_id uuid NOT NULL,
    rule_set_id uuid NOT NULL REFERENCES rule_sets(id),
    document_schema text NOT NULL
        CHECK (document_schema = 'solana-tax-protocol/tax-document/v1'),
    document_id text NOT NULL CHECK (length(document_id) BETWEEN 1 AND 64),
    facts_digest bytea NOT NULL CHECK (octet_length(facts_digest) = 32),
    rule_digest bytea NOT NULL CHECK (octet_length(rule_digest) = 32),
    engine_version text NOT NULL CHECK (length(engine_version) BETWEEN 1 AND 128),
    currency char(3) NOT NULL,
    jurisdiction text NOT NULL CHECK (length(jurisdiction) BETWEEN 1 AND 64),
    gross_atomic numeric(20, 0) NOT NULL
        CHECK (gross_atomic BETWEEN 0 AND 18446744073709551615),
    total_tax_atomic numeric(20, 0) NOT NULL
        CHECK (total_tax_atomic BETWEEN 0 AND 18446744073709551615),
    total_net_atomic numeric(20, 0) NOT NULL
        CHECK (total_net_atomic BETWEEN 0 AND 18446744073709551615),
    status text NOT NULL CHECK (status IN ('calculated_unattested', 'attested', 'expired', 'superseded')),
    result_document jsonb NOT NULL CHECK (jsonb_typeof(result_document) = 'object'),
    facts_ciphertext bytea NOT NULL,
    facts_nonce bytea NOT NULL CHECK (octet_length(facts_nonce) = 12),
    facts_key_id text NOT NULL CHECK (length(facts_key_id) BETWEEN 1 AND 128),
    evidence_refs jsonb NOT NULL CHECK (jsonb_typeof(evidence_refs) = 'object'),
    created_at timestamptz NOT NULL DEFAULT now(),
    expires_at timestamptz NOT NULL,
    CHECK (uuid_extract_version(id) = 7),
    CHECK (uuid_extract_version(correlation_id) = 7),
    CHECK (total_tax_atomic <= gross_atomic),
    CHECK (total_net_atomic = gross_atomic - total_tax_atomic)
);

CREATE INDEX IF NOT EXISTS tax_document_calculations_merchant_created_idx
    ON tax_document_calculations (merchant_id, created_at DESC, id DESC);

CREATE INDEX IF NOT EXISTS tax_document_calculations_document_digest_idx
    ON tax_document_calculations (merchant_id, document_id, facts_digest);

CREATE INDEX IF NOT EXISTS tax_document_calculations_rule_idx
    ON tax_document_calculations (merchant_id, rule_set_id, created_at DESC);

COMMENT ON TABLE tax_document_calculations IS
    'Tenant-scoped immutable document quote evidence; never decode as aggregate tax_calculations.';

COMMIT;
