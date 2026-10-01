-- Persist the finalized verifier's mint precision on the immutable route
-- version so checkout never guesses an asset's decimal scale.

BEGIN;

ALTER TABLE settlement_routes
    ADD COLUMN IF NOT EXISTS mint_decimals smallint;

-- Existing verifier evidence already carries the precision. Backfill only
-- canonical bounded decimal text; malformed active rows must fail the
-- activation constraint below rather than receive a guessed default.
UPDATE settlement_routes
SET mint_decimals = (verification_evidence ->> 'mint_decimals')::smallint
WHERE mint_decimals IS NULL
  AND verification_evidence IS NOT NULL
  AND jsonb_typeof(verification_evidence -> 'mint_decimals') = 'number'
  AND (verification_evidence ->> 'mint_decimals') ~ '^[0-9]{1,2}$';

ALTER TABLE settlement_routes
    DROP CONSTRAINT IF EXISTS settlement_routes_mint_decimals_check,
    DROP CONSTRAINT IF EXISTS settlement_routes_active_mint_decimals_check;

ALTER TABLE settlement_routes
    ADD CONSTRAINT settlement_routes_mint_decimals_check
        CHECK (mint_decimals IS NULL OR mint_decimals BETWEEN 0 AND 9),
    ADD CONSTRAINT settlement_routes_active_mint_decimals_check
        CHECK (status = 'pending_verification' OR mint_decimals IS NOT NULL),
    ADD CONSTRAINT settlement_routes_mint_decimals_evidence_check
        CHECK (
            status = 'pending_verification'
            OR (
                mint_decimals IS NOT NULL
                AND verification_evidence IS NOT NULL
                AND jsonb_typeof(verification_evidence -> 'mint_decimals') = 'number'
                AND (verification_evidence ->> 'mint_decimals') ~ '^[0-9]$'
                AND (verification_evidence ->> 'mint_decimals')::smallint = mint_decimals
            )
        );

CREATE OR REPLACE FUNCTION prevent_settlement_route_precision_mutation()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF OLD.status IN ('active', 'suspended', 'retired')
       AND NEW.mint_decimals IS DISTINCT FROM OLD.mint_decimals THEN
        RAISE EXCEPTION 'verified settlement route mint precision is immutable; create a new version';
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS settlement_route_precision_guard ON settlement_routes;
CREATE TRIGGER settlement_route_precision_guard
    BEFORE UPDATE ON settlement_routes
    FOR EACH ROW
    EXECUTE FUNCTION prevent_settlement_route_precision_mutation();

COMMIT;
