-- Cross-replica lease fencing for finalized refund-funding topology
-- verification. The lease is coordination state only; it cannot authorize
-- token movement or wallet signing.

BEGIN;

ALTER TABLE refund_funding_configurations
    ADD COLUMN IF NOT EXISTS verification_lease_token uuid,
    ADD COLUMN IF NOT EXISTS verification_lease_claimed_at timestamptz;

ALTER TABLE refund_funding_configurations
    DROP CONSTRAINT IF EXISTS refund_funding_verification_lease_pair;

ALTER TABLE refund_funding_configurations
    ADD CONSTRAINT refund_funding_verification_lease_pair CHECK (
        (verification_lease_token IS NULL) = (verification_lease_claimed_at IS NULL)
        AND (status = 'pending_verification' OR verification_lease_token IS NULL)
    );

CREATE INDEX IF NOT EXISTS refund_funding_verification_lease_idx
    ON refund_funding_configurations (status, updated_at, id)
    WHERE status IN ('approved', 'pending_verification');

COMMIT;
