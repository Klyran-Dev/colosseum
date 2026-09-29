-- Bound cross-replica ownership for automatic refund-claim finality
-- reconciliation. The lease is operational coordination only; it never
-- authorizes funds and is reclaimable after a bounded crash window.

BEGIN;

ALTER TABLE refund_claim_intents
    ADD COLUMN IF NOT EXISTS reconciliation_claim_token uuid,
    ADD COLUMN IF NOT EXISTS reconciliation_claimed_at timestamptz;

ALTER TABLE refund_claim_intents
    DROP CONSTRAINT IF EXISTS refund_claim_reconciliation_lease_pair,
    ADD CONSTRAINT refund_claim_reconciliation_lease_pair
        CHECK ((reconciliation_claim_token IS NULL)
            = (reconciliation_claimed_at IS NULL));

CREATE INDEX IF NOT EXISTS refund_claim_reconciliation_lease_idx
    ON refund_claim_intents (updated_at, id)
    WHERE signature IS NOT NULL
      AND status IN ('submitting', 'submitted', 'processed', 'confirmed', 'unknown');

COMMIT;
