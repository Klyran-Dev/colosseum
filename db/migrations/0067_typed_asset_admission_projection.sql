-- Expand/contract migration for the typed, tenant/version-bound projection of
-- the finalized Solana AssetConfig account. Raw verification evidence remains
-- immutable and authoritative for replay; these columns are an operational
-- projection with direct database invariants.

BEGIN;

ALTER TABLE settlement_routes
    ADD COLUMN IF NOT EXISTS asset_config text,
    ADD COLUMN IF NOT EXISTS asset_kind smallint,
    ADD COLUMN IF NOT EXISTS asset_enabled boolean,
    ADD COLUMN IF NOT EXISTS asset_config_bump smallint;

-- Safe, non-inferential expansion: only the already explicit PDA identity is
-- copied. Classification, enabled state, and bump must come from a new
-- finalized verifier result and are never manufactured from account digests.
UPDATE settlement_routes
SET asset_config = verification_evidence ->> 'asset_config'
WHERE asset_config IS NULL
  AND verification_evidence IS NOT NULL
  AND jsonb_typeof(verification_evidence -> 'asset_config') = 'string'
  AND length(verification_evidence ->> 'asset_config') BETWEEN 32 AND 44;

-- Existing active rows without the complete typed proof are not signable.
-- Preserve their raw evidence and suspend them for explicit re-verification.
UPDATE settlement_routes
SET status = 'suspended', updated_at = now()
WHERE status = 'active'
  AND (
      asset_config IS NULL
      OR asset_kind IS NULL
      OR asset_enabled IS NULL
      OR asset_config_bump IS NULL
  );

ALTER TABLE settlement_routes
    DROP CONSTRAINT IF EXISTS settlement_routes_asset_config_shape_check,
    DROP CONSTRAINT IF EXISTS settlement_routes_asset_kind_check,
    DROP CONSTRAINT IF EXISTS settlement_routes_asset_enabled_check,
    DROP CONSTRAINT IF EXISTS settlement_routes_asset_config_bump_check,
    DROP CONSTRAINT IF EXISTS settlement_routes_active_asset_projection_check,
    DROP CONSTRAINT IF EXISTS settlement_routes_asset_projection_evidence_check;

ALTER TABLE settlement_routes
    ADD CONSTRAINT settlement_routes_asset_config_shape_check
        CHECK (asset_config IS NULL OR length(asset_config) BETWEEN 32 AND 44),
    ADD CONSTRAINT settlement_routes_asset_kind_check
        CHECK (asset_kind IS NULL OR asset_kind BETWEEN 0 AND 255),
    ADD CONSTRAINT settlement_routes_asset_enabled_check
        CHECK (asset_enabled IS NULL OR asset_enabled IN (true, false)),
    ADD CONSTRAINT settlement_routes_asset_config_bump_check
        CHECK (asset_config_bump IS NULL OR asset_config_bump BETWEEN 0 AND 255),
    ADD CONSTRAINT settlement_routes_active_asset_projection_check
        CHECK (status <> 'active'
            OR (
                asset_config IS NOT NULL
                AND asset_kind = 1
                AND asset_enabled = true
                AND asset_config_bump IS NOT NULL
            )),
    ADD CONSTRAINT settlement_routes_asset_projection_evidence_check
        CHECK (
            status <> 'active'
            OR (
                verification_evidence IS NOT NULL
                AND verification_evidence ->> 'asset_config' = asset_config
                AND jsonb_typeof(verification_evidence -> 'asset_kind') = 'number'
                AND (verification_evidence ->> 'asset_kind') ~ '^[0-9]{1,3}$'
                AND (verification_evidence ->> 'asset_kind')::smallint = asset_kind
                AND jsonb_typeof(verification_evidence -> 'asset_enabled') = 'boolean'
                AND (verification_evidence ->> 'asset_enabled')::boolean = asset_enabled
                AND jsonb_typeof(verification_evidence -> 'asset_config_bump') = 'number'
                AND (verification_evidence ->> 'asset_config_bump') ~ '^[0-9]{1,3}$'
                AND (verification_evidence ->> 'asset_config_bump')::smallint = asset_config_bump
            )
        );

CREATE OR REPLACE FUNCTION prevent_settlement_route_mutation()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF OLD.status IN ('active', 'suspended', 'retired') THEN
        IF NEW.merchant_id IS DISTINCT FROM OLD.merchant_id
            OR NEW.route_key IS DISTINCT FROM OLD.route_key
            OR NEW.version IS DISTINCT FROM OLD.version
            OR NEW.merchant_account IS DISTINCT FROM OLD.merchant_account
            OR NEW.mint IS DISTINCT FROM OLD.mint
            OR NEW.tax_vault IS DISTINCT FROM OLD.tax_vault
            OR NEW.net_destination IS DISTINCT FROM OLD.net_destination
            OR NEW.token_program IS DISTINCT FROM OLD.token_program
            OR NEW.effective_from IS DISTINCT FROM OLD.effective_from
            OR NEW.effective_until IS DISTINCT FROM OLD.effective_until
        THEN
            RAISE EXCEPTION 'settlement route account fields are immutable; create a new version';
        END IF;
    END IF;

    IF OLD.status IN ('active', 'retired') THEN
        IF NEW.verification_digest IS DISTINCT FROM OLD.verification_digest
            OR NEW.verification_slot IS DISTINCT FROM OLD.verification_slot
            OR NEW.verification_commitment IS DISTINCT FROM OLD.verification_commitment
            OR NEW.verification_evidence IS DISTINCT FROM OLD.verification_evidence
            OR NEW.verification_schema IS DISTINCT FROM OLD.verification_schema
            OR NEW.verified_at IS DISTINCT FROM OLD.verified_at
            OR NEW.asset_config IS DISTINCT FROM OLD.asset_config
            OR NEW.asset_kind IS DISTINCT FROM OLD.asset_kind
            OR NEW.asset_enabled IS DISTINCT FROM OLD.asset_enabled
            OR NEW.asset_config_bump IS DISTINCT FROM OLD.asset_config_bump
        THEN
            RAISE EXCEPTION 'active or retired route verification evidence is immutable';
        END IF;
    ELSIF OLD.status = 'suspended' AND NEW.status <> 'active' THEN
        IF NEW.verification_digest IS DISTINCT FROM OLD.verification_digest
            OR NEW.verification_slot IS DISTINCT FROM OLD.verification_slot
            OR NEW.verification_commitment IS DISTINCT FROM OLD.verification_commitment
            OR NEW.verification_evidence IS DISTINCT FROM OLD.verification_evidence
            OR NEW.verification_schema IS DISTINCT FROM OLD.verification_schema
            OR NEW.verified_at IS DISTINCT FROM OLD.verified_at
            OR NEW.asset_config IS DISTINCT FROM OLD.asset_config
            OR NEW.asset_kind IS DISTINCT FROM OLD.asset_kind
            OR NEW.asset_enabled IS DISTINCT FROM OLD.asset_enabled
            OR NEW.asset_config_bump IS DISTINCT FROM OLD.asset_config_bump
        THEN
            RAISE EXCEPTION 'suspended route evidence may change only during reactivation';
        END IF;
    END IF;

    IF OLD.status = 'retired' AND NEW.status <> 'retired' THEN
        RAISE EXCEPTION 'retired settlement routes cannot be reactivated';
    END IF;

    IF NEW.status = 'active' AND OLD.status NOT IN ('pending_verification', 'suspended', 'active') THEN
        RAISE EXCEPTION 'only pending or suspended routes may become active';
    END IF;

    NEW.updated_at = now();
    RETURN NEW;
END;
$$;

COMMIT;
