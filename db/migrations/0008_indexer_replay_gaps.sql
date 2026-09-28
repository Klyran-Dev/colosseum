-- Crash-recoverable replay work. This is an operational ledger, not a source
-- of truth for settled funds; finalized chain evidence remains authoritative.

CREATE TABLE IF NOT EXISTS indexer_replay_gaps (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    stream_key text NOT NULL,
    start_slot numeric(20, 0) NOT NULL CHECK (start_slot >= 0 AND start_slot <= 18446744073709551615),
    end_slot numeric(20, 0) NOT NULL CHECK (end_slot >= 0 AND end_slot <= 18446744073709551615),
    observed_finalized_slot numeric(20, 0) NOT NULL CHECK (observed_finalized_slot >= 0 AND observed_finalized_slot <= 18446744073709551615),
    reason text NOT NULL CHECK (reason IN ('incomplete_provider_page', 'provider_disagreement', 'projection_commit_failure', 'operator_requested')),
    attempts smallint NOT NULL DEFAULT 0 CHECK (attempts >= 0 AND attempts <= 8),
    status text NOT NULL DEFAULT 'queued' CHECK (status IN ('queued', 'leased', 'completed', 'dead_lettered')),
    claim_token uuid,
    claimed_at timestamptz,
    last_error_code text,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    completed_at timestamptz,
    dead_lettered_at timestamptz,
    CHECK (start_slot <= end_slot),
    CHECK (uuid_extract_version(id) = 7),
    CHECK (length(stream_key) BETWEEN 1 AND 200),
    CHECK (last_error_code IS NULL OR length(last_error_code) BETWEEN 1 AND 128)
);

CREATE INDEX IF NOT EXISTS indexer_replay_gaps_claim_idx
    ON indexer_replay_gaps (stream_key, status, created_at, id)
    WHERE status IN ('queued', 'leased');

CREATE INDEX IF NOT EXISTS indexer_replay_gaps_dead_letter_idx
    ON indexer_replay_gaps (stream_key, dead_lettered_at DESC, id DESC)
    WHERE status = 'dead_lettered';

CREATE UNIQUE INDEX IF NOT EXISTS indexer_replay_gaps_active_identity_idx
    ON indexer_replay_gaps (stream_key, start_slot, end_slot, reason)
    WHERE status IN ('queued', 'leased');
