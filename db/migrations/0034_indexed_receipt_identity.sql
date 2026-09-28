-- Add the server-owned chain identity required to bind adjustments to the
-- original finalized SettlementExecuted event. This is an expand migration;
-- legacy rows are intentionally nullable until the raw-evidence rebuild has
-- backfilled them and a later release validates the completeness constraint.

BEGIN;

ALTER TABLE indexed_receipts
    ADD COLUMN IF NOT EXISTS receipt bytea,
    ADD COLUMN IF NOT EXISTS payer bytea,
    ADD COLUMN IF NOT EXISTS merchant bytea,
    ADD COLUMN IF NOT EXISTS mint bytea,
    ADD COLUMN IF NOT EXISTS nonce numeric(20, 0),
    ADD COLUMN IF NOT EXISTS policy_digest bytea;

ALTER TABLE indexed_receipts
    DROP CONSTRAINT IF EXISTS indexed_receipts_identity_shape_check;

ALTER TABLE indexed_receipts
    ADD CONSTRAINT indexed_receipts_identity_shape_check
    CHECK (
        (receipt IS NULL OR octet_length(receipt) = 32)
        AND (payer IS NULL OR octet_length(payer) = 32)
        AND (merchant IS NULL OR octet_length(merchant) = 32)
        AND (mint IS NULL OR octet_length(mint) = 32)
        AND (nonce IS NULL OR (nonce > 0 AND nonce <= 18446744073709551615))
        AND (policy_digest IS NULL OR octet_length(policy_digest) = 32)
    ) NOT VALID;

ALTER TABLE indexed_receipts
    DROP CONSTRAINT IF EXISTS indexed_receipts_identity_complete_check;

ALTER TABLE indexed_receipts
    ADD CONSTRAINT indexed_receipts_identity_complete_check
    CHECK (
        receipt IS NOT NULL
        AND payer IS NOT NULL
        AND merchant IS NOT NULL
        AND mint IS NOT NULL
        AND nonce IS NOT NULL
        AND policy_digest IS NOT NULL
    ) NOT VALID;

CREATE INDEX IF NOT EXISTS indexed_receipts_identity_idx
    ON indexed_receipts (stream_key, merchant, mint, slot, signature);

COMMIT;
