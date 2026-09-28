-- Harden the refund movement lifecycle trigger without rewriting migration
-- 0042. All authority, financial, topology, and artifact fields are
-- immutable; only bounded transport/evidence fields may change after intent
-- creation.

BEGIN;

CREATE OR REPLACE FUNCTION prevent_refund_funding_configuration_mutation()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF OLD.status <> NEW.status
        AND NOT (
            (OLD.status = 'proposed' AND NEW.status = 'approved')
            OR (OLD.status = 'approved' AND NEW.status = 'pending_verification')
            OR (OLD.status = 'pending_verification' AND NEW.status = 'active')
            OR (OLD.status = 'active' AND NEW.status IN ('suspended', 'retired'))
            OR (OLD.status = 'suspended' AND NEW.status = 'retired')
        )
    THEN
        RAISE EXCEPTION 'invalid refund funding configuration status transition';
    END IF;

    IF NEW.status IN ('approved', 'pending_verification', 'active')
        AND NEW.refund_funder_subject IS NULL
    THEN
        RAISE EXCEPTION 'reviewed refund funding configurations require a funder subject';
    END IF;

    IF NEW.status = 'active' AND NEW.verification_digest IS NULL THEN
        RAISE EXCEPTION 'active refund funding configurations require finalized verification';
    END IF;

    IF OLD.status IN ('approved', 'pending_verification', 'active', 'suspended', 'retired') THEN
        IF NEW.merchant_id IS DISTINCT FROM OLD.merchant_id
            OR NEW.idempotency_key IS DISTINCT FROM OLD.idempotency_key
            OR NEW.request_digest IS DISTINCT FROM OLD.request_digest
            OR NEW.configuration_digest IS DISTINCT FROM OLD.configuration_digest
            OR NEW.configuration_key IS DISTINCT FROM OLD.configuration_key
            OR NEW.version IS DISTINCT FROM OLD.version
            OR NEW.merchant_account IS DISTINCT FROM OLD.merchant_account
            OR NEW.mint IS DISTINCT FROM OLD.mint
            OR NEW.tax_vault IS DISTINCT FROM OLD.tax_vault
            OR NEW.vault_authority IS DISTINCT FROM OLD.vault_authority
            OR NEW.refund_funder IS DISTINCT FROM OLD.refund_funder
            OR NEW.refund_funder_subject IS DISTINCT FROM OLD.refund_funder_subject
            OR NEW.net_source IS DISTINCT FROM OLD.net_source
            OR NEW.token_program IS DISTINCT FROM OLD.token_program
            OR NEW.proposer_subject IS DISTINCT FROM OLD.proposer_subject
            OR (OLD.status <> 'pending_verification'
                AND NEW.verification_digest IS DISTINCT FROM OLD.verification_digest)
            OR NEW.effective_from IS DISTINCT FROM OLD.effective_from
            OR NEW.effective_until IS DISTINCT FROM OLD.effective_until
        THEN
            RAISE EXCEPTION 'refund funding account fields are immutable; create a new version';
        END IF;
    END IF;

    IF OLD.status = 'retired' AND NEW.status <> 'retired' THEN
        RAISE EXCEPTION 'retired refund funding configurations cannot be reactivated';
    END IF;

    NEW.updated_at = now();
    RETURN NEW;
END;
$$;

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
    IF OLD.status = 'prepared_unsubmitted' AND NEW.status NOT IN ('signed', 'cancelled', 'rejected')
        OR OLD.status = 'signed' AND NEW.status NOT IN ('submitting', 'cancelled', 'rejected', 'unknown')
        OR OLD.status = 'submitting' AND NEW.status NOT IN ('submitted', 'unknown', 'rejected')
        OR OLD.status = 'submitted' AND NEW.status NOT IN ('processed', 'confirmed', 'finalized', 'unknown', 'rejected')
        OR OLD.status = 'processed' AND NEW.status NOT IN ('confirmed', 'finalized', 'unknown')
        OR OLD.status = 'confirmed' AND NEW.status NOT IN ('finalized', 'unknown')
        OR OLD.status IN ('finalized', 'rejected', 'cancelled') AND NEW.status <> OLD.status
    THEN
        RAISE EXCEPTION 'invalid refund claim intent status transition';
    END IF;
    NEW.updated_at = now();
    RETURN NEW;
