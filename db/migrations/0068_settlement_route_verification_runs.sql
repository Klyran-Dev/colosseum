-- Durable, bounded accounting for finalized settlement-route verification
-- passes. The ledger is operational evidence only; route activation remains
-- the authority-bound transaction in settlement_routes.

BEGIN;

CREATE TABLE IF NOT EXISTS settlement_route_verification_runs (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    run_kind text NOT NULL
        CHECK (run_kind IN ('scheduled', 'backfill')),
    status text NOT NULL
        CHECK (status IN ('running', 'completed', 'completed_with_failures', 'failed')),
    batch_size smallint NOT NULL CHECK (batch_size BETWEEN 1 AND 64),
    examined_count integer NOT NULL DEFAULT 0 CHECK (examined_count BETWEEN 0 AND 64),
    activated_count integer NOT NULL DEFAULT 0 CHECK (activated_count BETWEEN 0 AND 64),
    already_active_count integer NOT NULL DEFAULT 0 CHECK (already_active_count BETWEEN 0 AND 64),
    failed_count integer NOT NULL DEFAULT 0 CHECK (failed_count BETWEEN 0 AND 64),
    failure_code text,
    started_at timestamptz NOT NULL DEFAULT now(),
    completed_at timestamptz,
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK (uuid_extract_version(id) = 7),
    CHECK (completed_at IS NULL OR completed_at >= started_at),
    CHECK (
        status = 'running'
        OR completed_at IS NOT NULL
    ),
    CHECK (
        status = 'running'
        OR examined_count = activated_count + already_active_count + failed_count
    ),
    CHECK (
        (status = 'completed' AND failed_count = 0 AND failure_code IS NULL)
        OR (status = 'completed_with_failures' AND failed_count > 0 AND failure_code = 'route_attempts_failed')
        OR (status = 'failed' AND failure_code IS NOT NULL)
        OR status = 'running'
    ),
    CHECK (failure_code IS NULL OR length(failure_code) BETWEEN 1 AND 128)
);

CREATE INDEX IF NOT EXISTS settlement_route_verification_runs_started_idx
    ON settlement_route_verification_runs (started_at DESC, id DESC);

CREATE INDEX IF NOT EXISTS settlement_route_verification_runs_status_idx
    ON settlement_route_verification_runs (status, started_at DESC, id DESC);

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
        OR NEW.started_at IS DISTINCT FROM OLD.started_at
    THEN
        RAISE EXCEPTION 'settlement route verification run identity is immutable';
    END IF;
    NEW.updated_at = now();
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS settlement_route_verification_run_guard
    ON settlement_route_verification_runs;
CREATE TRIGGER settlement_route_verification_run_guard
    BEFORE UPDATE ON settlement_route_verification_runs
    FOR EACH ROW
    EXECUTE FUNCTION prevent_settlement_route_verification_run_mutation();

COMMIT;
