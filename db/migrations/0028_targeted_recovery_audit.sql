-- Immutable audit evidence for authenticated targeted finalized-chain recovery.
-- This records operator intent and outcome; it never becomes chain authority.

BEGIN;

CREATE TABLE IF NOT EXISTS indexer_targeted_recovery_actions (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    request_id uuid NOT NULL,
    stream_key text NOT NULL,
    target_address text NOT NULL,
    target_signature text NOT NULL,
    operator_subject text NOT NULL,
    note text NOT NULL,
    outcome text NOT NULL CHECK (outcome IN ('requested', 'recovered', 'failed')),
    pages_scanned integer CHECK (pages_scanned IS NULL OR pages_scanned BETWEEN 1 AND 64),
    slot numeric(20, 0) CHECK (slot IS NULL OR (slot >= 0 AND slot <= 18446744073709551615)),
    error_code text,
    created_at timestamptz NOT NULL DEFAULT now(),
    CHECK (uuid_extract_version(id) = 7),
    CHECK (length(stream_key) BETWEEN 1 AND 200),
    CHECK (length(target_address) BETWEEN 32 AND 64),
    CHECK (length(target_signature) BETWEEN 80 AND 128),
    CHECK (length(operator_subject) BETWEEN 1 AND 128),
    CHECK (length(note) BETWEEN 1 AND 1024),
    CHECK (error_code IS NULL OR length(error_code) BETWEEN 1 AND 128),
    CHECK ((outcome = 'requested' AND error_code IS NULL)
        OR (outcome = 'recovered' AND error_code IS NULL AND pages_scanned IS NOT NULL AND slot IS NOT NULL)
        OR (outcome = 'failed' AND error_code IS NOT NULL))
);

CREATE INDEX IF NOT EXISTS indexer_targeted_recovery_actions_target_idx
    ON indexer_targeted_recovery_actions (target_signature, created_at DESC, id DESC);

CREATE INDEX IF NOT EXISTS indexer_targeted_recovery_actions_request_idx
    ON indexer_targeted_recovery_actions (request_id, created_at DESC, id DESC);

CREATE INDEX IF NOT EXISTS indexer_targeted_recovery_actions_outcome_time_idx
    ON indexer_targeted_recovery_actions (outcome, created_at);

CREATE INDEX IF NOT EXISTS indexer_targeted_recovery_actions_outcome_time_idx
    ON indexer_targeted_recovery_actions (outcome, created_at);

CREATE OR REPLACE FUNCTION prevent_targeted_recovery_action_mutation()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    RAISE EXCEPTION 'targeted recovery actions are append-only';
END;
$$;

DROP TRIGGER IF EXISTS indexer_targeted_recovery_actions_immutable
    ON indexer_targeted_recovery_actions;
CREATE TRIGGER indexer_targeted_recovery_actions_immutable
    BEFORE UPDATE OR DELETE ON indexer_targeted_recovery_actions
    FOR EACH ROW
    EXECUTE FUNCTION prevent_targeted_recovery_action_mutation();

COMMIT;
