-- Executable schema contract for application-generated sortable identifiers.
-- UUID references, external tenant claims, Solana identifiers, cryptographic
-- digests, and replay claim tokens are intentionally not in this set. The
-- catalog query is deliberately dynamic so later migrations cannot silently
-- add a uuidv7() default outside this contract.

CREATE TEMP TABLE expected_uuidv7_columns (
    table_name text NOT NULL,
    column_name text NOT NULL,
    PRIMARY KEY (table_name, column_name)
);

INSERT INTO expected_uuidv7_columns (table_name, column_name)
SELECT table_name, column_name
  FROM information_schema.columns
 WHERE table_schema = 'public'
   AND position(
       'uuidv7()' IN lower(replace(coalesce(column_default, ''), ' ', ''))
   ) > 0;

DO $$
DECLARE
    missing_columns text;
    missing_defaults text;
    missing_checks text;
BEGIN
    IF NOT EXISTS (SELECT 1 FROM expected_uuidv7_columns) THEN
        RAISE EXCEPTION 'UUIDv7 contract found no public uuidv7() defaults';
    END IF;

    SELECT string_agg(format('%s.%s', expected.table_name, expected.column_name), ', ' ORDER BY expected.table_name, expected.column_name)
      INTO missing_columns
      FROM expected_uuidv7_columns AS expected
      LEFT JOIN information_schema.columns AS actual
        ON actual.table_schema = 'public'
       AND actual.table_name = expected.table_name
       AND actual.column_name = expected.column_name
      WHERE actual.data_type IS DISTINCT FROM 'uuid';

    IF missing_columns IS NOT NULL THEN
        RAISE EXCEPTION 'UUIDv7 contract has non-uuid or missing columns: %', missing_columns;
    END IF;

    SELECT string_agg(format('%s.%s', expected.table_name, expected.column_name), ', ' ORDER BY expected.table_name, expected.column_name)
      INTO missing_defaults
      FROM expected_uuidv7_columns AS expected
      JOIN information_schema.columns AS actual
        ON actual.table_schema = 'public'
       AND actual.table_name = expected.table_name
       AND actual.column_name = expected.column_name
      WHERE actual.column_default IS NULL
         OR position(
             'uuidv7()' IN lower(replace(actual.column_default, ' ', ''))
         ) = 0;

    IF missing_defaults IS NOT NULL THEN
        RAISE EXCEPTION 'UUIDv7 contract has missing/non-UUIDv7 defaults: %', missing_defaults;
    END IF;

    SELECT string_agg(format('%s.%s', expected.table_name, expected.column_name), ', ' ORDER BY expected.table_name, expected.column_name)
      INTO missing_checks
      FROM expected_uuidv7_columns AS expected
      WHERE NOT EXISTS (
          SELECT 1
            FROM pg_constraint AS constraint_row
           WHERE constraint_row.conrelid = to_regclass(format('public.%I', expected.table_name))
             AND constraint_row.contype = 'c'
             AND constraint_row.convalidated
             AND pg_get_constraintdef(constraint_row.oid) LIKE format('%%uuid_extract_version(%I) = 7%%', expected.column_name)
      );

    IF missing_checks IS NOT NULL THEN
        RAISE EXCEPTION 'UUIDv7 contract has missing version checks: %', missing_checks;
    END IF;
END
$$;

DROP TABLE expected_uuidv7_columns;
