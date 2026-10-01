-- Bound cross-replica ownership for automatic finalized cancellation
-- reconciliation. The lease is coordination only; it never authorizes a
-- cancellation and is reclaimed after a bounded worker crash window.

BEGIN;

ALTER TABLE refund_claim_cancellation_intents
    ADD COLUMN IF NOT EXISTS reconciliation_claim_token uuid,
    ADD COLUMN IF NOT EXISTS reconciliation_claimed_at timestamptz;

ALTER TABLE refund_claim_cancellation_intents
    DROP CONSTRAINT IF EXISTS refund_claim_cancellation_reconciliation_lease_pair,
    ADD CONSTRAINT refund_claim_cancellation_reconciliation_lease_pair
        CHECK ((reconciliation_claim_token IS NULL)
            = (reconciliation_claimed_at IS NULL));

ALTER TABLE refund_claim_cancellation_intents
    DROP CONSTRAINT IF EXISTS refund_claim_cancellation_reconciliation_claim_token_v7,
    ADD CONSTRAINT refund_claim_cancellation_reconciliation_claim_token_v7
        CHECK (reconciliation_claim_token IS NULL
            OR uuid_extract_version(reconciliation_claim_token) = 7);

CREATE INDEX IF NOT EXISTS refund_claim_cancellation_reconciliation_lease_idx
    ON refund_claim_cancellation_intents (updated_at, id)
    WHERE signature IS NOT NULL
      AND status IN ('submitting', 'submitted', 'processed', 'confirmed', 'unknown');

COMMIT;
