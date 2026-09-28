-- Cryptographic wallet bindings and durable record-adjustment compiler
-- intents. These rows bind authenticated workflow subjects to public keys;
-- they never contain private keys and do not authorize token movement.

BEGIN;

CREATE TABLE IF NOT EXISTS settlement_adjustment_authority_challenges (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    merchant_id uuid NOT NULL,
    subject text NOT NULL,
    role text NOT NULL,
    message_digest bytea NOT NULL,
    expires_at timestamptz NOT NULL,
    consumed_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    CHECK (uuid_extract_version(id) = 7),
    CHECK (length(btrim(subject)) BETWEEN 1 AND 256),
    CHECK (role IN ('proposer', 'approver')),
    CHECK (octet_length(message_digest) = 32 AND message_digest <> repeat(E'\\000', 32)::bytea),
    CHECK (expires_at > created_at),
    UNIQUE (id, merchant_id)
);

CREATE INDEX IF NOT EXISTS settlement_adjustment_authority_challenge_idx
    ON settlement_adjustment_authority_challenges
       (merchant_id, subject, role, expires_at, id)
    WHERE consumed_at IS NULL;

CREATE TABLE IF NOT EXISTS settlement_adjustment_authority_bindings (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    merchant_id uuid NOT NULL,
    subject text NOT NULL,
    role text NOT NULL,
    public_key bytea NOT NULL,
    challenge_id uuid NOT NULL,
    proof_signature bytea NOT NULL,
    status text NOT NULL DEFAULT 'active',
    created_at timestamptz NOT NULL DEFAULT now(),
    revoked_at timestamptz,
    CHECK (uuid_extract_version(id) = 7),
    CHECK (length(btrim(subject)) BETWEEN 1 AND 256),
    CHECK (role IN ('proposer', 'approver')),
    CHECK (octet_length(public_key) = 32 AND public_key <> repeat(E'\\000', 32)::bytea),
    CHECK (octet_length(proof_signature) = 64),
    CHECK (status IN ('active', 'revoked')),
    CHECK ((status = 'active' AND revoked_at IS NULL)
        OR (status = 'revoked' AND revoked_at IS NOT NULL)),
    UNIQUE (id, merchant_id),
    FOREIGN KEY (challenge_id, merchant_id)
        REFERENCES settlement_adjustment_authority_challenges(id, merchant_id)
);

CREATE UNIQUE INDEX IF NOT EXISTS settlement_adjustment_authority_active_subject_idx
    ON settlement_adjustment_authority_bindings (merchant_id, subject, role)
    WHERE status = 'active';

CREATE UNIQUE INDEX IF NOT EXISTS settlement_adjustment_authority_active_key_idx
    ON settlement_adjustment_authority_bindings (merchant_id, public_key)
    WHERE status = 'active';

CREATE OR REPLACE FUNCTION prevent_adjustment_authority_binding_mutation()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF OLD.merchant_id IS DISTINCT FROM NEW.merchant_id
        OR OLD.subject IS DISTINCT FROM NEW.subject
        OR OLD.role IS DISTINCT FROM NEW.role
        OR OLD.public_key IS DISTINCT FROM NEW.public_key
        OR OLD.challenge_id IS DISTINCT FROM NEW.challenge_id
        OR OLD.proof_signature IS DISTINCT FROM NEW.proof_signature
        OR OLD.created_at IS DISTINCT FROM NEW.created_at
        OR (OLD.status = 'revoked' AND NEW.status <> 'revoked')
        OR (OLD.status = 'revoked' AND NEW.status = 'revoked'
            AND OLD.revoked_at IS DISTINCT FROM NEW.revoked_at)
        OR (OLD.status = 'active' AND NEW.status = 'active')
    THEN
        RAISE EXCEPTION 'adjustment authority proof fields are immutable';
    END IF;
    IF OLD.status = 'active' AND NEW.status = 'revoked' AND NEW.revoked_at IS NULL THEN
        RAISE EXCEPTION 'revoked adjustment authority requires revoked_at';
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS adjustment_authority_binding_mutation_guard
    ON settlement_adjustment_authority_bindings;
CREATE TRIGGER adjustment_authority_binding_mutation_guard
    BEFORE UPDATE ON settlement_adjustment_authority_bindings
    FOR EACH ROW
    EXECUTE FUNCTION prevent_adjustment_authority_binding_mutation();

CREATE SEQUENCE IF NOT EXISTS settlement_adjustment_chain_nonce_seq
    AS bigint START WITH 1 INCREMENT BY 1 MINVALUE 1;

