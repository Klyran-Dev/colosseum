DO $$
DECLARE
    scheduled_id uuid := uuidv7();
    backfill_id uuid := uuidv7();
    approval_id uuid := uuidv7();
BEGIN
    BEGIN
        INSERT INTO settlement_route_verification_runs
            (id, run_kind, status, batch_size)
        VALUES ('00000000-0000-0000-0000-000000000001', 'scheduled', 'running', 1);
        RAISE EXCEPTION 'route-verification run accepted a non-v7 run ID';
    EXCEPTION WHEN check_violation THEN
        NULL;
    END;

    INSERT INTO settlement_route_verification_runs (id, run_kind, status, batch_size)
    VALUES (scheduled_id, 'scheduled', 'running', 1);

    BEGIN
        INSERT INTO settlement_route_verification_runs
            (id, run_kind, status, batch_size, operator_approval_id)
        VALUES (uuidv7(), 'scheduled', 'running', 1, approval_id);
        RAISE EXCEPTION 'scheduled route-verification run accepted an approval ID';
    EXCEPTION WHEN check_violation THEN
        NULL;
    END;

    INSERT INTO settlement_route_verification_runs
        (id, run_kind, status, batch_size, operator_approval_id)
    VALUES (backfill_id, 'backfill', 'running', 1, approval_id);

    BEGIN
        INSERT INTO settlement_route_verification_runs (id, run_kind, status, batch_size)
        VALUES (uuidv7(), 'backfill', 'running', 1);
        RAISE EXCEPTION 'backfill route-verification run accepted a missing approval ID';
    EXCEPTION WHEN check_violation THEN
        NULL;
    END;

    BEGIN
        INSERT INTO settlement_route_verification_runs
            (id, run_kind, status, batch_size, operator_approval_id)
        VALUES (uuidv7(), 'backfill', 'running', 1, '00000000-0000-0000-0000-000000000001');
        RAISE EXCEPTION 'backfill route-verification run accepted a non-v7 approval ID';
    EXCEPTION WHEN check_violation THEN
        NULL;
    END;

    UPDATE settlement_route_verification_runs
    SET status = 'completed', completed_at = now()
    WHERE id IN (scheduled_id, backfill_id);

    BEGIN
        UPDATE settlement_route_verification_runs
        SET operator_approval_id = uuidv7()
        WHERE id = backfill_id;
        RAISE EXCEPTION 'terminal route-verification approval binding was mutable';
    EXCEPTION WHEN others THEN
        IF SQLERRM <> 'terminal settlement route verification runs are immutable' THEN
            RAISE;
        END IF;
    END;
END;
$$;
