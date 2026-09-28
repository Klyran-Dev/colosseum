-- Bind every persisted calculation to its authenticated merchant and give it
-- a finite settlement-authorization lifetime. Legacy orphan rows deliberately
-- stop the migration instead of receiving guessed tenant identity.

BEGIN;

ALTER TABLE tax_calculations
    ADD COLUMN IF NOT EXISTS merchant_id uuid;

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM tax_calculations WHERE merchant_id IS NULL) THEN
        RAISE EXCEPTION 'tax_calculations contains orphan rows; backfill merchant_id before enabling tenant-scoped settlement';
    END IF;
END
$$;

ALTER TABLE tax_calculations
    ALTER COLUMN merchant_id SET NOT NULL,
    ALTER COLUMN expires_at SET NOT NULL;

ALTER TABLE tax_calculations
    DROP CONSTRAINT IF EXISTS tax_calculations_expiry_check;

ALTER TABLE tax_calculations
    ADD CONSTRAINT tax_calculations_expiry_check
    CHECK (expires_at > created_at);

CREATE INDEX IF NOT EXISTS tax_calculations_merchant_expiry_idx
    ON tax_calculations (merchant_id, expires_at, id);

COMMIT;