CREATE TABLE IF NOT EXISTS settlement_adjustment_evidence_intents (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    adjustment_id uuid NOT NULL,
    merchant_id uuid NOT NULL,
    nonce numeric(20, 0) NOT NULL DEFAULT nextval('settlement_adjustment_chain_nonce_seq'),
    proposer bytea NOT NULL,
    approver bytea NOT NULL,
    original_receipt bytea NOT NULL,
    merchant bytea NOT NULL,
    mint bytea NOT NULL,
    token_program text NOT NULL,
    registry_address bytea NOT NULL,
    registry_slot numeric(20, 0) NOT NULL,
    registry_commitment text NOT NULL,
    registry_account_digest bytea NOT NULL,
    request_digest bytea NOT NULL,
    instruction_digest bytea,
    status text NOT NULL DEFAULT 'prepared_unsubmitted',
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK (uuid_extract_version(id) = 7),
    CHECK (uuid_extract_version(adjustment_id) = 7),
    CHECK (nonce > 0 AND nonce <= 18446744073709551615),
    CHECK (octet_length(proposer) = 32 AND proposer <> repeat(E'\\000', 32)::bytea),
    CHECK (octet_length(approver) = 32 AND approver <> repeat(E'\\000', 32)::bytea),
    CHECK (proposer <> approver),
    CHECK (octet_length(original_receipt) = 32 AND original_receipt <> repeat(E'\\000', 32)::bytea),
    CHECK (octet_length(merchant) = 32 AND merchant <> repeat(E'\\000', 32)::bytea),
    CHECK (octet_length(mint) = 32 AND mint <> repeat(E'\\000', 32)::bytea),
    CHECK (length(token_program) BETWEEN 32 AND 44),
    CHECK (octet_length(registry_address) = 32
        AND registry_address <> repeat(E'\\000', 32)::bytea),
    CHECK (registry_slot > 0 AND registry_slot <= 18446744073709551615),
    CHECK (registry_commitment = 'finalized'),
    CHECK (octet_length(registry_account_digest) = 32
        AND registry_account_digest <> repeat(E'\\000', 32)::bytea),
    CHECK (octet_length(request_digest) = 32 AND request_digest <> repeat(E'\\000', 32)::bytea),
    CHECK (instruction_digest IS NULL OR octet_length(instruction_digest) = 32),
    CHECK (status = 'prepared_unsubmitted'),
    UNIQUE (adjustment_id, merchant_id),
    UNIQUE (original_receipt, nonce),
    FOREIGN KEY (adjustment_id, merchant_id)
        REFERENCES settlement_adjustments(id, merchant_id)
);

CREATE INDEX IF NOT EXISTS settlement_adjustment_evidence_intent_lookup_idx
    ON settlement_adjustment_evidence_intents (merchant_id, id, adjustment_id);

CREATE OR REPLACE FUNCTION prevent_adjustment_evidence_intent_mutation()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF OLD.adjustment_id IS DISTINCT FROM NEW.adjustment_id
        OR OLD.merchant_id IS DISTINCT FROM NEW.merchant_id
        OR OLD.nonce IS DISTINCT FROM NEW.nonce
        OR OLD.proposer IS DISTINCT FROM NEW.proposer
        OR OLD.approver IS DISTINCT FROM NEW.approver
        OR OLD.original_receipt IS DISTINCT FROM NEW.original_receipt
        OR OLD.merchant IS DISTINCT FROM NEW.merchant
        OR OLD.mint IS DISTINCT FROM NEW.mint
        OR OLD.token_program IS DISTINCT FROM NEW.token_program
        OR OLD.registry_address IS DISTINCT FROM NEW.registry_address
        OR OLD.registry_slot IS DISTINCT FROM NEW.registry_slot
        OR OLD.registry_commitment IS DISTINCT FROM NEW.registry_commitment
        OR OLD.registry_account_digest IS DISTINCT FROM NEW.registry_account_digest
        OR OLD.request_digest IS DISTINCT FROM NEW.request_digest
        OR OLD.status IS DISTINCT FROM NEW.status
        OR OLD.created_at IS DISTINCT FROM NEW.created_at
        OR (OLD.instruction_digest IS NOT NULL
            AND OLD.instruction_digest IS DISTINCT FROM NEW.instruction_digest)
    THEN
        RAISE EXCEPTION 'adjustment evidence intent fields are immutable';
    END IF;
    NEW.updated_at = now();
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS adjustment_evidence_intent_mutation_guard
    ON settlement_adjustment_evidence_intents;
CREATE TRIGGER adjustment_evidence_intent_mutation_guard
    BEFORE UPDATE ON settlement_adjustment_evidence_intents
    FOR EACH ROW
    EXECUTE FUNCTION prevent_adjustment_evidence_intent_mutation();

COMMIT;
