-- Permit a reconciler lease to update only its coordination fields without
-- pretending that the cancellation lifecycle has advanced.

BEGIN;

CREATE OR REPLACE FUNCTION validate_refund_claim_cancellation_transition()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF OLD.merchant_id IS DISTINCT FROM NEW.merchant_id
        OR OLD.idempotency_key IS DISTINCT FROM NEW.idempotency_key
        OR OLD.request_digest IS DISTINCT FROM NEW.request_digest
        OR OLD.claim_intent_id IS DISTINCT FROM NEW.claim_intent_id
        OR OLD.adjustment_id IS DISTINCT FROM NEW.adjustment_id
        OR OLD.merchant IS DISTINCT FROM NEW.merchant
        OR OLD.mint IS DISTINCT FROM NEW.mint
        OR OLD.adjustment IS DISTINCT FROM NEW.adjustment
        OR OLD.claim_nonce IS DISTINCT FROM NEW.claim_nonce
        OR OLD.claim IS DISTINCT FROM NEW.claim
        OR OLD.governance_authority IS DISTINCT FROM NEW.governance_authority
        OR OLD.governance_subject IS DISTINCT FROM NEW.governance_subject
        OR OLD.instruction_digest IS DISTINCT FROM NEW.instruction_digest
        OR OLD.artifact_digest IS DISTINCT FROM NEW.artifact_digest
        OR OLD.unsigned_artifact IS DISTINCT FROM NEW.unsigned_artifact
        OR OLD.created_at IS DISTINCT FROM NEW.created_at
    THEN
        RAISE EXCEPTION 'refund claim cancellation authority and artifact fields are immutable';
    END IF;

    IF OLD.status = NEW.status THEN
        IF OLD.signed_transaction_digest IS DISTINCT FROM NEW.signed_transaction_digest
            OR OLD.signature IS DISTINCT FROM NEW.signature
            OR OLD.submitted_slot IS DISTINCT FROM NEW.submitted_slot
            OR OLD.commitment IS DISTINCT FROM NEW.commitment
            OR OLD.last_error_code IS DISTINCT FROM NEW.last_error_code
        THEN
            RAISE EXCEPTION 'same-status refund claim cancellation updates may only change reconciliation lease fields';
        END IF;
    ELSIF OLD.status = 'prepared_unsubmitted' AND NEW.status NOT IN ('signed', 'rejected')
        OR OLD.status = 'signed' AND NEW.status NOT IN ('submitting', 'rejected', 'unknown')
        OR OLD.status = 'submitting' AND NEW.status NOT IN ('submitted', 'unknown', 'rejected')
        OR OLD.status = 'submitted' AND NEW.status NOT IN ('processed', 'confirmed', 'finalized', 'unknown', 'rejected')
        OR OLD.status = 'processed' AND NEW.status NOT IN ('confirmed', 'finalized', 'unknown')
        OR OLD.status = 'confirmed' AND NEW.status NOT IN ('finalized', 'unknown')
        OR OLD.status = 'unknown' AND NEW.status NOT IN ('submitted', 'processed', 'confirmed', 'finalized', 'rejected')
        OR OLD.status IN ('finalized', 'rejected') AND NEW.status <> OLD.status
    THEN
        RAISE EXCEPTION 'invalid refund claim cancellation status transition';
    END IF;

    NEW.updated_at = now();
    RETURN NEW;
END;
$$;

COMMIT;
