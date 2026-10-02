-- Close UUIDv7 version-check coverage for every catalog-discovered generated
-- identifier. Existing invalid rows fail the migration and leave the schema
-- ledger unchanged; no generated identifier is silently grandfathered.

BEGIN;

DO $$
DECLARE
    generated_column record;
    constraint_name text;
    constraint_exists boolean;
    constraint_name_taken boolean;
BEGIN
    FOR generated_column IN
        SELECT table_schema, table_name, column_name
          FROM information_schema.columns
         WHERE table_schema = 'public'
           AND position(
               'uuidv7()' IN lower(replace(coalesce(column_default, ''), ' ', ''))
           ) > 0
         ORDER BY table_name, column_name
    LOOP
        constraint_name := left(
            format('%s_%s_uuidv7_check', generated_column.table_name, generated_column.column_name),
            55
        ) || '_' || substr(
            md5(generated_column.table_name || '.' || generated_column.column_name),
            1,
            7
        );

        SELECT EXISTS (
            SELECT 1
              FROM pg_constraint
             WHERE conrelid = format(
                 '%I.%I', generated_column.table_schema, generated_column.table_name
             )::regclass
               AND conname = constraint_name
               AND contype = 'c'
               AND pg_get_constraintdef(oid) LIKE format(
                   '%%uuid_extract_version(%I) = 7%%', generated_column.column_name
               )
        )
          INTO constraint_exists;

        IF NOT constraint_exists THEN
            SELECT EXISTS (
                SELECT 1
                  FROM pg_constraint
                 WHERE conrelid = format(
                     '%I.%I', generated_column.table_schema, generated_column.table_name
                 )::regclass
                   AND conname = constraint_name
            )
              INTO constraint_name_taken;

            IF constraint_name_taken THEN
                RAISE EXCEPTION 'UUIDv7 constraint name collision on %.%',
                    generated_column.table_name, generated_column.column_name;
            END IF;

            EXECUTE format(
                'ALTER TABLE %I.%I ADD CONSTRAINT %I CHECK (uuid_extract_version(%I) = 7) NOT VALID',
                generated_column.table_schema,
                generated_column.table_name,
                constraint_name,
                generated_column.column_name
            );
        END IF;

        EXECUTE format(
            'ALTER TABLE %I.%I VALIDATE CONSTRAINT %I',
            generated_column.table_schema,
            generated_column.table_name,
            constraint_name
        );
    END LOOP;
END
$$;

COMMIT;
