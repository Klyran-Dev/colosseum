-- Expand the calculation lifecycle for the persisted published-rule quote path.
-- This migration is idempotent so local `make migrate` can be rerun safely.

BEGIN;

ALTER TABLE tax_calculations
    DROP CONSTRAINT IF EXISTS tax_calculations_status_check;

ALTER TABLE tax_calculations
    ADD CONSTRAINT tax_calculations_status_check
    CHECK (status IN ('simulated_unattested', 'calculated_unattested', 'attested', 'expired', 'superseded'));

ALTER TABLE idempotency_records
    DROP CONSTRAINT IF EXISTS idempotency_records_key_length_check;

ALTER TABLE idempotency_records
    ADD CONSTRAINT idempotency_records_key_length_check
    CHECK (length(idempotency_key) BETWEEN 1 AND 200);

ALTER TABLE idempotency_records
    DROP CONSTRAINT IF EXISTS idempotency_records_digest_length_check;

ALTER TABLE idempotency_records
    ADD CONSTRAINT idempotency_records_digest_length_check
    CHECK (octet_length(request_digest) = 32);

COMMIT;
