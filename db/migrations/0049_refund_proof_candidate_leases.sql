-- Lease-fenced ownership for the server-owned ADR-0158 refund-proof worker.
-- Lease state is coordination only; it cannot authorize funds or lifecycle
-- transitions. The nullable expand shape keeps this migration compatible with
-- existing indexed executions while the service rolls out the fenced path.

BEGIN;

ALTER TABLE indexed_adjustment_refund_executions
    ADD COLUMN IF NOT EXISTS proof_claim_token uuid,
    ADD COLUMN IF NOT EXISTS proof_claimed_at timestamptz,
    ADD COLUMN IF NOT EXISTS proof_claim_attempts smallint NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS proof_claim_last_error text,
    ADD COLUMN IF NOT EXISTS proof_claim_dead_lettered_at timestamptz;

ALTER TABLE indexed_adjustment_refund_executions
    DROP CONSTRAINT IF EXISTS indexed_refund_proof_lease_pair,
    ADD CONSTRAINT indexed_refund_proof_lease_pair CHECK (
        (proof_claim_token IS NULL) = (proof_claimed_at IS NULL)
        AND proof_claim_attempts BETWEEN 0 AND 3
        AND (proof_claim_last_error IS NULL
             OR length(proof_claim_last_error) BETWEEN 1 AND 1024)
        AND (proof_claim_dead_lettered_at IS NULL
             OR (proof_claim_token IS NULL AND proof_claimed_at IS NULL
                 AND proof_claim_attempts = 3))
    );

CREATE INDEX IF NOT EXISTS indexed_refund_proof_candidate_idx
    ON indexed_adjustment_refund_executions
       (stream_key, proof_claimed_at, slot, signature, event_index);

COMMIT;
