-- Tenant-scoped, versioned public-account configuration for settlement.
-- Account ownership and token compatibility remain Solana authorities; this
-- table is an off-chain configuration and lifecycle boundary only.

BEGIN;

CREATE TABLE IF NOT EXISTS settlement_routes (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    merchant_id uuid NOT NULL,
    route_key text NOT NULL,
    version integer NOT NULL CHECK (version > 0),
    status text NOT NULL CHECK (status IN ('pending_verification', 'active', 'suspended', 'retired')),
    merchant_account text NOT NULL CHECK (length(merchant_account) BETWEEN 32 AND 44),
    mint text NOT NULL CHECK (length(mint) BETWEEN 32 AND 44),
    tax_vault text NOT NULL CHECK (length(tax_vault) BETWEEN 32 AND 44),
    net_destination text NOT NULL CHECK (length(net_destination) BETWEEN 32 AND 44),
    token_program text NOT NULL CHECK (length(token_program) BETWEEN 32 AND 44),
    verification_digest bytea,
    effective_from timestamptz NOT NULL DEFAULT now(),
    effective_until timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK (uuid_extract_version(id) = 7),
    CHECK (length(route_key) BETWEEN 1 AND 128),
    CHECK (route_key = btrim(route_key)),
    CHECK (effective_until IS NULL OR effective_until > effective_from),
    CHECK (verification_digest IS NULL OR octet_length(verification_digest) = 32),
    CHECK (status = 'pending_verification' OR verification_digest IS NOT NULL),
    UNIQUE (merchant_id, route_key, version)
);

CREATE INDEX IF NOT EXISTS settlement_routes_active_lookup_idx
    ON settlement_routes (merchant_id, status, effective_from, id)
    WHERE status = 'active';

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
            OR NEW.verification_digest IS DISTINCT FROM OLD.verification_digest
            OR NEW.effective_from IS DISTINCT FROM OLD.effective_from
            OR NEW.effective_until IS DISTINCT FROM OLD.effective_until
        THEN
            RAISE EXCEPTION 'active settlement route account fields are immutable; create a new version';
        END IF;
    END IF;

    IF OLD.status = 'retired' AND NEW.status <> 'retired' THEN
        RAISE EXCEPTION 'retired settlement routes cannot be reactivated';
    END IF;

    NEW.updated_at = now();
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS settlement_route_mutation_guard ON settlement_routes;
CREATE TRIGGER settlement_route_mutation_guard
    BEFORE UPDATE ON settlement_routes
    FOR EACH ROW
    EXECUTE FUNCTION prevent_settlement_route_mutation();

COMMIT;
