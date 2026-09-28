-- Durable, non-custodial settlement intent and append-only lifecycle evidence.
-- Signed transaction bytes are never stored here; only their digest is kept.

BEGIN;

CREATE TABLE IF NOT EXISTS settlement_intents (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    merchant_id uuid NOT NULL,
    idempotency_key text NOT NULL,
    request_digest bytea NOT NULL,
    instruction_digest bytea NOT NULL,
    status text NOT NULL DEFAULT 'prepared_unsubmitted',
    signed_transaction_digest bytea,
    signature text,
    slot numeric(20, 0),
    commitment text,
    last_error_code text,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK (uuid_extract_version(id) = 7),
    CHECK (length(idempotency_key) BETWEEN 1 AND 200),
    CHECK (octet_length(request_digest) = 32),
    CHECK (octet_length(instruction_digest) = 32),
    CHECK (signed_transaction_digest IS NULL OR octet_length(signed_transaction_digest) = 32),
    CHECK (status IN ('prepared_unsubmitted', 'signed', 'submitted', 'processed',
                      'confirmed', 'finalized', 'rejected', 'unknown')),
    CHECK (slot IS NULL OR (slot >= 0 AND slot <= 18446744073709551615)),
    CHECK (commitment IS NULL OR commitment IN ('processed', 'confirmed', 'finalized')),
    CHECK (signature IS NULL OR length(signature) BETWEEN 1 AND 128),
    CHECK (last_error_code IS NULL OR length(last_error_code) BETWEEN 1 AND 128)
);

CREATE UNIQUE INDEX IF NOT EXISTS settlement_intents_merchant_key_idx
    ON settlement_intents (merchant_id, idempotency_key);
CREATE INDEX IF NOT EXISTS settlement_intents_status_updated_idx
    ON settlement_intents (status, updated_at, id);
CREATE INDEX IF NOT EXISTS settlement_intents_signature_idx
    ON settlement_intents (signature) WHERE signature IS NOT NULL;

CREATE TABLE IF NOT EXISTS settlement_intent_events (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    intent_id uuid NOT NULL REFERENCES settlement_intents(id),
    from_status text NOT NULL,
    to_status text NOT NULL,
    signature text,
    slot numeric(20, 0),
    evidence jsonb NOT NULL DEFAULT '{}'::jsonb,
    occurred_at timestamptz NOT NULL DEFAULT now(),
    CHECK (uuid_extract_version(id) = 7),
    CHECK (from_status IN ('prepared_unsubmitted', 'signed', 'submitted', 'processed',
                           'confirmed', 'finalized', 'rejected', 'unknown')),
    CHECK (to_status IN ('prepared_unsubmitted', 'signed', 'submitted', 'processed',
                         'confirmed', 'finalized', 'rejected', 'unknown')),
    CHECK (signature IS NULL OR length(signature) BETWEEN 1 AND 128),
    CHECK (slot IS NULL OR (slot >= 0 AND slot <= 18446744073709551615))
);

CREATE INDEX IF NOT EXISTS settlement_intent_events_intent_time_idx
    ON settlement_intent_events (intent_id, occurred_at, id);

COMMIT;
