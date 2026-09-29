DO $$
DECLARE
    event_index_type text;
    event_digest_type text;
    proof_index_exists boolean;
BEGIN
    SELECT format_type(a.atttypid, a.atttypmod)
      INTO event_index_type
      FROM pg_attribute a
      JOIN pg_class c ON c.oid = a.attrelid
      JOIN pg_namespace n ON n.oid = c.relnamespace
     WHERE n.nspname = 'public'
       AND c.relname = 'settlement_adjustment_chain_intents'
       AND a.attname = 'finalized_event_index'
       AND NOT a.attisdropped;
    IF event_index_type <> 'smallint' THEN
        RAISE EXCEPTION 'finalized event index column contract is missing or invalid: %', event_index_type;
    END IF;

    SELECT format_type(a.atttypid, a.atttypmod)
      INTO event_digest_type
      FROM pg_attribute a
      JOIN pg_class c ON c.oid = a.attrelid
      JOIN pg_namespace n ON n.oid = c.relnamespace
     WHERE n.nspname = 'public'
       AND c.relname = 'settlement_adjustment_chain_intents'
       AND a.attname = 'finalized_event_evidence_digest'
       AND NOT a.attisdropped;
    IF event_digest_type <> 'bytea' THEN
        RAISE EXCEPTION 'finalized event digest column contract is missing or invalid: %', event_digest_type;
    END IF;

    SELECT EXISTS (
        SELECT 1
          FROM pg_indexes
         WHERE schemaname = 'public'
           AND indexname = 'settlement_adjustment_chain_finalized_proof_idx'
    ) INTO proof_index_exists;
    IF NOT proof_index_exists THEN
        RAISE EXCEPTION 'finalized adjustment proof index is missing';
    END IF;
END
$$;
