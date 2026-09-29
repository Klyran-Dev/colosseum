\set ON_ERROR_STOP on

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_schema = 'public'
          AND table_name = 'refund_claim_intents'
          AND column_name = 'reconciliation_claim_token'
          AND udt_name = 'uuid'
    ) THEN
        RAISE EXCEPTION 'refund claim reconciliation lease token is missing';
    END IF;
    IF NOT EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_schema = 'public'
          AND table_name = 'refund_claim_intents'
          AND column_name = 'reconciliation_claimed_at'
          AND udt_name = 'timestamptz'
    ) THEN
        RAISE EXCEPTION 'refund claim reconciliation lease timestamp is missing';
    END IF;
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conrelid = 'public.refund_claim_intents'::regclass
          AND conname = 'refund_claim_reconciliation_lease_pair'
    ) THEN
        RAISE EXCEPTION 'refund claim reconciliation lease pair constraint is missing';
    END IF;
    IF NOT EXISTS (
        SELECT 1
        FROM pg_class
        WHERE relname = 'refund_claim_reconciliation_lease_idx'
          AND relkind = 'i'
    ) THEN
        RAISE EXCEPTION 'refund claim reconciliation lease index is missing';
    END IF;
END
$$;
