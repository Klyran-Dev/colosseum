-- Finalized chain evidence for tenant-scoped settlement-route activation.

BEGIN;

ALTER TABLE settlement_routes
    ADD COLUMN IF NOT EXISTS verification_slot numeric(20, 0),
    ADD COLUMN IF NOT EXISTS verification_commitment text,
    ADD COLUMN IF NOT EXISTS verification_evidence jsonb,
    ADD COLUMN IF NOT EXISTS verified_at timestamptz;

ALTER TABLE settlement_routes
    DROP CONSTRAINT IF EXISTS settlement_routes_verification_slot_check,
    DROP CONSTRAINT IF EXISTS settlement_routes_verification_commitment_check,
    DROP CONSTRAINT IF EXISTS settlement_routes_activation_evidence_check;

ALTER TABLE settlement_routes
    ADD CONSTRAINT settlement_routes_verification_slot_check
        CHECK (verification_slot IS NULL OR verification_slot >= 0),
    ADD CONSTRAINT settlement_routes_verification_commitment_check
        CHECK (verification_commitment IS NULL
            OR verification_commitment IN ('finalized', 'local_synthetic')),
    ADD CONSTRAINT settlement_routes_activation_evidence_check
        CHECK (
            status = 'pending_verification'
            OR (
                verification_digest IS NOT NULL
                AND verification_slot IS NOT NULL
                AND verification_commitment IS NOT NULL
                AND verification_evidence IS NOT NULL
                AND verified_at IS NOT NULL
            )
        ),
    ADD CONSTRAINT settlement_routes_verification_evidence_size_check
        CHECK (verification_evidence IS NULL OR pg_column_size(verification_evidence) <= 32768);

ALTER TABLE settlement_routes
    DROP CONSTRAINT IF EXISTS settlement_routes_id_merchant_unique;

ALTER TABLE settlement_routes
    ADD CONSTRAINT settlement_routes_id_merchant_unique UNIQUE (id, merchant_id);

CREATE TABLE IF NOT EXISTS settlement_route_verifications (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    route_id uuid NOT NULL,
    merchant_id uuid NOT NULL,
    route_version integer NOT NULL CHECK (route_version > 0),
    verification_digest bytea NOT NULL CHECK (octet_length(verification_digest) = 32),
    verification_slot numeric(20, 0) NOT NULL CHECK (verification_slot >= 0),
    verification_commitment text NOT NULL
        CHECK (verification_commitment IN ('finalized', 'local_synthetic')),
    evidence jsonb NOT NULL CHECK (pg_column_size(evidence) <= 32768),
    verified_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (route_id, verification_digest),
    FOREIGN KEY (route_id, merchant_id)
        REFERENCES settlement_routes(id, merchant_id)
);

ALTER TABLE settlement_route_verifications
    DROP CONSTRAINT IF EXISTS settlement_route_verifications_id_uuidv7_check,
    ADD CONSTRAINT settlement_route_verifications_id_uuidv7_check
        CHECK (uuid_extract_version(id) = 7);

CREATE INDEX IF NOT EXISTS settlement_route_verifications_route_idx
    ON settlement_route_verifications (route_id, verification_slot DESC, id DESC);

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
            OR NEW.verified_at IS DISTINCT FROM OLD.verified_at
        THEN
            RAISE EXCEPTION 'active or retired route verification evidence is immutable';
        END IF;
    ELSIF OLD.status = 'suspended' AND NEW.status <> 'active' THEN
        IF NEW.verification_digest IS DISTINCT FROM OLD.verification_digest
            OR NEW.verification_slot IS DISTINCT FROM OLD.verification_slot
            OR NEW.verification_commitment IS DISTINCT FROM OLD.verification_commitment
            OR NEW.verification_evidence IS DISTINCT FROM OLD.verification_evidence
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
