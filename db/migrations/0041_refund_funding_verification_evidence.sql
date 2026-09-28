-- Finalized chain evidence for refund-funding topology activation.
-- The verifier, not the browser, is the only writer of these fields.

BEGIN;

ALTER TABLE refund_funding_configurations
    ADD COLUMN IF NOT EXISTS verification_slot numeric(20, 0),
    ADD COLUMN IF NOT EXISTS verification_commitment text,
    ADD COLUMN IF NOT EXISTS verification_evidence jsonb,
    ADD COLUMN IF NOT EXISTS verifier_version text,
    ADD COLUMN IF NOT EXISTS verified_at timestamptz;

ALTER TABLE refund_funding_configurations
    DROP CONSTRAINT IF EXISTS refund_funding_verification_contract;

ALTER TABLE refund_funding_configurations
    ADD CONSTRAINT refund_funding_verification_contract CHECK (
        (verification_slot IS NULL AND verification_commitment IS NULL
         AND verification_evidence IS NULL AND verifier_version IS NULL
         AND verified_at IS NULL)
        OR
        (verification_slot IS NOT NULL AND verification_slot >= 1
         AND verification_slot <= 18446744073709551615
         AND verification_commitment = 'finalized'
         AND verification_evidence IS NOT NULL
         AND octet_length(verification_evidence::text) BETWEEN 1 AND 32768
         AND verifier_version IS NOT NULL
         AND length(verifier_version) BETWEEN 1 AND 64
         AND verified_at IS NOT NULL)
    );

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

    IF NEW.status = 'approved' AND NEW.approver_subject IS NULL THEN
        RAISE EXCEPTION 'approved refund funding configurations require an approver';
    END IF;

    IF NEW.status = 'active'
        AND (NEW.verification_digest IS NULL OR NEW.verification_slot IS NULL
             OR NEW.verification_commitment IS DISTINCT FROM 'finalized'
             OR NEW.verification_evidence IS NULL
             OR NEW.verifier_version IS NULL OR NEW.verified_at IS NULL)
    THEN
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
            OR NEW.net_source IS DISTINCT FROM OLD.net_source
            OR NEW.token_program IS DISTINCT FROM OLD.token_program
            OR NEW.proposer_subject IS DISTINCT FROM OLD.proposer_subject
            OR NEW.approver_subject IS DISTINCT FROM OLD.approver_subject
            OR NEW.effective_from IS DISTINCT FROM OLD.effective_from
            OR NEW.effective_until IS DISTINCT FROM OLD.effective_until
        THEN
            RAISE EXCEPTION 'refund funding account fields are immutable; create a new version';
        END IF;
    END IF;

    IF OLD.status IN ('active', 'suspended', 'retired')
        AND (NEW.verification_digest IS DISTINCT FROM OLD.verification_digest
             OR NEW.verification_slot IS DISTINCT FROM OLD.verification_slot
             OR NEW.verification_commitment IS DISTINCT FROM OLD.verification_commitment
             OR NEW.verification_evidence IS DISTINCT FROM OLD.verification_evidence
             OR NEW.verifier_version IS DISTINCT FROM OLD.verifier_version
             OR NEW.verified_at IS DISTINCT FROM OLD.verified_at)
    THEN
        RAISE EXCEPTION 'refund funding verification evidence is immutable';
    END IF;

    IF OLD.status = 'retired' AND NEW.status <> 'retired' THEN
        RAISE EXCEPTION 'retired refund funding configurations cannot be reactivated';
    END IF;

    NEW.updated_at = now();
    RETURN NEW;
END;
$$;

COMMIT;
