-- Durable, tenant-scoped refund claim and execute intents.
-- These rows are wallet-transport and reconciliation state. Solana remains
-- authoritative for movement and finalized transaction evidence.

BEGIN;

ALTER TABLE refund_funding_configurations
    ADD COLUMN refund_funder_subject text;

ALTER TABLE refund_funding_configurations
    ADD CONSTRAINT refund_funding_configuration_funder_subject_shape
    CHECK (refund_funder_subject IS NULL
        OR length(btrim(refund_funder_subject)) BETWEEN 1 AND 256);

ALTER TABLE refund_funding_configurations
    ADD CONSTRAINT refund_funding_configuration_id_merchant_unique
    UNIQUE (id, merchant_id);

CREATE TABLE IF NOT EXISTS refund_claim_nonce_counters (
    merchant_id uuid NOT NULL,
    adjustment_id uuid NOT NULL,
    next_nonce numeric(20, 0) NOT NULL DEFAULT 1,
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (merchant_id, adjustment_id),
    CHECK (uuid_extract_version(merchant_id) = 7),
    CHECK (uuid_extract_version(adjustment_id) = 7),
    CHECK (next_nonce > 0 AND next_nonce <= 18446744073709551615)
);

CREATE TABLE IF NOT EXISTS refund_claim_intents (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    merchant_id uuid NOT NULL,
    idempotency_key text NOT NULL,
    request_digest bytea NOT NULL CHECK (octet_length(request_digest) = 32),
    adjustment_id uuid NOT NULL,
    funding_configuration_id uuid NOT NULL,
    funding_configuration_digest bytea NOT NULL CHECK (octet_length(funding_configuration_digest) = 32),
    receipt_signature text NOT NULL CHECK (length(receipt_signature) BETWEEN 1 AND 128),
    receipt_slot numeric(20, 0) NOT NULL,
    original_receipt bytea NOT NULL CHECK (octet_length(original_receipt) = 32),
    payer bytea NOT NULL CHECK (octet_length(payer) = 32),
    merchant bytea NOT NULL CHECK (octet_length(merchant) = 32),
    mint bytea NOT NULL CHECK (octet_length(mint) = 32),
    token_program text NOT NULL CHECK (length(token_program) BETWEEN 32 AND 44),
    proposer text NOT NULL CHECK (length(btrim(proposer)) BETWEEN 32 AND 44),
    refund_funder text NOT NULL CHECK (length(btrim(refund_funder)) BETWEEN 32 AND 44),
    refund_funder_subject text NOT NULL CHECK (length(btrim(refund_funder_subject)) BETWEEN 1 AND 256),
    tax_vault text NOT NULL CHECK (length(btrim(tax_vault)) BETWEEN 32 AND 44),
    net_source text NOT NULL CHECK (length(btrim(net_source)) BETWEEN 32 AND 44),
    refund_destination text NOT NULL CHECK (length(btrim(refund_destination)) BETWEEN 32 AND 44),
    claim_nonce numeric(20, 0) NOT NULL,
    gross_atomic numeric(20, 0) NOT NULL,
    tax_atomic numeric(20, 0) NOT NULL,
    net_atomic numeric(20, 0) NOT NULL,
    reference_digest bytea NOT NULL CHECK (octet_length(reference_digest) = 32),
    execute_after_unix bigint NOT NULL,
    expires_at_unix bigint NOT NULL,
    claim bytea NOT NULL CHECK (octet_length(claim) = 32),
    instruction_digest bytea NOT NULL CHECK (octet_length(instruction_digest) = 32),
    artifact_digest bytea NOT NULL CHECK (octet_length(artifact_digest) = 32),
    unsigned_artifact jsonb NOT NULL CHECK (jsonb_typeof(unsigned_artifact) = 'object'
        AND pg_column_size(unsigned_artifact) <= 65536),
    status text NOT NULL DEFAULT 'prepared_unsubmitted',
    signed_transaction_digest bytea CHECK (signed_transaction_digest IS NULL OR octet_length(signed_transaction_digest) = 32),
    signature text CHECK (signature IS NULL OR length(signature) BETWEEN 1 AND 128),
    submitted_slot numeric(20, 0),
    commitment text CHECK (commitment IS NULL OR commitment IN ('processed', 'confirmed', 'finalized')),
    last_error_code text,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK (uuid_extract_version(id) = 7),
    CHECK (uuid_extract_version(merchant_id) = 7),
    CHECK (uuid_extract_version(adjustment_id) = 7),
    CHECK (uuid_extract_version(funding_configuration_id) = 7),
    CHECK (receipt_slot > 0 AND receipt_slot <= 18446744073709551615),
    CHECK (claim_nonce > 0 AND claim_nonce <= 18446744073709551615),
    CHECK (gross_atomic > 0 AND tax_atomic >= 0 AND net_atomic >= 0
        AND tax_atomic + net_atomic = gross_atomic),
    CHECK (expires_at_unix > execute_after_unix),
    CHECK (status IN ('prepared_unsubmitted', 'signed', 'submitting', 'submitted',
                      'processed', 'confirmed', 'finalized', 'rejected',
                      'unknown', 'cancelled')),
    CHECK (status IN ('prepared_unsubmitted', 'signed') OR signature IS NOT NULL),
    CHECK (status <> 'unknown' OR last_error_code IS NOT NULL),
    UNIQUE (merchant_id, idempotency_key),
    UNIQUE (merchant_id, claim),
    UNIQUE (id, merchant_id),
    FOREIGN KEY (adjustment_id, merchant_id)
        REFERENCES settlement_adjustments(id, merchant_id),
    FOREIGN KEY (funding_configuration_id, merchant_id)
        REFERENCES refund_funding_configurations(id, merchant_id)
);

