-- Permit exactly one evidence-gated lifecycle convergence: a finalized
-- refund-claim creation may become cancelled only inside the storage method
-- that has already matched a finalized AdjustmentRefundClaimCancelled event.
-- The transaction-local setting is a database defense-in-depth boundary; API
-- authorization and event matching remain mandatory in the application.

BEGIN;

CREATE OR REPLACE FUNCTION validate_refund_claim_intent_transition()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF OLD.merchant_id IS DISTINCT FROM NEW.merchant_id
        OR OLD.idempotency_key IS DISTINCT FROM NEW.idempotency_key
        OR OLD.request_digest IS DISTINCT FROM NEW.request_digest
        OR OLD.adjustment_id IS DISTINCT FROM NEW.adjustment_id
        OR OLD.funding_configuration_id IS DISTINCT FROM NEW.funding_configuration_id
        OR OLD.funding_configuration_digest IS DISTINCT FROM NEW.funding_configuration_digest
        OR OLD.receipt_signature IS DISTINCT FROM NEW.receipt_signature
        OR OLD.receipt_slot IS DISTINCT FROM NEW.receipt_slot
        OR OLD.original_receipt IS DISTINCT FROM NEW.original_receipt
        OR OLD.payer IS DISTINCT FROM NEW.payer
        OR OLD.merchant IS DISTINCT FROM NEW.merchant
        OR OLD.mint IS DISTINCT FROM NEW.mint
        OR OLD.token_program IS DISTINCT FROM NEW.token_program
        OR OLD.proposer IS DISTINCT FROM NEW.proposer
        OR OLD.refund_funder IS DISTINCT FROM NEW.refund_funder
        OR OLD.refund_funder_subject IS DISTINCT FROM NEW.refund_funder_subject
        OR OLD.tax_vault IS DISTINCT FROM NEW.tax_vault
        OR OLD.net_source IS DISTINCT FROM NEW.net_source
        OR OLD.refund_destination IS DISTINCT FROM NEW.refund_destination
        OR OLD.claim_nonce IS DISTINCT FROM NEW.claim_nonce
        OR OLD.gross_atomic IS DISTINCT FROM NEW.gross_atomic
        OR OLD.tax_atomic IS DISTINCT FROM NEW.tax_atomic
        OR OLD.net_atomic IS DISTINCT FROM NEW.net_atomic
        OR OLD.reference_digest IS DISTINCT FROM NEW.reference_digest
        OR OLD.execute_after_unix IS DISTINCT FROM NEW.execute_after_unix
        OR OLD.expires_at_unix IS DISTINCT FROM NEW.expires_at_unix
        OR OLD.claim IS DISTINCT FROM NEW.claim
        OR OLD.instruction_digest IS DISTINCT FROM NEW.instruction_digest
        OR OLD.artifact_digest IS DISTINCT FROM NEW.artifact_digest
        OR OLD.unsigned_artifact IS DISTINCT FROM NEW.unsigned_artifact
        OR OLD.created_at IS DISTINCT FROM NEW.created_at
    THEN
        RAISE EXCEPTION 'refund claim authority, financial, topology, and artifact fields are immutable';
    END IF;
    IF OLD.status = 'finalized' AND NEW.status = 'cancelled' THEN
        IF current_setting('app.refund_claim_cancellation_reconciliation', true) <> 'true' THEN
            RAISE EXCEPTION 'finalized refund claim cancellation requires reconciliation evidence';
        END IF;
    ELSIF OLD.status = 'prepared_unsubmitted' AND NEW.status NOT IN ('signed', 'cancelled', 'rejected')
        OR OLD.status = 'signed' AND NEW.status NOT IN ('submitting', 'cancelled', 'rejected', 'unknown')
        OR OLD.status = 'submitting' AND NEW.status NOT IN ('submitted', 'unknown', 'rejected')
        OR OLD.status = 'submitted' AND NEW.status NOT IN ('processed', 'confirmed', 'finalized', 'unknown', 'rejected')
        OR OLD.status = 'processed' AND NEW.status NOT IN ('confirmed', 'finalized', 'unknown')
        OR OLD.status = 'confirmed' AND NEW.status NOT IN ('finalized', 'unknown')
        OR OLD.status = 'unknown' AND NEW.status NOT IN ('submitted', 'processed', 'confirmed', 'finalized', 'rejected')
        OR OLD.status IN ('finalized', 'rejected', 'cancelled') AND NEW.status <> OLD.status
    THEN
        RAISE EXCEPTION 'invalid refund claim intent status transition';
    END IF;
    NEW.updated_at = now();
    RETURN NEW;
END;
$$;

COMMIT;
