-- Authenticated indexer-operations evidence. Replay gaps remain recovery work;
-- operator acknowledgement is never equivalent to finalized chain repair.

BEGIN;

ALTER TABLE indexer_replay_gaps
    ADD COLUMN IF NOT EXISTS operator_acknowledged_at timestamptz,
    ADD COLUMN IF NOT EXISTS operator_acknowledged_by text,
    ADD COLUMN IF NOT EXISTS operator_acknowledgement text;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'indexer_replay_gaps_operator_ack_shape_check'
    ) THEN
        ALTER TABLE indexer_replay_gaps
            ADD CONSTRAINT indexer_replay_gaps_operator_ack_shape_check
            CHECK (
                (operator_acknowledged_at IS NULL
                 AND operator_acknowledged_by IS NULL
                 AND operator_acknowledgement IS NULL)
                OR
                (operator_acknowledged_at IS NOT NULL
                 AND operator_acknowledged_by IS NOT NULL
                 AND operator_acknowledgement IS NOT NULL)
            );
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'indexer_replay_gaps_operator_subject_length_check'
    ) THEN
        ALTER TABLE indexer_replay_gaps
            ADD CONSTRAINT indexer_replay_gaps_operator_subject_length_check
            CHECK (operator_acknowledged_by IS NULL
                   OR length(operator_acknowledged_by) BETWEEN 1 AND 128);
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'indexer_replay_gaps_operator_note_length_check'
    ) THEN
        ALTER TABLE indexer_replay_gaps
            ADD CONSTRAINT indexer_replay_gaps_operator_note_length_check
            CHECK (operator_acknowledgement IS NULL
                   OR length(operator_acknowledgement) BETWEEN 1 AND 1024);
    END IF;
END
$$;

CREATE TABLE IF NOT EXISTS replay_gap_operator_actions (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    gap_id uuid NOT NULL REFERENCES indexer_replay_gaps(id) ON DELETE RESTRICT,
    action text NOT NULL CHECK (action IN ('acknowledge', 'requeue')),
    operator_subject text NOT NULL CHECK (length(operator_subject) BETWEEN 1 AND 128),
    note text NOT NULL CHECK (length(note) BETWEEN 1 AND 1024),
    created_at timestamptz NOT NULL DEFAULT now(),
    CHECK (uuid_extract_version(id) = 7)
);

CREATE INDEX IF NOT EXISTS replay_gap_operator_actions_gap_idx
    ON replay_gap_operator_actions (gap_id, created_at DESC, id DESC);

CREATE OR REPLACE FUNCTION prevent_replay_gap_operator_action_mutation()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    RAISE EXCEPTION 'replay gap operator actions are append-only';
END;
$$;

DROP TRIGGER IF EXISTS replay_gap_operator_actions_immutable
    ON replay_gap_operator_actions;
CREATE TRIGGER replay_gap_operator_actions_immutable
    BEFORE UPDATE OR DELETE ON replay_gap_operator_actions
    FOR EACH ROW
    EXECUTE FUNCTION prevent_replay_gap_operator_action_mutation();

COMMIT;
