-- Persist the exact finalized typed-event identity needed by the API/UI proof
-- envelope. Existing finalized rows without these fields remain readable as
-- incomplete and fail closed at the API boundary until independently
-- reconciled; no historical status is upgraded by this migration.

BEGIN;

ALTER TABLE settlement_adjustment_chain_intents
    ADD COLUMN finalized_event_index smallint,
    ADD COLUMN finalized_event_evidence_digest bytea;

ALTER TABLE settlement_adjustment_chain_intents
    ADD CONSTRAINT settlement_adjustment_chain_finalized_event_index_shape
    CHECK (finalized_event_index IS NULL OR finalized_event_index >= 0),
    ADD CONSTRAINT settlement_adjustment_chain_finalized_event_digest_shape
    CHECK (finalized_event_evidence_digest IS NULL
        OR octet_length(finalized_event_evidence_digest) = 32);

CREATE INDEX IF NOT EXISTS settlement_adjustment_chain_finalized_proof_idx
    ON settlement_adjustment_chain_intents (merchant_id, status, signature, slot)
    WHERE status = 'finalized';

COMMIT;
