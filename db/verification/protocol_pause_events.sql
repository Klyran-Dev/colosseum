-- Prove that the finalized protocol-pause evidence projection is append-only.

BEGIN;

INSERT INTO chain_checkpoints (stream_key, next_slot, finalized_slot, generation)
VALUES ('verification/protocol-pause', 1, 1, 0)
ON CONFLICT (stream_key) DO NOTHING;

INSERT INTO protocol_pause_events
    (stream_key, signature, slot, event_index, authority, paused, evidence_digest)
VALUES
    ('verification/protocol-pause',
     'protocol-pause-verification-signature', 1, 0,
     decode(repeat('11', 32), 'hex'), false,
     decode(repeat('22', 32), 'hex'))
ON CONFLICT (stream_key, signature, event_index) DO NOTHING;

DO $$
BEGIN
    UPDATE protocol_pause_events
       SET paused = true
     WHERE stream_key = 'verification/protocol-pause'
       AND signature = 'protocol-pause-verification-signature'
       AND event_index = 0;
    RAISE EXCEPTION 'protocol pause update guard did not reject mutation';
EXCEPTION
    WHEN OTHERS THEN
        IF SQLERRM <> 'protocol pause events are append-only' THEN
            RAISE;
        END IF;
END;
$$;

DO $$
BEGIN
    DELETE FROM protocol_pause_events
     WHERE stream_key = 'verification/protocol-pause'
       AND signature = 'protocol-pause-verification-signature'
       AND event_index = 0;
    RAISE EXCEPTION 'protocol pause delete guard did not reject mutation';
EXCEPTION
    WHEN OTHERS THEN
        IF SQLERRM <> 'protocol pause events are append-only' THEN
            RAISE;
        END IF;
END;
$$;

ROLLBACK;
