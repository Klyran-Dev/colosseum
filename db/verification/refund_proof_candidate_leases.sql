\set ON_ERROR_STOP on

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_schema = 'public'
          AND table_name = 'indexed_adjustment_refund_executions'
          AND column_name = 'proof_claim_token'
          AND udt_name = 'uuid'
    ) THEN
        RAISE EXCEPTION 'refund proof candidate lease token is missing';
    END IF;
    IF NOT EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_schema = 'public'
          AND table_name = 'indexed_adjustment_refund_executions'
          AND column_name = 'proof_claimed_at'
          AND udt_name = 'timestamptz'
    ) THEN
        RAISE EXCEPTION 'refund proof candidate lease timestamp is missing';
    END IF;
    IF NOT EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_schema = 'public'
          AND table_name = 'indexed_adjustment_refund_executions'
          AND column_name = 'proof_claim_attempts'
          AND udt_name = 'int2'
    ) THEN
        RAISE EXCEPTION 'refund proof candidate attempt counter is missing';
    END IF;
    IF NOT EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_schema = 'public'
          AND table_name = 'indexed_adjustment_refund_executions'
          AND column_name = 'proof_claim_last_error'
          AND udt_name = 'text'
    ) OR NOT EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_schema = 'public'
          AND table_name = 'indexed_adjustment_refund_executions'
          AND column_name = 'proof_claim_dead_lettered_at'
          AND udt_name = 'timestamptz'
    ) THEN
        RAISE EXCEPTION 'refund proof bounded failure columns are missing';
    END IF;
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conrelid = 'public.indexed_adjustment_refund_executions'::regclass
          AND conname = 'indexed_refund_proof_lease_pair'
    ) THEN
        RAISE EXCEPTION 'refund proof candidate lease pair constraint is missing';
    END IF;
    IF NOT EXISTS (
        SELECT 1
        FROM pg_class
        WHERE relname = 'indexed_refund_proof_candidate_idx'
          AND relkind = 'i'
    ) THEN
        RAISE EXCEPTION 'refund proof candidate lease index is missing';
    END IF;
END
$$;
