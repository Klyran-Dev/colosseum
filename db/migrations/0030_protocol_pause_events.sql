-- Finalized, append-only governance evidence for the on-chain protocol pause.
-- The latest row for a stream is an operational projection; the chain remains
-- authoritative and the event evidence is retained for incident replay.

BEGIN;

CREATE TABLE IF NOT EXISTS protocol_pause_events (
    stream_key text NOT NULL REFERENCES chain_checkpoints(stream_key),
    signature text NOT NULL,
    slot numeric(20, 0) NOT NULL CHECK (slot >= 0 AND slot <= 18446744073709551615),
    event_index smallint NOT NULL CHECK (event_index >= 0),
    authority bytea NOT NULL CHECK (octet_length(authority) = 32),
    paused boolean NOT NULL,
    evidence_digest bytea NOT NULL CHECK (octet_length(evidence_digest) = 32),
    observed_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (stream_key, signature, event_index),
    CHECK (length(signature) BETWEEN 1 AND 128)
);

CREATE INDEX IF NOT EXISTS protocol_pause_events_latest_idx
    ON protocol_pause_events (stream_key, slot DESC, event_index DESC);

CREATE OR REPLACE FUNCTION prevent_protocol_pause_event_mutation()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    RAISE EXCEPTION 'protocol pause events are append-only';
END;
$$;

DROP TRIGGER IF EXISTS protocol_pause_events_immutable
    ON protocol_pause_events;
CREATE TRIGGER protocol_pause_events_immutable
    BEFORE UPDATE OR DELETE ON protocol_pause_events
    FOR EACH ROW
    EXECUTE FUNCTION prevent_protocol_pause_event_mutation();

COMMIT;
