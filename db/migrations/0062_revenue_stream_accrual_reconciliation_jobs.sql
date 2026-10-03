-- Durable retry records for finalized split settlements. The job is created
-- in the same transaction as indexed_split_settlements and the chain cursor;
-- it is coordination only and never authorizes token movement.

BEGIN;

CREATE TABLE IF NOT EXISTS revenue_stream_accrual_reconciliation_jobs (
    stream_key text NOT NULL,
    source_signature text NOT NULL,
    evidence_digest bytea NOT NULL,
    observed_finalized_slot numeric(20, 0) NOT NULL,
    claim_token uuid,
    claimed_at timestamptz,
    attempts smallint NOT NULL DEFAULT 0,
    last_error_code text,
    completed_at timestamptz,
    dead_lettered_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (stream_key, source_signature),
    FOREIGN KEY (stream_key, source_signature)
        REFERENCES indexed_split_settlements(stream_key, signature),
    CHECK (length(source_signature) BETWEEN 1 AND 128),
    CHECK (octet_length(evidence_digest) = 32 AND evidence_digest <> repeat(E'\\000', 32)::bytea),
    CHECK (observed_finalized_slot > 0 AND observed_finalized_slot <= 18446744073709551615),
    CHECK (claim_token IS NULL OR uuid_extract_version(claim_token) = 7),
    CHECK (attempts BETWEEN 0 AND 8),
    CHECK (last_error_code IS NULL OR length(btrim(last_error_code)) BETWEEN 1 AND 200),
    CHECK ((claim_token IS NULL) = (claimed_at IS NULL)),
    CHECK (completed_at IS NULL OR (claim_token IS NULL AND claimed_at IS NULL)),
    CHECK (dead_lettered_at IS NULL OR (completed_at IS NULL AND claim_token IS NULL AND claimed_at IS NULL))
);

CREATE INDEX IF NOT EXISTS revenue_stream_accrual_jobs_pending_idx
    ON revenue_stream_accrual_reconciliation_jobs (created_at, stream_key, source_signature)
    WHERE completed_at IS NULL AND dead_lettered_at IS NULL;

COMMIT;
