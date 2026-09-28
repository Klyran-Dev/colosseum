-- Tighten vault-release state invariants without rewriting the applied 0023 migration.
-- Base claim intent remains immutable after approval; chain evidence is advanced by
-- the bounded transition command and preserved in the append-only event stream.

BEGIN;

CREATE OR REPLACE FUNCTION validate_vault_release_claim_transition()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NOT (
        (OLD.status = 'proposed' AND NEW.status IN ('approved', 'onchain_created', 'cancelled', 'unknown'))
        OR (OLD.status = 'approved' AND NEW.status IN ('onchain_created', 'cancelled', 'unknown'))
        OR (OLD.status = 'onchain_created' AND NEW.status IN ('executed', 'cancelled', 'unknown'))
        OR (OLD.status = 'unknown' AND NEW.status IN ('onchain_created', 'executed', 'cancelled'))
        OR (OLD.status = NEW.status)
    ) THEN
        RAISE EXCEPTION 'invalid vault release claim transition from % to %', OLD.status, NEW.status;
    END IF;
    IF OLD.status IN ('approved', 'onchain_created', 'executed', 'cancelled') AND (
        NEW.merchant_id IS DISTINCT FROM OLD.merchant_id
        OR NEW.idempotency_key IS DISTINCT FROM OLD.idempotency_key
        OR NEW.request_digest IS DISTINCT FROM OLD.request_digest
        OR NEW.mint IS DISTINCT FROM OLD.mint
        OR NEW.token_program IS DISTINCT FROM OLD.token_program
        OR NEW.tax_vault IS DISTINCT FROM OLD.tax_vault
        OR NEW.destination IS DISTINCT FROM OLD.destination
        OR (OLD.claim_pda IS NOT NULL AND NEW.claim_pda IS DISTINCT FROM OLD.claim_pda)
        OR NEW.nonce IS DISTINCT FROM OLD.nonce
        OR NEW.amount_atomic IS DISTINCT FROM OLD.amount_atomic
        OR NEW.reference_digest IS DISTINCT FROM OLD.reference_digest
        OR NEW.proposer_subject IS DISTINCT FROM OLD.proposer_subject
        OR NEW.execute_after IS DISTINCT FROM OLD.execute_after
        OR NEW.expires_at IS DISTINCT FROM OLD.expires_at
    ) THEN
        RAISE EXCEPTION 'approved and finalized vault release claim fields are immutable';
    END IF;
    RETURN NEW;
END;
$$;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conrelid = 'vault_release_claims'::regclass
          AND conname = 'vault_release_claim_chain_evidence_check'
    ) THEN
        ALTER TABLE vault_release_claims
            ADD CONSTRAINT vault_release_claim_chain_evidence_check CHECK (
                status NOT IN ('onchain_created', 'executed')
                OR (claim_pda IS NOT NULL AND transaction_signature IS NOT NULL AND slot IS NOT NULL)
            );
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conrelid = 'vault_release_claims'::regclass
          AND conname = 'vault_release_claim_unknown_evidence_check'
    ) THEN
        ALTER TABLE vault_release_claims
            ADD CONSTRAINT vault_release_claim_unknown_evidence_check CHECK (
                status <> 'unknown' OR last_error_code IS NOT NULL
            );
    END IF;
END;
$$;

COMMIT;
