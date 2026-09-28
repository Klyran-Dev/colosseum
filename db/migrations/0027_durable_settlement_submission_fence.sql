-- Fence concurrent settlement submission before external RPC I/O.
-- A submitting intent already has the verified transaction signature bound to it.

BEGIN;

ALTER TABLE settlement_intents
    DROP CONSTRAINT IF EXISTS settlement_intents_status_check;

ALTER TABLE settlement_intents
    ADD CONSTRAINT settlement_intents_status_check CHECK (
        status IN ('prepared_unsubmitted', 'signed', 'submitting', 'submitted',
                   'processed', 'confirmed', 'finalized', 'rejected', 'unknown')
    );

ALTER TABLE settlement_intents
    ADD CONSTRAINT settlement_intents_submitting_signature_check CHECK (
        status <> 'submitting' OR signature IS NOT NULL
    );

ALTER TABLE settlement_intents
    ADD CONSTRAINT settlement_intents_chain_status_signature_check CHECK (
        status IN ('prepared_unsubmitted', 'signed') OR signature IS NOT NULL
    ) NOT VALID,
    ADD CONSTRAINT settlement_intents_unknown_error_check CHECK (
        status <> 'unknown' OR last_error_code IS NOT NULL
    ) NOT VALID;

ALTER TABLE settlement_intent_events
    DROP CONSTRAINT IF EXISTS settlement_intent_events_from_status_check,
    DROP CONSTRAINT IF EXISTS settlement_intent_events_to_status_check;

ALTER TABLE settlement_intent_events
    ADD CONSTRAINT settlement_intent_events_from_status_check CHECK (
        from_status IN ('prepared_unsubmitted', 'signed', 'submitting',
                        'submitted', 'processed', 'confirmed', 'finalized',
                        'rejected', 'unknown')
    ),
    ADD CONSTRAINT settlement_intent_events_to_status_check CHECK (
        to_status IN ('prepared_unsubmitted', 'signed', 'submitting',
                      'submitted', 'processed', 'confirmed', 'finalized',
                      'rejected', 'unknown')
    );

COMMIT;
