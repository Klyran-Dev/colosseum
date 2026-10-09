-- Complete the UUIDv7 invariant for the worker coordination identifier.
-- This is additive so databases that already applied 0072 receive the same
-- protection as clean bootstraps without rewriting migration history.

BEGIN;

ALTER TABLE applicability_binding_verification_jobs
    ADD CONSTRAINT applicability_binding_verification_jobs_id_uuidv7_check
    CHECK (uuid_extract_version(id) = 7) NOT VALID;

ALTER TABLE applicability_binding_verification_jobs
    VALIDATE CONSTRAINT applicability_binding_verification_jobs_id_uuidv7_check;

COMMIT;
