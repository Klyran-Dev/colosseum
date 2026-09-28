-- Tenant-scoped vault-release command/evidence projection.
-- Solana claim accounts and finalized token balances remain authoritative.

BEGIN;

CREATE TABLE IF NOT EXISTS vault_release_claims (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    merchant_id uuid NOT NULL,
    idempotency_key text NOT NULL,
    request_digest bytea NOT NULL,
    mint text NOT NULL,
    token_program text NOT NULL,
    tax_vault text NOT NULL,
    destination text NOT NULL,
    claim_pda text,
    nonce numeric(20, 0) NOT NULL,
    amount_atomic numeric(20, 0) NOT NULL,
    reference_digest bytea NOT NULL,
    status text NOT NULL DEFAULT 'proposed',
    proposer_subject text NOT NULL,
    approver_subject text,
    execute_after timestamptz NOT NULL,
    expires_at timestamptz NOT NULL,
    transaction_signature text,
    slot numeric(20, 0),
    last_error_code text,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK (uuid_extract_version(id) = 7),
    CHECK (length(idempotency_key) BETWEEN 1 AND 200),
    CHECK (octet_length(request_digest) = 32),
    CHECK (length(btrim(mint)) BETWEEN 32 AND 64),
    CHECK (length(btrim(token_program)) BETWEEN 32 AND 64),
    CHECK (length(btrim(tax_vault)) BETWEEN 32 AND 64),
    CHECK (length(btrim(destination)) BETWEEN 32 AND 64),
    CHECK (claim_pda IS NULL OR length(btrim(claim_pda)) BETWEEN 32 AND 64),
    CHECK (nonce >= 0 AND nonce <= 18446744073709551615),
    CHECK (amount_atomic > 0 AND amount_atomic <= 18446744073709551615),
    CHECK (octet_length(reference_digest) = 32),
    CHECK (reference_digest <> repeat(E'\\000', 32)::bytea),
    CHECK (status IN ('proposed', 'approved', 'onchain_created', 'executed',
                      'cancelled', 'unknown')),
    CHECK (length(btrim(proposer_subject)) BETWEEN 1 AND 256),
    CHECK (approver_subject IS NULL OR length(btrim(approver_subject)) BETWEEN 1 AND 256),
    CHECK (approver_subject IS NULL OR approver_subject <> proposer_subject),
    CHECK (expires_at > execute_after),
    CHECK (expires_at <= execute_after + interval '90 days'),
    CHECK (transaction_signature IS NULL OR length(btrim(transaction_signature)) BETWEEN 1 AND 128),
    CHECK (slot IS NULL OR (slot >= 0 AND slot <= 18446744073709551615)),
    CHECK (last_error_code IS NULL OR length(btrim(last_error_code)) BETWEEN 1 AND 128),
    UNIQUE (merchant_id, idempotency_key),
    UNIQUE (merchant_id, mint, nonce),
    UNIQUE (id, merchant_id)
);

CREATE UNIQUE INDEX IF NOT EXISTS vault_release_claims_pda_idx
    ON vault_release_claims (claim_pda) WHERE claim_pda IS NOT NULL;
CREATE INDEX IF NOT EXISTS vault_release_claims_status_idx
    ON vault_release_claims (merchant_id, status, updated_at DESC, id DESC);
CREATE INDEX IF NOT EXISTS vault_release_claims_signature_idx
    ON vault_release_claims (transaction_signature) WHERE transaction_signature IS NOT NULL;

CREATE TABLE IF NOT EXISTS vault_release_claim_events (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    claim_id uuid NOT NULL,
    merchant_id uuid NOT NULL,
    actor_subject text NOT NULL,
    action text NOT NULL CHECK (action IN ('propose', 'approve', 'chain_created', 'execute', 'cancel', 'unknown')),
    from_status text NOT NULL,
    to_status text NOT NULL,
    evidence jsonb NOT NULL DEFAULT '{}'::jsonb,
    occurred_at timestamptz NOT NULL DEFAULT now(),
    CHECK (uuid_extract_version(id) = 7),
    CHECK (length(btrim(actor_subject)) BETWEEN 1 AND 256),
    CHECK (jsonb_typeof(evidence) = 'object'),
    FOREIGN KEY (claim_id, merchant_id) REFERENCES vault_release_claims(id, merchant_id),
    UNIQUE (claim_id, id)
);

CREATE INDEX IF NOT EXISTS vault_release_claim_events_claim_idx
    ON vault_release_claim_events (claim_id, occurred_at ASC, id ASC);

CREATE OR REPLACE FUNCTION prevent_vault_release_claim_event_mutation()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    RAISE EXCEPTION 'vault release claim events are append-only';
END;
$$;

DROP TRIGGER IF EXISTS vault_release_claim_event_mutation_guard ON vault_release_claim_events;
CREATE TRIGGER vault_release_claim_event_mutation_guard
    BEFORE UPDATE OR DELETE ON vault_release_claim_events
    FOR EACH ROW
    EXECUTE FUNCTION prevent_vault_release_claim_event_mutation();

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
        OR NEW.claim_pda IS DISTINCT FROM OLD.claim_pda
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

DROP TRIGGER IF EXISTS vault_release_claim_transition_guard ON vault_release_claims;
CREATE TRIGGER vault_release_claim_transition_guard
    BEFORE UPDATE ON vault_release_claims
    FOR EACH ROW
    EXECUTE FUNCTION validate_vault_release_claim_transition();

COMMIT;
