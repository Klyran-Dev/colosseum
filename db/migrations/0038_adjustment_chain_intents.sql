-- Retryable wallet-submission lifecycle for the immutable adjustment evidence
-- intent. This table is transport/reconciliation state, not chain authority.

BEGIN;

ALTER TABLE settlement_adjustment_evidence_intents
    ADD CONSTRAINT settlement_adjustment_evidence_intent_id_merchant_unique
    UNIQUE (id, merchant_id);

CREATE TABLE IF NOT EXISTS settlement_adjustment_chain_intents (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    evidence_intent_id uuid NOT NULL,
    adjustment_id uuid NOT NULL,
    merchant_id uuid NOT NULL,
    instruction_digest bytea NOT NULL CHECK (octet_length(instruction_digest) = 32),
    status text NOT NULL DEFAULT 'prepared_unsubmitted',
    signed_transaction_digest bytea,
    signature text,
    slot numeric(20, 0),
    commitment text,
    last_error_code text,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK (uuid_extract_version(id) = 7),
    CHECK (uuid_extract_version(evidence_intent_id) = 7),
    CHECK (uuid_extract_version(adjustment_id) = 7),
    CHECK (octet_length(instruction_digest) = 32),
    CHECK (signed_transaction_digest IS NULL OR octet_length(signed_transaction_digest) = 32),
    CHECK (signature IS NULL OR length(signature) BETWEEN 1 AND 128),
    CHECK (slot IS NULL OR (slot >= 0 AND slot <= 18446744073709551615)),
    CHECK (commitment IS NULL OR commitment IN ('processed', 'confirmed', 'finalized')),
    CHECK (status IN ('prepared_unsubmitted', 'signed', 'submitting', 'submitted',
                      'processed', 'confirmed', 'finalized', 'rejected', 'unknown')),
    CHECK (status IN ('prepared_unsubmitted', 'signed') OR signature IS NOT NULL),
    CHECK (status <> 'unknown' OR last_error_code IS NOT NULL),
    FOREIGN KEY (evidence_intent_id, merchant_id)
        REFERENCES settlement_adjustment_evidence_intents(id, merchant_id),
    FOREIGN KEY (adjustment_id, merchant_id) REFERENCES settlement_adjustments(id, merchant_id),
    UNIQUE (evidence_intent_id, merchant_id),
    UNIQUE (id, merchant_id)
);

CREATE UNIQUE INDEX IF NOT EXISTS settlement_adjustment_chain_signature_idx
    ON settlement_adjustment_chain_intents (merchant_id, signature)
    WHERE signature IS NOT NULL;

CREATE INDEX IF NOT EXISTS settlement_adjustment_chain_status_idx
    ON settlement_adjustment_chain_intents (merchant_id, status, updated_at, id);

CREATE TABLE IF NOT EXISTS settlement_adjustment_chain_intent_events (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    intent_id uuid NOT NULL,
    merchant_id uuid NOT NULL,
    from_status text NOT NULL,
    to_status text NOT NULL,
    signature text,
    slot numeric(20, 0),
    evidence jsonb NOT NULL DEFAULT '{}'::jsonb,
    occurred_at timestamptz NOT NULL DEFAULT now(),
    CHECK (uuid_extract_version(id) = 7),
    CHECK (jsonb_typeof(evidence) = 'object'),
    FOREIGN KEY (intent_id, merchant_id)
        REFERENCES settlement_adjustment_chain_intents(id, merchant_id),
    UNIQUE (intent_id, id)
);

CREATE INDEX IF NOT EXISTS settlement_adjustment_chain_events_order_idx
    ON settlement_adjustment_chain_intent_events (intent_id, occurred_at, id);

CREATE OR REPLACE FUNCTION update_settlement_adjustment_chain_intent_timestamp()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS settlement_adjustment_chain_intent_timestamp
    ON settlement_adjustment_chain_intents;
CREATE TRIGGER settlement_adjustment_chain_intent_timestamp
    BEFORE UPDATE ON settlement_adjustment_chain_intents
    FOR EACH ROW EXECUTE FUNCTION update_settlement_adjustment_chain_intent_timestamp();

COMMIT;
