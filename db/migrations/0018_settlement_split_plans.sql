-- Tenant-scoped immutable Split Plan registry. Solana remains authoritative
-- for account ownership and atomic movement; this table is configuration and
-- quote-compilation evidence only.

BEGIN;

CREATE TABLE IF NOT EXISTS settlement_split_plans (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    merchant_id uuid NOT NULL,
    plan_key text NOT NULL,
    version integer NOT NULL CHECK (version > 0),
    status text NOT NULL CHECK (status IN ('draft', 'pending_activation', 'active', 'suspended', 'retired')),
    merchant_account text NOT NULL CHECK (length(merchant_account) BETWEEN 32 AND 44),
    mint text NOT NULL CHECK (length(mint) BETWEEN 32 AND 44),
    plan_digest bytea NOT NULL CHECK (octet_length(plan_digest) = 32),
    recipients jsonb NOT NULL,
    activation_digest bytea,
    activation_slot numeric(20, 0),
    activation_commitment text,
    activation_evidence jsonb,
    activated_at timestamptz,
    effective_from timestamptz NOT NULL DEFAULT now(),
    effective_until timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK (uuid_extract_version(id) = 7),
    CHECK (merchant_id IS NOT NULL),
    CHECK (plan_key = btrim(plan_key) AND length(plan_key) BETWEEN 1 AND 128),
    CHECK (effective_until IS NULL OR effective_until > effective_from),
    CHECK (activation_digest IS NULL OR octet_length(activation_digest) = 32),
    CHECK (activation_slot IS NULL OR activation_slot >= 0),
    CHECK (activation_commitment IS NULL OR activation_commitment IN ('finalized', 'local_synthetic')),
    CHECK (activation_evidence IS NULL OR pg_column_size(activation_evidence) <= 32768),
    CHECK (
        status <> 'active'
        OR (
            activation_digest IS NOT NULL
            AND activation_slot IS NOT NULL
            AND activation_commitment IS NOT NULL
            AND activation_evidence IS NOT NULL
            AND activated_at IS NOT NULL
        )
    ),
    UNIQUE (merchant_id, plan_key, version),
    UNIQUE (id, merchant_id)
);

CREATE INDEX IF NOT EXISTS settlement_split_plans_active_lookup_idx
    ON settlement_split_plans (merchant_id, status, effective_from, id)
    WHERE status = 'active';

CREATE OR REPLACE FUNCTION validate_settlement_split_plan_document()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    recipient jsonb;
    token_account text;
    basis_points integer;
    total_basis_points integer := 0;
    seen_accounts text[] := ARRAY[]::text[];
BEGIN
    IF jsonb_typeof(NEW.recipients) <> 'array'
        OR jsonb_array_length(NEW.recipients) < 1
        OR jsonb_array_length(NEW.recipients) > 8 THEN
        RAISE EXCEPTION 'split plan recipients must contain one to eight entries';
    END IF;

    FOR recipient IN SELECT value FROM jsonb_array_elements(NEW.recipients)
    LOOP
        IF jsonb_typeof(recipient) <> 'object'
            OR jsonb_typeof(recipient -> 'token_account') <> 'string'
            OR jsonb_typeof(recipient -> 'basis_points') <> 'number' THEN
            RAISE EXCEPTION 'split plan recipient has an invalid shape';
        END IF;
        token_account := recipient ->> 'token_account';
        IF token_account <> btrim(token_account)
            OR length(token_account) NOT BETWEEN 32 AND 44 THEN
            RAISE EXCEPTION 'split plan recipient token account is invalid';
        END IF;
        IF token_account = ANY(seen_accounts) THEN
            RAISE EXCEPTION 'split plan recipient token accounts must be distinct';
        END IF;
        seen_accounts := array_append(seen_accounts, token_account);
        IF (recipient ->> 'basis_points') !~ '^[1-9][0-9]{0,3}$'
            OR (recipient ->> 'basis_points')::integer > 10000 THEN
            RAISE EXCEPTION 'split plan recipient basis points are invalid';
        END IF;
        basis_points := (recipient ->> 'basis_points')::integer;
        total_basis_points := total_basis_points + basis_points;
    END LOOP;

    IF total_basis_points <> 10000 THEN
        RAISE EXCEPTION 'split plan recipient basis points must sum to 10000';
    END IF;
    IF NEW.status = 'active' AND NEW.effective_from > now() THEN
        RAISE EXCEPTION 'active split plan must be effective now';
    END IF;
    NEW.updated_at := now();
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS settlement_split_plan_document_guard ON settlement_split_plans;
CREATE TRIGGER settlement_split_plan_document_guard
    BEFORE INSERT OR UPDATE ON settlement_split_plans
    FOR EACH ROW
    EXECUTE FUNCTION validate_settlement_split_plan_document();

CREATE OR REPLACE FUNCTION prevent_settlement_split_plan_mutation()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF OLD.status IN ('pending_activation', 'active', 'suspended', 'retired') THEN
        IF NEW.merchant_id IS DISTINCT FROM OLD.merchant_id
            OR NEW.plan_key IS DISTINCT FROM OLD.plan_key
            OR NEW.version IS DISTINCT FROM OLD.version
            OR NEW.merchant_account IS DISTINCT FROM OLD.merchant_account
            OR NEW.mint IS DISTINCT FROM OLD.mint
            OR NEW.plan_digest IS DISTINCT FROM OLD.plan_digest
            OR NEW.recipients IS DISTINCT FROM OLD.recipients
            OR NEW.effective_from IS DISTINCT FROM OLD.effective_from
            OR NEW.effective_until IS DISTINCT FROM OLD.effective_until
        THEN
            RAISE EXCEPTION 'published split plan content is immutable; create a new version';
        END IF;
    END IF;
    IF OLD.status IN ('active', 'retired') THEN
        IF NEW.activation_digest IS DISTINCT FROM OLD.activation_digest
            OR NEW.activation_slot IS DISTINCT FROM OLD.activation_slot
            OR NEW.activation_commitment IS DISTINCT FROM OLD.activation_commitment
            OR NEW.activation_evidence IS DISTINCT FROM OLD.activation_evidence
            OR NEW.activated_at IS DISTINCT FROM OLD.activated_at
        THEN
            RAISE EXCEPTION 'active or retired split plan activation evidence is immutable';
        END IF;
    END IF;
    IF OLD.status = 'retired' AND NEW.status <> 'retired' THEN
        RAISE EXCEPTION 'retired split plans cannot be reactivated';
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS settlement_split_plan_mutation_guard ON settlement_split_plans;
CREATE TRIGGER settlement_split_plan_mutation_guard
    BEFORE UPDATE ON settlement_split_plans
    FOR EACH ROW
    EXECUTE FUNCTION prevent_settlement_split_plan_mutation();

COMMIT;
