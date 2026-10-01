-- Server-owned, tenant-scoped cancellation intents. A cancellation artifact is
-- deliberately separate from the immutable refund movement artifact and has no
-- financial values or token-account topology.

BEGIN;

CREATE TABLE IF NOT EXISTS refund_claim_cancellation_intents (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    merchant_id uuid NOT NULL,
    idempotency_key text NOT NULL,
    request_digest bytea NOT NULL CHECK (octet_length(request_digest) = 32),
    claim_intent_id uuid NOT NULL,
    adjustment_id uuid NOT NULL,
    merchant bytea NOT NULL CHECK (octet_length(merchant) = 32),
    mint bytea NOT NULL CHECK (octet_length(mint) = 32),
    adjustment bytea NOT NULL CHECK (octet_length(adjustment) = 32),
    claim_nonce numeric(20, 0) NOT NULL,
    claim bytea NOT NULL CHECK (octet_length(claim) = 32),
    governance_authority text NOT NULL CHECK (length(btrim(governance_authority)) BETWEEN 32 AND 44),
    governance_subject text NOT NULL CHECK (length(btrim(governance_subject)) BETWEEN 1 AND 256),
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
    CHECK (uuid_extract_version(adjustment_id) = 7),
    CHECK (claim_nonce > 0 AND claim_nonce <= 18446744073709551615),
    CHECK (status IN ('prepared_unsubmitted', 'signed', 'submitting', 'submitted',
                      'processed', 'confirmed', 'finalized', 'rejected', 'unknown')),
    CHECK (status IN ('prepared_unsubmitted', 'signed') OR signature IS NOT NULL),
    CHECK (status <> 'unknown' OR last_error_code IS NOT NULL),
    UNIQUE (merchant_id, idempotency_key),
    UNIQUE (merchant_id, claim_intent_id),
    UNIQUE (id, merchant_id),
    FOREIGN KEY (claim_intent_id, merchant_id)
        REFERENCES refund_claim_intents(id, merchant_id),
    FOREIGN KEY (adjustment_id, merchant_id)
        REFERENCES settlement_adjustments(id, merchant_id)
);

CREATE INDEX IF NOT EXISTS refund_claim_cancellation_intents_status_idx
    ON refund_claim_cancellation_intents (merchant_id, status, updated_at, id);

CREATE TABLE IF NOT EXISTS refund_claim_cancellation_intent_events (
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
    FOREIGN KEY (intent_id, merchant_id)
        REFERENCES refund_claim_cancellation_intents(id, merchant_id),
    UNIQUE (intent_id, id)
);

CREATE INDEX IF NOT EXISTS refund_claim_cancellation_events_order_idx
    ON refund_claim_cancellation_intent_events (intent_id, occurred_at, id);

CREATE OR REPLACE FUNCTION update_refund_claim_cancellation_timestamp()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$;

CREATE OR REPLACE FUNCTION validate_refund_claim_cancellation_transition()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF OLD.merchant_id IS DISTINCT FROM NEW.merchant_id
        OR OLD.idempotency_key IS DISTINCT FROM NEW.idempotency_key
        OR OLD.request_digest IS DISTINCT FROM NEW.request_digest
        OR OLD.claim_intent_id IS DISTINCT FROM NEW.claim_intent_id
        OR OLD.adjustment_id IS DISTINCT FROM NEW.adjustment_id
        OR OLD.merchant IS DISTINCT FROM NEW.merchant
        OR OLD.mint IS DISTINCT FROM NEW.mint
        OR OLD.adjustment IS DISTINCT FROM NEW.adjustment
        OR OLD.claim_nonce IS DISTINCT FROM NEW.claim_nonce
        OR OLD.claim IS DISTINCT FROM NEW.claim
        OR OLD.governance_authority IS DISTINCT FROM NEW.governance_authority
        OR OLD.governance_subject IS DISTINCT FROM NEW.governance_subject
        OR OLD.instruction_digest IS DISTINCT FROM NEW.instruction_digest
        OR OLD.artifact_digest IS DISTINCT FROM NEW.artifact_digest
        OR OLD.unsigned_artifact IS DISTINCT FROM NEW.unsigned_artifact
        OR OLD.created_at IS DISTINCT FROM NEW.created_at
    THEN
        RAISE EXCEPTION 'refund claim cancellation authority and artifact fields are immutable';
    END IF;
    IF OLD.status = 'prepared_unsubmitted' AND NEW.status NOT IN ('signed', 'rejected')
        OR OLD.status = 'signed' AND NEW.status NOT IN ('submitting', 'rejected', 'unknown')
        OR OLD.status = 'submitting' AND NEW.status NOT IN ('submitted', 'unknown', 'rejected')
        OR OLD.status = 'submitted' AND NEW.status NOT IN ('processed', 'confirmed', 'finalized', 'unknown', 'rejected')
        OR OLD.status = 'processed' AND NEW.status NOT IN ('confirmed', 'finalized', 'unknown')
        OR OLD.status = 'confirmed' AND NEW.status NOT IN ('finalized', 'unknown')
        OR OLD.status = 'unknown' AND NEW.status NOT IN ('submitted', 'processed', 'confirmed', 'finalized', 'rejected')
        OR OLD.status IN ('finalized', 'rejected') AND NEW.status <> OLD.status
    THEN
        RAISE EXCEPTION 'invalid refund claim cancellation status transition';
    END IF;
    NEW.updated_at = now();
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS refund_claim_cancellation_mutation_guard
    ON refund_claim_cancellation_intents;
CREATE TRIGGER refund_claim_cancellation_mutation_guard
    BEFORE UPDATE ON refund_claim_cancellation_intents
    FOR EACH ROW EXECUTE FUNCTION validate_refund_claim_cancellation_transition();

DROP TRIGGER IF EXISTS refund_claim_cancellation_timestamp
    ON refund_claim_cancellation_intents;
CREATE TRIGGER refund_claim_cancellation_timestamp
    BEFORE UPDATE ON refund_claim_cancellation_intents
    FOR EACH ROW EXECUTE FUNCTION update_refund_claim_cancellation_timestamp();

COMMIT;
