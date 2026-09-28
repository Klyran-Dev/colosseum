-- Tenant-scoped, versioned funding topology for governed adjustment refunds.
-- Solana remains authoritative for account ownership, mint, and token-program
-- compatibility. This table is an off-chain governance/configuration boundary.

BEGIN;

CREATE TABLE IF NOT EXISTS refund_funding_configurations (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    merchant_id uuid NOT NULL,
    idempotency_key text NOT NULL,
    request_digest bytea NOT NULL CHECK (octet_length(request_digest) = 32),
    configuration_digest bytea NOT NULL CHECK (octet_length(configuration_digest) = 32),
    configuration_key text NOT NULL,
    version integer NOT NULL CHECK (version > 0),
    status text NOT NULL CHECK (status IN (
        'proposed', 'approved', 'pending_verification', 'active',
        'suspended', 'retired'
    )),
    merchant_account text NOT NULL CHECK (length(merchant_account) BETWEEN 32 AND 44),
    mint text NOT NULL CHECK (length(mint) BETWEEN 32 AND 44),
    tax_vault text NOT NULL CHECK (length(tax_vault) BETWEEN 32 AND 44),
    vault_authority text NOT NULL CHECK (length(vault_authority) BETWEEN 32 AND 44),
    refund_funder text NOT NULL CHECK (length(refund_funder) BETWEEN 32 AND 44),
    net_source text NOT NULL CHECK (length(net_source) BETWEEN 32 AND 44),
    token_program text NOT NULL CHECK (length(token_program) BETWEEN 32 AND 44),
    proposer_subject text NOT NULL CHECK (length(proposer_subject) BETWEEN 1 AND 256),
    approver_subject text,
    verification_digest bytea CHECK (verification_digest IS NULL OR octet_length(verification_digest) = 32),
    effective_from timestamptz NOT NULL DEFAULT now(),
    effective_until timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK (uuid_extract_version(id) = 7),
    CHECK (uuid_extract_version(merchant_id) = 7),
    CHECK (length(idempotency_key) BETWEEN 1 AND 200 AND idempotency_key = btrim(idempotency_key)),
    CHECK (length(configuration_key) BETWEEN 1 AND 128 AND configuration_key = btrim(configuration_key)),
    CHECK (effective_until IS NULL OR effective_until > effective_from),
    CHECK (tax_vault <> net_source AND tax_vault <> refund_funder
           AND tax_vault <> mint AND net_source <> refund_funder
           AND net_source <> mint AND refund_funder <> mint),
    UNIQUE (merchant_id, idempotency_key),
    UNIQUE (merchant_id, configuration_key, version)
);

CREATE UNIQUE INDEX IF NOT EXISTS refund_funding_configurations_active_idx
    ON refund_funding_configurations (merchant_id, configuration_key)
    WHERE status = 'active';

CREATE INDEX IF NOT EXISTS refund_funding_configurations_lookup_idx
    ON refund_funding_configurations (merchant_id, configuration_key, version DESC, id DESC);

CREATE OR REPLACE FUNCTION prevent_refund_funding_configuration_mutation()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF OLD.status <> NEW.status
        AND NOT (
            (OLD.status = 'proposed' AND NEW.status = 'approved')
            OR (OLD.status = 'approved' AND NEW.status = 'pending_verification')
            OR (OLD.status = 'pending_verification' AND NEW.status = 'active')
            OR (OLD.status = 'active' AND NEW.status IN ('suspended', 'retired'))
            OR (OLD.status = 'suspended' AND NEW.status = 'retired')
        )
    THEN
        RAISE EXCEPTION 'invalid refund funding configuration status transition';
    END IF;

    IF NEW.status = 'approved' AND NEW.approver_subject IS NULL THEN
        RAISE EXCEPTION 'approved refund funding configurations require an approver';
    END IF;

    IF NEW.status = 'active' AND NEW.verification_digest IS NULL THEN
        RAISE EXCEPTION 'active refund funding configurations require finalized verification';
    END IF;

    IF OLD.status IN ('approved', 'pending_verification', 'active', 'suspended', 'retired') THEN
        IF NEW.merchant_id IS DISTINCT FROM OLD.merchant_id
            OR NEW.idempotency_key IS DISTINCT FROM OLD.idempotency_key
            OR NEW.request_digest IS DISTINCT FROM OLD.request_digest
            OR NEW.configuration_digest IS DISTINCT FROM OLD.configuration_digest
            OR NEW.configuration_key IS DISTINCT FROM OLD.configuration_key
            OR NEW.version IS DISTINCT FROM OLD.version
            OR NEW.merchant_account IS DISTINCT FROM OLD.merchant_account
            OR NEW.mint IS DISTINCT FROM OLD.mint
            OR NEW.tax_vault IS DISTINCT FROM OLD.tax_vault
            OR NEW.vault_authority IS DISTINCT FROM OLD.vault_authority
            OR NEW.refund_funder IS DISTINCT FROM OLD.refund_funder
            OR NEW.net_source IS DISTINCT FROM OLD.net_source
            OR NEW.token_program IS DISTINCT FROM OLD.token_program
            OR NEW.proposer_subject IS DISTINCT FROM OLD.proposer_subject
            OR (OLD.status <> 'pending_verification'
                AND NEW.verification_digest IS DISTINCT FROM OLD.verification_digest)
            OR NEW.effective_from IS DISTINCT FROM OLD.effective_from
            OR NEW.effective_until IS DISTINCT FROM OLD.effective_until
        THEN
            RAISE EXCEPTION 'refund funding account fields are immutable; create a new version';
        END IF;
    END IF;

    IF OLD.status = 'retired' AND NEW.status <> 'retired' THEN
        RAISE EXCEPTION 'retired refund funding configurations cannot be reactivated';
    END IF;

    NEW.updated_at = now();
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS refund_funding_configuration_mutation_guard
    ON refund_funding_configurations;
CREATE TRIGGER refund_funding_configuration_mutation_guard
    BEFORE UPDATE ON refund_funding_configurations
    FOR EACH ROW
    EXECUTE FUNCTION prevent_refund_funding_configuration_mutation();

CREATE TABLE IF NOT EXISTS refund_funding_configuration_events (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    configuration_id uuid NOT NULL,
    merchant_id uuid NOT NULL,
    actor_subject text NOT NULL CHECK (length(actor_subject) BETWEEN 1 AND 256),
    action text NOT NULL CHECK (length(action) BETWEEN 1 AND 64),
    from_status text NOT NULL,
    to_status text NOT NULL,
    evidence jsonb NOT NULL,
    occurred_at timestamptz NOT NULL DEFAULT now(),
    CHECK (uuid_extract_version(id) = 7),
    FOREIGN KEY (configuration_id) REFERENCES refund_funding_configurations(id),
    UNIQUE (configuration_id, id)
);

CREATE INDEX IF NOT EXISTS refund_funding_configuration_events_order_idx
    ON refund_funding_configuration_events (configuration_id, occurred_at, id);

COMMIT;