CREATE INDEX IF NOT EXISTS refund_claim_intents_status_idx
    ON refund_claim_intents (merchant_id, status, updated_at, id);

CREATE TABLE IF NOT EXISTS refund_execute_intents (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    merchant_id uuid NOT NULL,
    idempotency_key text NOT NULL,
    request_digest bytea NOT NULL CHECK (octet_length(request_digest) = 32),
    claim_intent_id uuid NOT NULL,
    funding_configuration_id uuid NOT NULL,
    funding_configuration_digest bytea NOT NULL CHECK (octet_length(funding_configuration_digest) = 32),
    claim bytea NOT NULL CHECK (octet_length(claim) = 32),
    adjustment_id uuid NOT NULL,
    original_receipt bytea NOT NULL CHECK (octet_length(original_receipt) = 32),
    merchant bytea NOT NULL CHECK (octet_length(merchant) = 32),
    mint bytea NOT NULL CHECK (octet_length(mint) = 32),
    token_program text NOT NULL CHECK (length(token_program) BETWEEN 32 AND 44),
    approver text NOT NULL CHECK (length(btrim(approver)) BETWEEN 32 AND 44),
    refund_funder text NOT NULL CHECK (length(btrim(refund_funder)) BETWEEN 32 AND 44),
    refund_funder_subject text NOT NULL CHECK (length(btrim(refund_funder_subject)) BETWEEN 1 AND 256),
    instruction_digest bytea NOT NULL CHECK (octet_length(instruction_digest) = 32),
    artifact_digest bytea NOT NULL CHECK (octet_length(artifact_digest) = 32),
    unsigned_artifact jsonb NOT NULL CHECK (jsonb_typeof(unsigned_artifact) = 'object'
        AND pg_column_size(unsigned_artifact) <= 65536),
    status text NOT NULL DEFAULT 'prepared_unsubmitted',
    signed_transaction_digest bytea CHECK (signed_transaction_digest IS NULL OR octet_length(signed_transaction_digest) = 32),
    signature text CHECK (signature IS NULL OR length(signature) BETWEEN 1 AND 128),
    submitted_slot numeric(20, 0),
    commitment text CHECK (commitment IS NULL OR commitment IN ('processed', 'confirmed', 'finalized')),
    last_error_code text,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK (uuid_extract_version(id) = 7),
    CHECK (uuid_extract_version(merchant_id) = 7),
    CHECK (uuid_extract_version(claim_intent_id) = 7),
    CHECK (uuid_extract_version(funding_configuration_id) = 7),
    CHECK (uuid_extract_version(adjustment_id) = 7),
    CHECK (status IN ('prepared_unsubmitted', 'signed', 'submitting', 'submitted',
                      'processed', 'confirmed', 'finalized', 'rejected', 'unknown',
                      'cancelled')),
    CHECK (status IN ('prepared_unsubmitted', 'signed') OR signature IS NOT NULL),
    CHECK (status <> 'unknown' OR last_error_code IS NOT NULL),
    UNIQUE (merchant_id, idempotency_key),
    UNIQUE (id, merchant_id),
    FOREIGN KEY (claim_intent_id, merchant_id)
        REFERENCES refund_claim_intents(id, merchant_id),
    FOREIGN KEY (funding_configuration_id, merchant_id)
        REFERENCES refund_funding_configurations(id, merchant_id),
    FOREIGN KEY (adjustment_id, merchant_id)
        REFERENCES settlement_adjustments(id, merchant_id)
);

CREATE INDEX IF NOT EXISTS refund_execute_intents_status_idx
    ON refund_execute_intents (merchant_id, status, updated_at, id);

