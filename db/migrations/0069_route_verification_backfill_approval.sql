-- Approval binding for deliberate one-shot route-verification backfills.

BEGIN;

ALTER TABLE settlement_route_verification_runs
    ADD COLUMN IF NOT EXISTS operator_approval_id uuid;

ALTER TABLE settlement_route_verification_runs
    ADD CONSTRAINT settlement_route_verification_runs_id_uuidv7_strict_check
    CHECK (COALESCE(uuid_extract_version(id), 0) = 7);

ALTER TABLE settlement_route_verification_runs
    DROP CONSTRAINT IF EXISTS settlement_route_verification_runs_approval_binding_check;
ALTER TABLE settlement_route_verification_runs
    ADD CONSTRAINT settlement_route_verification_runs_approval_binding_check
    CHECK (
        (run_kind = 'scheduled' AND operator_approval_id IS NULL)
        OR (run_kind = 'backfill' AND operator_approval_id IS NOT NULL)
    );

ALTER TABLE settlement_route_verification_runs
    DROP CONSTRAINT IF EXISTS settlement_route_verification_runs_approval_uuidv7_check;
ALTER TABLE settlement_route_verification_runs
    ADD CONSTRAINT settlement_route_verification_runs_approval_uuidv7_check
    CHECK (
        operator_approval_id IS NULL
        OR COALESCE(uuid_extract_version(operator_approval_id), 0) = 7
    );

CREATE INDEX IF NOT EXISTS settlement_route_verification_runs_approval_idx
    ON settlement_route_verification_runs (operator_approval_id)
    WHERE operator_approval_id IS NOT NULL;

CREATE OR REPLACE FUNCTION prevent_settlement_route_verification_run_mutation()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF OLD.status <> 'running' THEN
        RAISE EXCEPTION 'terminal settlement route verification runs are immutable';
    END IF;
    IF NEW.id IS DISTINCT FROM OLD.id
        OR NEW.run_kind IS DISTINCT FROM OLD.run_kind
        OR NEW.batch_size IS DISTINCT FROM OLD.batch_size
        OR NEW.operator_approval_id IS DISTINCT FROM OLD.operator_approval_id
        OR NEW.started_at IS DISTINCT FROM OLD.started_at
    THEN
        RAISE EXCEPTION 'settlement route verification run identity is immutable';
    END IF;
    NEW.updated_at = now();
    RETURN NEW;
END;
$$;

COMMIT;
