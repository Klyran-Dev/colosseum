-- Verification contract for the ADR-0414 worker coordination table.

SELECT 'applicability_binding_verification_jobs' AS table_name,
       to_regclass('public.applicability_binding_verification_jobs') IS NOT NULL AS table_exists;

SELECT conname,
       pg_get_constraintdef(oid) AS definition
FROM pg_constraint
WHERE conrelid = 'public.applicability_binding_verification_jobs'::regclass
ORDER BY conname;

SELECT 'applicability_binding_verification_jobs.id_uuidv7' AS contract,
       EXISTS (
           SELECT 1
           FROM pg_constraint
           WHERE conrelid = 'public.applicability_binding_verification_jobs'::regclass
             AND contype = 'c'
             AND convalidated
             AND pg_get_constraintdef(oid) LIKE '%uuid_extract_version(id) = 7%'
       ) AS passed;