CREATE TABLE IF NOT EXISTS refund_claim_intent_events (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    intent_id uuid NOT NULL,
    merchant_id uuid NOT NULL,
    from_status text NOT NULL,
    to_status text NOT NULL,
    signature text,
    slot numeric(20, 0),
    evidence jsonb NOT NULL DEFAULT '{}'::jsonb CHECK (jsonb_typeof(evidence) = 'object'
        AND pg_column_size(evidence) <= 65536),
    occurred_at timestamptz NOT NULL DEFAULT now(),
    CHECK (uuid_extract_version(id) = 7),
    FOREIGN KEY (intent_id, merchant_id) REFERENCES refund_claim_intents(id, merchant_id),
    UNIQUE (intent_id, id)
);

CREATE TABLE IF NOT EXISTS refund_execute_intent_events (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    intent_id uuid NOT NULL,
    merchant_id uuid NOT NULL,
    from_status text NOT NULL,
    to_status text NOT NULL,
    signature text,
    slot numeric(20, 0),
    evidence jsonb NOT NULL DEFAULT '{}'::jsonb CHECK (jsonb_typeof(evidence) = 'object'
        AND pg_column_size(evidence) <= 65536),
    occurred_at timestamptz NOT NULL DEFAULT now(),
    CHECK (uuid_extract_version(id) = 7),
    FOREIGN KEY (intent_id, merchant_id) REFERENCES refund_execute_intents(id, merchant_id),
    UNIQUE (intent_id, id)
);

CREATE INDEX IF NOT EXISTS refund_claim_intent_events_order_idx
    ON refund_claim_intent_events (intent_id, occurred_at, id);

CREATE INDEX IF NOT EXISTS refund_execute_intent_events_order_idx
    ON refund_execute_intent_events (intent_id, occurred_at, id);

CREATE OR REPLACE FUNCTION update_refund_intent_timestamp()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$;

CREATE OR REPLACE FUNCTION validate_refund_intent_transition()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF OLD.merchant_id IS DISTINCT FROM NEW.merchant_id
        OR OLD.idempotency_key IS DISTINCT FROM NEW.idempotency_key
        OR OLD.request_digest IS DISTINCT FROM NEW.request_digest
        OR OLD.adjustment_id IS DISTINCT FROM NEW.adjustment_id
        OR OLD.funding_configuration_id IS DISTINCT FROM NEW.funding_configuration_id
        OR OLD.funding_configuration_digest IS DISTINCT FROM NEW.funding_configuration_digest
        OR OLD.claim IS DISTINCT FROM NEW.claim
        OR OLD.instruction_digest IS DISTINCT FROM NEW.instruction_digest
        OR OLD.artifact_digest IS DISTINCT FROM NEW.artifact_digest
        OR OLD.unsigned_artifact IS DISTINCT FROM NEW.unsigned_artifact
        OR OLD.created_at IS DISTINCT FROM NEW.created_at
    THEN
        RAISE EXCEPTION 'refund intent authority and artifact fields are immutable';
    END IF;
    IF OLD.status = 'prepared_unsubmitted' AND NEW.status NOT IN ('signed', 'cancelled', 'rejected')
        OR OLD.status = 'signed' AND NEW.status NOT IN ('submitting', 'cancelled', 'rejected', 'unknown')
        OR OLD.status = 'submitting' AND NEW.status NOT IN ('submitted', 'unknown', 'rejected')
        OR OLD.status = 'submitted' AND NEW.status NOT IN ('processed', 'confirmed', 'finalized', 'unknown', 'rejected')
        OR OLD.status = 'processed' AND NEW.status NOT IN ('confirmed', 'finalized', 'unknown')
        OR OLD.status = 'confirmed' AND NEW.status NOT IN ('finalized', 'unknown')
        OR OLD.status IN ('finalized', 'rejected', 'cancelled') AND NEW.status <> OLD.status
    THEN
        RAISE EXCEPTION 'invalid refund intent status transition';
    END IF;
    NEW.updated_at = now();
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS refund_claim_intent_mutation_guard ON refund_claim_intents;
CREATE TRIGGER refund_claim_intent_mutation_guard
    BEFORE UPDATE ON refund_claim_intents
    FOR EACH ROW EXECUTE FUNCTION validate_refund_intent_transition();

DROP TRIGGER IF EXISTS refund_execute_intent_mutation_guard ON refund_execute_intents;
CREATE TRIGGER refund_execute_intent_mutation_guard
    BEFORE UPDATE ON refund_execute_intents
    FOR EACH ROW EXECUTE FUNCTION validate_refund_intent_transition();

DROP TRIGGER IF EXISTS refund_claim_intent_timestamp ON refund_claim_intents;
CREATE TRIGGER refund_claim_intent_timestamp
    BEFORE UPDATE ON refund_claim_intents
    FOR EACH ROW EXECUTE FUNCTION update_refund_intent_timestamp();

DROP TRIGGER IF EXISTS refund_execute_intent_timestamp ON refund_execute_intents;
CREATE TRIGGER refund_execute_intent_timestamp
    BEFORE UPDATE ON refund_execute_intents
    FOR EACH ROW EXECUTE FUNCTION update_refund_intent_timestamp();

COMMIT;
