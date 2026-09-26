-- Bounded, crash-recoverable delivery leases for the transactional outbox.

ALTER TABLE outbox_events
    ADD COLUMN IF NOT EXISTS claim_token uuid,
    ADD COLUMN IF NOT EXISTS claimed_at timestamptz,
    ADD COLUMN IF NOT EXISTS dead_lettered_at timestamptz,
    ADD COLUMN IF NOT EXISTS last_error_code text;

ALTER TABLE outbox_events
    DROP CONSTRAINT IF EXISTS outbox_last_error_code_length_check;

ALTER TABLE outbox_events
    ADD CONSTRAINT outbox_last_error_code_length_check
    CHECK (last_error_code IS NULL OR length(last_error_code) BETWEEN 1 AND 128);

CREATE INDEX IF NOT EXISTS outbox_dispatch_idx
    ON outbox_events (occurred_at, id)
    WHERE published_at IS NULL AND dead_lettered_at IS NULL;
