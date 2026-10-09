-- Verification contract for the ADR-0413 applicability-binding projection.

SELECT 'applicability_binding_verifications' AS table_name,
       to_regclass('public.applicability_binding_verifications') IS NOT NULL AS table_exists;

SELECT conname,
       pg_get_constraintdef(oid) AS definition
FROM pg_constraint
WHERE conrelid = 'public.applicability_binding_verifications'::regclass
ORDER BY conname;
