-- Expand migration for the on-chain SettlementAdjustmentRecorded evidence
-- digest. Existing projection rows predate this typed field and remain
-- explicitly incomplete until rebuilt from raw finalized event bytes.

BEGIN;

ALTER TABLE indexed_settlement_adjustments
    ADD COLUMN IF NOT EXISTS adjustment_evidence_digest bytea;

ALTER TABLE indexed_settlement_adjustments
    DROP CONSTRAINT IF EXISTS indexed_settlement_adjustments_evidence_digest_shape;

ALTER TABLE indexed_settlement_adjustments
    ADD CONSTRAINT indexed_settlement_adjustments_evidence_digest_shape
    CHECK (
        adjustment_evidence_digest IS NULL
        OR octet_length(adjustment_evidence_digest) = 32
    );

COMMIT;
