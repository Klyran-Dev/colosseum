-- Immutable target registry and append-only finalized vault reconciliation
-- evidence. Solana token accounts remain the authority for funds.

BEGIN;

CREATE TABLE IF NOT EXISTS vault_reconciliation_targets (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    merchant_id uuid NOT NULL,
    route_id uuid NOT NULL,
    stream_key text NOT NULL,
    mint text NOT NULL,
    token_program text NOT NULL,
    tax_vault text NOT NULL,
    opening_balance_atomic numeric(20, 0) NOT NULL,
    opening_balance_slot numeric(20, 0) NOT NULL,
    opening_reference text NOT NULL,
    status text NOT NULL DEFAULT 'active',
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK (uuid_extract_version(id) = 7),
    CHECK (length(btrim(stream_key)) BETWEEN 1 AND 200),
    CHECK (length(btrim(mint)) BETWEEN 32 AND 44),
    CHECK (length(btrim(token_program)) BETWEEN 32 AND 44),
    CHECK (length(btrim(tax_vault)) BETWEEN 32 AND 44),
    CHECK (opening_balance_atomic >= 0 AND opening_balance_atomic <= 18446744073709551615),
    CHECK (opening_balance_slot >= 0 AND opening_balance_slot <= 18446744073709551615),
    CHECK (length(btrim(opening_reference)) BETWEEN 1 AND 256),
    CHECK (status IN ('active', 'suspended', 'retired')),
    UNIQUE (merchant_id, route_id),
    FOREIGN KEY (route_id, merchant_id) REFERENCES settlement_routes(id, merchant_id)
);

CREATE INDEX IF NOT EXISTS vault_reconciliation_targets_active_idx
    ON vault_reconciliation_targets (status, updated_at, id)
    WHERE status = 'active';

CREATE OR REPLACE FUNCTION prevent_vault_reconciliation_target_mutation()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF OLD.status IN ('active', 'suspended', 'retired') AND (
        NEW.merchant_id IS DISTINCT FROM OLD.merchant_id
        OR NEW.route_id IS DISTINCT FROM OLD.route_id
        OR NEW.stream_key IS DISTINCT FROM OLD.stream_key
        OR NEW.mint IS DISTINCT FROM OLD.mint
        OR NEW.token_program IS DISTINCT FROM OLD.token_program
        OR NEW.tax_vault IS DISTINCT FROM OLD.tax_vault
        OR NEW.opening_balance_atomic IS DISTINCT FROM OLD.opening_balance_atomic
        OR NEW.opening_balance_slot IS DISTINCT FROM OLD.opening_balance_slot
        OR NEW.opening_reference IS DISTINCT FROM OLD.opening_reference
    ) THEN
        RAISE EXCEPTION 'vault reconciliation target identity and opening evidence are immutable';
    END IF;
    IF OLD.status = 'retired' AND NEW.status <> 'retired' THEN
        RAISE EXCEPTION 'retired vault reconciliation targets cannot be reactivated';
    END IF;
    NEW.updated_at = now();
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS vault_reconciliation_target_mutation_guard
    ON vault_reconciliation_targets;
CREATE TRIGGER vault_reconciliation_target_mutation_guard
    BEFORE UPDATE ON vault_reconciliation_targets
    FOR EACH ROW
    EXECUTE FUNCTION prevent_vault_reconciliation_target_mutation();

CREATE TABLE IF NOT EXISTS vault_reconciliation_runs (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    target_id uuid NOT NULL REFERENCES vault_reconciliation_targets(id),
    route_id uuid NOT NULL,
    merchant_id uuid NOT NULL,
    stream_key text NOT NULL,
    mint text NOT NULL,
    token_program text NOT NULL,
    tax_vault text NOT NULL,
    opening_balance_atomic numeric(20, 0) NOT NULL,
    receipt_tax_atomic numeric(20, 0) NOT NULL,
    release_atomic numeric(20, 0) NOT NULL,
    expected_balance_atomic numeric(20, 0),
    observed_balance_atomic numeric(20, 0),
    observed_slot numeric(20, 0),
    status text NOT NULL,
    error_code text,
    created_at timestamptz NOT NULL DEFAULT now(),
    CHECK (uuid_extract_version(id) = 7),
    CHECK (length(btrim(stream_key)) BETWEEN 1 AND 200),
    CHECK (length(btrim(mint)) BETWEEN 32 AND 44),
    CHECK (length(btrim(token_program)) BETWEEN 32 AND 44),
    CHECK (length(btrim(tax_vault)) BETWEEN 32 AND 44),
    CHECK (opening_balance_atomic >= 0 AND opening_balance_atomic <= 18446744073709551615),
    CHECK (receipt_tax_atomic >= 0 AND receipt_tax_atomic <= 18446744073709551615),
    CHECK (release_atomic >= 0 AND release_atomic <= 18446744073709551615),
    CHECK (expected_balance_atomic IS NULL OR (expected_balance_atomic >= 0 AND expected_balance_atomic <= 18446744073709551615)),
    CHECK (observed_balance_atomic IS NULL OR (observed_balance_atomic >= 0 AND observed_balance_atomic <= 18446744073709551615)),
    CHECK (observed_slot IS NULL OR (observed_slot >= 0 AND observed_slot <= 18446744073709551615)),
    CHECK (status IN ('matched', 'mismatch', 'error')),
    CHECK (error_code IS NULL OR length(btrim(error_code)) BETWEEN 1 AND 128),
    CHECK ((status = 'error' AND error_code IS NOT NULL) OR (status <> 'error' AND error_code IS NULL)),
    CHECK ((status = 'matched' AND expected_balance_atomic IS NOT NULL AND observed_balance_atomic IS NOT NULL AND observed_slot IS NOT NULL) OR status <> 'matched')
);

CREATE INDEX IF NOT EXISTS vault_reconciliation_runs_target_idx
    ON vault_reconciliation_runs (target_id, created_at DESC, id DESC);

CREATE INDEX IF NOT EXISTS vault_reconciliation_runs_status_idx
    ON vault_reconciliation_runs (status, created_at DESC, id DESC)
    WHERE status <> 'matched';

CREATE OR REPLACE FUNCTION prevent_vault_reconciliation_run_mutation()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    RAISE EXCEPTION 'vault reconciliation runs are append-only';
END;
$$;

DROP TRIGGER IF EXISTS vault_reconciliation_run_mutation_guard
    ON vault_reconciliation_runs;
CREATE TRIGGER vault_reconciliation_run_mutation_guard
    BEFORE UPDATE OR DELETE ON vault_reconciliation_runs
    FOR EACH ROW
    EXECUTE FUNCTION prevent_vault_reconciliation_run_mutation();

COMMIT;
