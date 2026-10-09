-- Bounded, lease-fenced jobs for the ADR-0414 finalized applicability-binding
-- verifier. Raw chain evidence and the typed verification projection remain
-- authoritative; this table is coordination and retry state only.

BEGIN;

CREATE TABLE IF NOT EXISTS applicability_binding_verification_jobs (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    stream_key text NOT NULL REFERENCES chain_checkpoints(stream_key),
    signature text NOT NULL,
    event_index smallint NOT NULL CHECK (event_index >= 0),
    attempts smallint NOT NULL DEFAULT 0 CHECK (attempts >= 0 AND attempts <= 8),
    claim_token uuid,
    claimed_at timestamptz,
    completed_at timestamptz,
    dead_lettered_at timestamptz,
    last_error_code text,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK (length(stream_key) BETWEEN 1 AND 200),
    CHECK (length(signature) BETWEEN 1 AND 128),
    CHECK (claim_token IS NULL OR uuid_extract_version(claim_token) = 7),
    CHECK (completed_at IS NULL OR dead_lettered_at IS NULL),
    UNIQUE (stream_key, signature, event_index)
);

CREATE INDEX IF NOT EXISTS applicability_binding_verification_jobs_claim_idx
    ON applicability_binding_verification_jobs
       (stream_key, completed_at, dead_lettered_at, claimed_at, created_at, id);

CREATE INDEX IF NOT EXISTS applicability_binding_verification_jobs_error_idx
    ON applicability_binding_verification_jobs (stream_key, dead_lettered_at, last_error_code);

COMMIT;
