DO $$
DECLARE
    token_type text;
    claimed_type text;
    lease_constraint text;
    lease_index_exists boolean;
BEGIN
    SELECT data_type INTO token_type
    FROM information_schema.columns
    WHERE table_schema = 'public'
      AND table_name = 'refund_funding_configurations'
      AND column_name = 'verification_lease_token';
    IF token_type <> 'uuid' THEN
        RAISE EXCEPTION 'refund funding verification lease token must be uuid, got %', token_type;
    END IF;

    SELECT data_type INTO claimed_type
    FROM information_schema.columns
    WHERE table_schema = 'public'
      AND table_name = 'refund_funding_configurations'
      AND column_name = 'verification_lease_claimed_at';
    IF claimed_type <> 'timestamp with time zone' THEN
        RAISE EXCEPTION 'refund funding verification lease timestamp must be timestamptz, got %', claimed_type;
    END IF;

    SELECT pg_get_constraintdef(oid) INTO lease_constraint
    FROM pg_constraint
    WHERE conrelid = 'public.refund_funding_configurations'::regclass
      AND conname = 'refund_funding_verification_lease_pair';
    IF lease_constraint IS NULL
       OR lease_constraint NOT LIKE '%verification_lease_token%'
       OR lease_constraint NOT LIKE '%verification_lease_claimed_at%' THEN
        RAISE EXCEPTION 'refund funding verification lease pair constraint is missing or incomplete';
    END IF;

    SELECT EXISTS (
        SELECT 1
        FROM pg_indexes
        WHERE schemaname = 'public'
          AND tablename = 'refund_funding_configurations'
          AND indexname = 'refund_funding_verification_lease_idx'
    ) INTO lease_index_exists;
    IF NOT lease_index_exists THEN
        RAISE EXCEPTION 'refund funding verification lease index is missing';
    END IF;
END;
$$;
