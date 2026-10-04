-- Expand/contract migration for the five-account stablecoin route proof.
-- Legacy evidence is retained, but active routes without v2 evidence are
-- suspended before the new activation contract becomes visible.

BEGIN;

ALTER TABLE settlement_routes
    ADD COLUMN IF NOT EXISTS verification_schema text;

ALTER TABLE settlement_route_verifications
    ADD COLUMN IF NOT EXISTS verification_schema text NOT NULL DEFAULT 'legacy';

UPDATE settlement_routes
SET status = 'suspended', updated_at = now()
WHERE status = 'active'
  AND verification_schema IS DISTINCT FROM 'settlement-route-verification/v2';

ALTER TABLE settlement_routes
    DROP CONSTRAINT IF EXISTS settlement_routes_verification_schema_check,
    DROP CONSTRAINT IF EXISTS settlement_routes_active_v2_evidence_check;

ALTER TABLE settlement_routes
    ADD CONSTRAINT settlement_routes_verification_schema_check
        CHECK (verification_schema IS NULL
            OR verification_schema IN ('legacy', 'settlement-route-verification/v2')),
    ADD CONSTRAINT settlement_routes_active_v2_evidence_check
        CHECK (status <> 'active'
            OR verification_schema = 'settlement-route-verification/v2');

ALTER TABLE settlement_route_verifications
    DROP CONSTRAINT IF EXISTS settlement_route_verifications_schema_check;

ALTER TABLE settlement_route_verifications
    ADD CONSTRAINT settlement_route_verifications_schema_check
        CHECK (verification_schema IN ('legacy', 'settlement-route-verification/v2'));

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
