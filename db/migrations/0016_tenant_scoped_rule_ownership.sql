-- Tenant-owned rule releases. This migration deliberately refuses to guess
-- ownership for pre-existing rows. Production operators must perform an
-- audited merchant_id backfill before applying it to a populated database.

BEGIN;

ALTER TABLE rule_sets
    ADD COLUMN IF NOT EXISTS merchant_id uuid;

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM rule_sets WHERE merchant_id IS NULL) THEN
        RAISE EXCEPTION
            'rule_sets contains unowned rows; perform an audited merchant_id backfill before migration 0016';
    END IF;
END
$$;

ALTER TABLE rule_sets
    ALTER COLUMN merchant_id SET NOT NULL;

ALTER TABLE rule_sets
    DROP CONSTRAINT IF EXISTS rule_sets_external_id_version_key,
    DROP CONSTRAINT IF EXISTS rule_sets_id_merchant_unique,
    DROP CONSTRAINT IF EXISTS rule_sets_merchant_external_version_key,
    ADD CONSTRAINT rule_sets_id_merchant_unique UNIQUE (id, merchant_id),
    ADD CONSTRAINT rule_sets_merchant_owner_check CHECK (merchant_id IS NOT NULL);

ALTER TABLE rule_sets
    ADD CONSTRAINT rule_sets_merchant_external_version_key
        UNIQUE (merchant_id, external_id, version);

ALTER TABLE rule_sets
    DROP CONSTRAINT IF EXISTS published_rule_windows_no_overlap;

ALTER TABLE rule_sets
    ADD CONSTRAINT published_rule_windows_no_overlap
    EXCLUDE USING gist (
        merchant_id WITH =,
        external_id WITH =,
        jurisdiction WITH =,
        regime WITH =,
        tstzrange(effective_from, COALESCE(effective_until, 'infinity'::timestamptz), '[)') WITH &&
    )
    WHERE (status = 'published');

CREATE INDEX IF NOT EXISTS rule_sets_merchant_lookup_idx
    ON rule_sets (merchant_id, external_id, version, jurisdiction, regime,
                  status, effective_from DESC);

CREATE OR REPLACE FUNCTION prevent_rule_owner_mutation()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NEW.merchant_id IS DISTINCT FROM OLD.merchant_id THEN
        RAISE EXCEPTION 'rule ownership is immutable for every lifecycle state';
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS rule_owner_mutation_guard ON rule_sets;
CREATE TRIGGER rule_owner_mutation_guard
    BEFORE UPDATE ON rule_sets
    FOR EACH ROW
    EXECUTE FUNCTION prevent_rule_owner_mutation();

ALTER TABLE tax_calculations
    DROP CONSTRAINT IF EXISTS tax_calculations_rule_owner_fk,
    ADD CONSTRAINT tax_calculations_rule_owner_fk
        FOREIGN KEY (rule_set_id, merchant_id)
        REFERENCES rule_sets (id, merchant_id);

COMMIT;
