-- Immutable, tenant-scoped signed calculation-attestation evidence.
-- Issuer private keys remain outside the API and database.

BEGIN;

ALTER TABLE tax_calculations
    DROP CONSTRAINT IF EXISTS tax_calculations_id_merchant_unique,
    ADD CONSTRAINT tax_calculations_id_merchant_unique UNIQUE (id, merchant_id);

CREATE TABLE IF NOT EXISTS tax_calculation_attestations (
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
        REFERENCES tax_calculations (id, merchant_id)
);

CREATE INDEX IF NOT EXISTS tax_calculation_attestations_expiry_idx
    ON tax_calculation_attestations (merchant_id, expires_at, calculation_id);

COMMIT;
