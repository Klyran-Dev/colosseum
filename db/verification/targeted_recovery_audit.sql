-- PostgreSQL contract for ADR-0126 targeted-recovery audit reconciliation.
-- Run only against a disposable verification database.

BEGIN;

INSERT INTO indexer_targeted_recovery_actions
    (request_id, stream_key, target_address, target_signature, operator_subject,
     note, outcome, pages_scanned, slot, error_code, created_at)
VALUES
    ('01992f1b-4a70-7e30-8ea2-b6e37a9d9d41', 'localnet/settlement/v1',
     repeat('1', 32), repeat('5', 88), 'verification-operator',
     'synthetic stale request', 'requested', NULL, NULL, NULL,
     now() - interval '6 minutes'),
    ('01992f1b-4a70-7e30-8ea2-b6e37a9d9d42', 'localnet/settlement/v1',
     repeat('1', 32), repeat('6', 88), 'verification-operator',
     'synthetic completed request', 'requested', NULL, NULL, NULL,
     now() - interval '6 minutes'),
    ('01992f1b-4a70-7e30-8ea2-b6e37a9d9d42', 'localnet/settlement/v1',
     repeat('1', 32), repeat('6', 88), 'verification-operator',
     'synthetic completed request', 'recovered', 8, 42, NULL,
     now() - interval '5 minutes'),
    ('01992f1b-4a70-7e30-8ea2-b6e37a9d9d43', 'localnet/settlement/v1',
     repeat('1', 32), repeat('7', 88), 'verification-operator',
     'synthetic failed request', 'requested', NULL, NULL, NULL,
     now() - interval '6 minutes'),
    ('01992f1b-4a70-7e30-8ea2-b6e37a9d9d43', 'localnet/settlement/v1',
     repeat('1', 32), repeat('7', 88), 'verification-operator',
     'synthetic failed request', 'failed', NULL, NULL,
     'provider_failure', now() - interval '5 minutes');

DO $$
DECLARE
    stale_count bigint;
    mutated boolean := false;
    deleted boolean := false;
BEGIN
    SELECT COUNT(*) INTO stale_count
    FROM indexer_targeted_recovery_actions requested
    WHERE requested.outcome = 'requested'
      AND requested.created_at < now() - interval '5 minutes'
      AND NOT EXISTS (
          SELECT 1
          FROM indexer_targeted_recovery_actions terminal
          WHERE terminal.request_id = requested.request_id
            AND terminal.outcome IN ('recovered', 'failed')
      );
    IF stale_count <> 1 THEN
        RAISE EXCEPTION 'expected exactly one stale targeted-recovery request, found %', stale_count;
    END IF;

    BEGIN
        UPDATE indexer_targeted_recovery_actions
        SET note = 'mutation must fail'
        WHERE request_id = '01992f1b-4a70-7e30-8ea2-b6e37a9d9d41';
        mutated := true;
    EXCEPTION WHEN OTHERS THEN
        NULL;
    END;
    IF mutated THEN
        RAISE EXCEPTION 'targeted-recovery audit UPDATE unexpectedly succeeded';
    END IF;

    BEGIN
        DELETE FROM indexer_targeted_recovery_actions
        WHERE request_id = '01992f1b-4a70-7e30-8ea2-b6e37a9d9d41';
        deleted := true;
    EXCEPTION WHEN OTHERS THEN
        NULL;
    END;
    IF deleted THEN
        RAISE EXCEPTION 'targeted-recovery audit DELETE unexpectedly succeeded';
    END IF;
END;
$$;

ROLLBACK;