END;
$$;

CREATE OR REPLACE FUNCTION validate_refund_execute_intent_transition()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF OLD.merchant_id IS DISTINCT FROM NEW.merchant_id
        OR OLD.idempotency_key IS DISTINCT FROM NEW.idempotency_key
        OR OLD.request_digest IS DISTINCT FROM NEW.request_digest
        OR OLD.claim_intent_id IS DISTINCT FROM NEW.claim_intent_id
        OR OLD.funding_configuration_id IS DISTINCT FROM NEW.funding_configuration_id
        OR OLD.funding_configuration_digest IS DISTINCT FROM NEW.funding_configuration_digest
        OR OLD.claim IS DISTINCT FROM NEW.claim
        OR OLD.adjustment_id IS DISTINCT FROM NEW.adjustment_id
        OR OLD.original_receipt IS DISTINCT FROM NEW.original_receipt
        OR OLD.merchant IS DISTINCT FROM NEW.merchant
        OR OLD.mint IS DISTINCT FROM NEW.mint
        OR OLD.token_program IS DISTINCT FROM NEW.token_program
        OR OLD.approver IS DISTINCT FROM NEW.approver
        OR OLD.refund_funder IS DISTINCT FROM NEW.refund_funder
        OR OLD.refund_funder_subject IS DISTINCT FROM NEW.refund_funder_subject
        OR OLD.instruction_digest IS DISTINCT FROM NEW.instruction_digest
        OR OLD.artifact_digest IS DISTINCT FROM NEW.artifact_digest
        OR OLD.unsigned_artifact IS DISTINCT FROM NEW.unsigned_artifact
        OR OLD.created_at IS DISTINCT FROM NEW.created_at
    THEN
        RAISE EXCEPTION 'refund execute authority, topology, and artifact fields are immutable';
    END IF;
    IF OLD.status = 'prepared_unsubmitted' AND NEW.status NOT IN ('signed', 'cancelled', 'rejected')
        OR OLD.status = 'signed' AND NEW.status NOT IN ('submitting', 'cancelled', 'rejected', 'unknown')
        OR OLD.status = 'submitting' AND NEW.status NOT IN ('submitted', 'unknown', 'rejected')
        OR OLD.status = 'submitted' AND NEW.status NOT IN ('processed', 'confirmed', 'finalized', 'unknown', 'rejected')
        OR OLD.status = 'processed' AND NEW.status NOT IN ('confirmed', 'finalized', 'unknown')
        OR OLD.status = 'confirmed' AND NEW.status NOT IN ('finalized', 'unknown')
        OR OLD.status IN ('finalized', 'rejected', 'cancelled') AND NEW.status <> OLD.status
    THEN
        RAISE EXCEPTION 'invalid refund execute intent status transition';
    END IF;
    NEW.updated_at = now();
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS refund_claim_intent_mutation_guard ON refund_claim_intents;
CREATE TRIGGER refund_claim_intent_mutation_guard
    BEFORE UPDATE ON refund_claim_intents
    FOR EACH ROW EXECUTE FUNCTION validate_refund_claim_intent_transition();

DROP TRIGGER IF EXISTS refund_execute_intent_mutation_guard ON refund_execute_intents;
CREATE TRIGGER refund_execute_intent_mutation_guard
    BEFORE UPDATE ON refund_execute_intents
    FOR EACH ROW EXECUTE FUNCTION validate_refund_execute_intent_transition();

COMMIT;
