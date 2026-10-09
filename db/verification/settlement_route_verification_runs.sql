DO $$
DECLARE
    run_id uuid;
    running_id uuid;
    approval_id uuid := uuidv7();
BEGIN
    INSERT INTO settlement_route_verification_runs
        (run_kind, status, batch_size, operator_approval_id)
    VALUES ('backfill', 'running', 2, approval_id)
    RETURNING id INTO run_id;

    IF uuid_extract_version(run_id) <> 7 THEN
        RAISE EXCEPTION 'verification run identity is not UUIDv7';
    END IF;

    UPDATE settlement_route_verification_runs
    SET status = 'completed', examined_count = 2, activated_count = 1,
        already_active_count = 1, failed_count = 0, completed_at = now()
    WHERE id = run_id;

    BEGIN
        UPDATE settlement_route_verification_runs
        SET status = 'failed', failure_code = 'should_not_mutate'
        WHERE id = run_id;
        RAISE EXCEPTION 'terminal verification run mutation unexpectedly succeeded';
    EXCEPTION WHEN OTHERS THEN
        IF POSITION('terminal settlement route verification runs are immutable' IN SQLERRM) = 0 THEN
            RAISE;
        END IF;
    END;

    INSERT INTO settlement_route_verification_runs (run_kind, status, batch_size)
    VALUES ('scheduled', 'running', 1)
    RETURNING id INTO running_id;

    BEGIN
        UPDATE settlement_route_verification_runs
        SET status = 'completed', examined_count = 1, activated_count = 1,
            already_active_count = 0, failed_count = 1, completed_at = now()
        WHERE id = running_id;
        RAISE EXCEPTION 'inconsistent verification run counters unexpectedly succeeded';
    EXCEPTION WHEN OTHERS THEN
        IF POSITION('settlement_route_verification_runs' IN SQLERRM) = 0 THEN
            RAISE;
        END IF;
    END;

    BEGIN
        INSERT INTO settlement_route_verification_runs
            (run_kind, status, batch_size, examined_count, failed_count, completed_at)
        VALUES ('scheduled', 'completed', 1, 1, 1, now());
        RAISE EXCEPTION 'completed verification run without failure classification unexpectedly succeeded';
    EXCEPTION WHEN OTHERS THEN
        IF POSITION('settlement_route_verification_runs' IN SQLERRM) = 0 THEN
            RAISE;
        END IF;
    END;
END $$;
