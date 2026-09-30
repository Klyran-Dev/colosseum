-- ADR-0205 -- Dedicated document-calculation attestation evidence.
-- Aggregate attestations and document attestations are intentionally separate.

BEGIN;

ALTER TABLE tax_document_calculations
    ADD CONSTRAINT tax_document_calculations_id_merchant_unique
    UNIQUE (id, merchant_id);

CREATE TABLE IF NOT EXISTS tax_document_calculation_attestations (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    merchant_id uuid NOT NULL,
    calculation_id uuid NOT NULL,
    payload_digest bytea NOT NULL,
    issuer_public_key bytea NOT NULL,
    attestation_document jsonb NOT NULL,
    issued_at timestamptz NOT NULL,
    expires_at timestamptz NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    CHECK (uuid_extract_version(id) = 7),
    CHECK (length(payload_digest) = 32),
    CHECK (length(issuer_public_key) = 32),
    CHECK (expires_at > issued_at),
    UNIQUE (merchant_id, calculation_id),
    FOREIGN KEY (calculation_id, merchant_id)
        REFERENCES tax_document_calculations (id, merchant_id)
);

CREATE INDEX IF NOT EXISTS tax_document_calculation_attestations_expiry_idx
    ON tax_document_calculation_attestations (merchant_id, expires_at, calculation_id);

COMMENT ON TABLE tax_document_calculation_attestations IS
    'Immutable document-scoped attestation evidence; never read by aggregate settlement paths.';

COMMIT;
