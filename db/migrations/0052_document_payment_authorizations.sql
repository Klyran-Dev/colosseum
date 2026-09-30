-- ADR-0206: durable mapping between a document attestation and a settlement intent.

BEGIN;

ALTER TABLE settlement_intents
    DROP CONSTRAINT IF EXISTS settlement_intents_id_merchant_unique;

ALTER TABLE settlement_intents
    ADD CONSTRAINT settlement_intents_id_merchant_unique UNIQUE (id, merchant_id);

CREATE TABLE IF NOT EXISTS document_payment_authorizations (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    intent_id uuid NOT NULL,
    merchant_id uuid NOT NULL,
    calculation_id uuid NOT NULL,
    route_id uuid NOT NULL,
    mint text NOT NULL CHECK (length(mint) BETWEEN 32 AND 44),
    source_attestation_digest bytea NOT NULL CHECK (octet_length(source_attestation_digest) = 32),
    payment_scope_digest bytea NOT NULL CHECK (octet_length(payment_scope_digest) = 32),
    scope_document jsonb NOT NULL CHECK (jsonb_typeof(scope_document) = 'object'),
    gross_atomic numeric(20, 0) NOT NULL
        CHECK (gross_atomic BETWEEN 1 AND 18446744073709551615),
    tax_atomic numeric(20, 0) NOT NULL
        CHECK (tax_atomic BETWEEN 0 AND 18446744073709551615),
    net_atomic numeric(20, 0) NOT NULL
        CHECK (net_atomic BETWEEN 0 AND 18446744073709551615),
    created_at timestamptz NOT NULL DEFAULT now(),
    CHECK (uuid_extract_version(id) = 7),
    CHECK (tax_atomic + net_atomic = gross_atomic),
    UNIQUE (intent_id),
    FOREIGN KEY (intent_id, merchant_id)
        REFERENCES settlement_intents (id, merchant_id),
    FOREIGN KEY (calculation_id, merchant_id)
        REFERENCES tax_document_calculations (id, merchant_id),
    FOREIGN KEY (route_id, merchant_id)
        REFERENCES settlement_routes (id, merchant_id)
);

CREATE INDEX IF NOT EXISTS document_payment_authorizations_calculation_idx
    ON document_payment_authorizations (merchant_id, calculation_id, created_at DESC, id DESC);

CREATE INDEX IF NOT EXISTS document_payment_authorizations_scope_idx
    ON document_payment_authorizations (merchant_id, payment_scope_digest);

CREATE OR REPLACE FUNCTION prevent_document_payment_authorization_mutation()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    RAISE EXCEPTION 'document payment authorizations are immutable evidence';
END;
$$;

DROP TRIGGER IF EXISTS document_payment_authorization_immutable
    ON document_payment_authorizations;
CREATE TRIGGER document_payment_authorization_immutable
    BEFORE UPDATE OR DELETE ON document_payment_authorizations
    FOR EACH ROW
    EXECUTE FUNCTION prevent_document_payment_authorization_mutation();

COMMENT ON TABLE document_payment_authorizations IS
    'Immutable server-owned mapping from a signed document calculation to a settlement intent.';

COMMIT;
