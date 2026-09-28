#!/bin/sh

set -eu

project_name="solana-tax-protocol-adjustment-authority"
export POSTGRES_PORT="25436"

cleanup() {
    docker compose -p "$project_name" down --volumes --remove-orphans >/dev/null 2>&1 || true
}
trap cleanup EXIT INT TERM

docker compose -p "$project_name" up --build --detach postgres
ready="false"
for _attempt in $(seq 1 60); do
    if docker compose -p "$project_name" exec -T postgres \
        pg_isready -U tax_protocol -d tax_protocol >/dev/null 2>&1; then
        ready="true"
        break
    fi
    sleep 1
done
if [ "$ready" != "true" ]; then
    echo "PostgreSQL did not become ready" >&2
    exit 1
fi

docker compose -p "$project_name" run --build --rm migrator >/dev/null

docker compose -p "$project_name" exec -T postgres psql \
    -v ON_ERROR_STOP=1 -U tax_protocol -d tax_protocol <<'SQL'
DO $setup$
DECLARE
    tenant_id uuid := '019a1111-1111-7111-8111-111111111111';
    adjustment_id uuid := uuidv7();
    source_calculation_id uuid := uuidv7();
    proposer_challenge_id uuid := uuidv7();
    approver_challenge_id uuid := uuidv7();
BEGIN
    INSERT INTO settlement_adjustments
        (id, merchant_id, idempotency_key, request_digest, adjustment_digest,
         original_settlement_id, asset_id, kind, gross_delta, tax_delta,
         net_delta, reason_code, evidence_digest, source_calculation_id,
         status, proposer_subject)
    VALUES
        (adjustment_id, tenant_id, 'authority-drill', decode(repeat('01', 32), 'hex'),
         decode(repeat('02', 32), 'hex'), uuidv7(), decode(repeat('03', 32), 'hex'),
         'correction', 10, 4, 6, 'drill', decode(repeat('04', 32), 'hex'),
         source_calculation_id, 'approved', 'proposer-subject');

    INSERT INTO settlement_adjustment_authority_challenges
        (id, merchant_id, subject, role, message_digest, expires_at)
    VALUES
        (proposer_challenge_id, tenant_id, 'proposer-subject', 'proposer',
         decode(repeat('05', 32), 'hex'), now() + interval '5 minutes'),
        (approver_challenge_id, tenant_id, 'approver-subject', 'approver',
         decode(repeat('06', 32), 'hex'), now() + interval '5 minutes');

    INSERT INTO settlement_adjustment_authority_bindings
        (merchant_id, subject, role, public_key, challenge_id, proof_signature)
    VALUES
        (tenant_id, 'proposer-subject', 'proposer', decode(repeat('11', 32), 'hex'),
         proposer_challenge_id, decode(repeat('21', 64), 'hex')),
        (tenant_id, 'approver-subject', 'approver', decode(repeat('12', 32), 'hex'),
         approver_challenge_id, decode(repeat('22', 64), 'hex'));

    INSERT INTO settlement_adjustment_evidence_intents
        (adjustment_id, merchant_id, proposer, approver, original_receipt,
         merchant, mint, token_program, registry_address, registry_slot,
         registry_commitment, registry_account_digest, request_digest)
    VALUES
        (adjustment_id, tenant_id, decode(repeat('11', 32), 'hex'),
         decode(repeat('12', 32), 'hex'), decode(repeat('13', 32), 'hex'),
         decode(repeat('14', 32), 'hex'), decode(repeat('15', 32), 'hex'),
         '11111111111111111111111111111111', decode(repeat('17', 32), 'hex'),
         42, 'finalized', decode(repeat('18', 32), 'hex'),
         decode(repeat('16', 32), 'hex'));
END
$setup$;

DO $duplicate_key$
DECLARE
    tenant_id uuid := '019a1111-1111-7111-8111-111111111111';
    challenge_id uuid := uuidv7();
BEGIN
    INSERT INTO settlement_adjustment_authority_challenges
        (id, merchant_id, subject, role, message_digest, expires_at)
    VALUES (challenge_id, tenant_id, 'other-subject', 'proposer',
            decode(repeat('31', 32), 'hex'), now() + interval '5 minutes');
    BEGIN
        INSERT INTO settlement_adjustment_authority_bindings
            (merchant_id, subject, role, public_key, challenge_id, proof_signature)
        VALUES (tenant_id, 'other-subject', 'proposer', decode(repeat('11', 32), 'hex'),
                challenge_id, decode(repeat('41', 64), 'hex'));
        RAISE EXCEPTION 'active public-key uniqueness was not enforced';
    EXCEPTION WHEN unique_violation THEN
        NULL;
    END;
END
$duplicate_key$;

DO $immutable_binding$
DECLARE
    binding_id uuid;
    tenant_id uuid := '019a1111-1111-7111-8111-111111111111';
BEGIN
    SELECT b.id INTO binding_id
    FROM settlement_adjustment_authority_bindings AS b
    WHERE b.merchant_id = tenant_id AND b.subject = 'proposer-subject';
    BEGIN
        UPDATE settlement_adjustment_authority_bindings
        SET proof_signature = decode(repeat('42', 64), 'hex')
        WHERE id = binding_id;
        RAISE EXCEPTION 'binding proof evidence was mutable';
    EXCEPTION WHEN raise_exception THEN
        IF SQLERRM <> 'adjustment authority proof fields are immutable' THEN
            RAISE;
        END IF;
    END;
    UPDATE settlement_adjustment_authority_bindings
    SET status = 'revoked', revoked_at = now()
    WHERE id = binding_id;
    BEGIN
        UPDATE settlement_adjustment_authority_bindings
        SET revoked_at = now() + interval '1 second'
        WHERE id = binding_id;
        RAISE EXCEPTION 'revocation evidence was mutable';
    EXCEPTION WHEN raise_exception THEN
        IF SQLERRM <> 'adjustment authority proof fields are immutable' THEN
            RAISE;
        END IF;
    END;
END
$immutable_binding$;

DO $intent_digest$
DECLARE
    intent_id uuid;
    tenant_id uuid := '019a1111-1111-7111-8111-111111111111';
BEGIN
    SELECT e.id INTO intent_id
    FROM settlement_adjustment_evidence_intents AS e
    WHERE e.merchant_id = tenant_id;
    UPDATE settlement_adjustment_evidence_intents
    SET instruction_digest = decode(repeat('61', 32), 'hex')
    WHERE id = intent_id;
    BEGIN
        UPDATE settlement_adjustment_evidence_intents
        SET registry_account_digest = decode(repeat('63', 32), 'hex')
        WHERE id = intent_id;
        RAISE EXCEPTION 'registry evidence was mutable';
    EXCEPTION WHEN raise_exception THEN
        IF SQLERRM <> 'adjustment evidence intent fields are immutable' THEN
            RAISE;
        END IF;
    END;
    UPDATE settlement_adjustment_evidence_intents
    SET instruction_digest = decode(repeat('61', 32), 'hex')
    WHERE id = intent_id;
    BEGIN
        UPDATE settlement_adjustment_evidence_intents
        SET instruction_digest = decode(repeat('62', 32), 'hex')
        WHERE id = intent_id;
        RAISE EXCEPTION 'instruction digest was mutable';
    EXCEPTION WHEN raise_exception THEN
        IF SQLERRM <> 'adjustment evidence intent fields are immutable' THEN
            RAISE;
        END IF;
    END;
END
$intent_digest$;

DO $assertions$
DECLARE
    tenant_id uuid := '019a1111-1111-7111-8111-111111111111';
    active_count integer;
    revoked_count integer;
    intent_count integer;
    nonce_count integer;
BEGIN
    SELECT count(*) INTO active_count
    FROM settlement_adjustment_authority_bindings AS b
    WHERE b.merchant_id = tenant_id AND b.status = 'active';
    SELECT count(*) INTO revoked_count
    FROM settlement_adjustment_authority_bindings AS b
    WHERE b.merchant_id = tenant_id AND b.status = 'revoked';
    SELECT count(*) INTO intent_count
    FROM settlement_adjustment_evidence_intents AS e
    WHERE e.merchant_id = tenant_id
      AND e.instruction_digest = decode(repeat('61', 32), 'hex');
    SELECT count(DISTINCT e.nonce) INTO nonce_count
    FROM settlement_adjustment_evidence_intents AS e
    WHERE e.merchant_id = tenant_id;
    IF active_count <> 1 OR revoked_count <> 1 OR intent_count <> 1 OR nonce_count <> 1 THEN
        RAISE EXCEPTION 'authority/intent drill assertions failed: active %, revoked %, intent %, nonce %',
            active_count, revoked_count, intent_count, nonce_count;
    END IF;
END
$assertions$;
SQL

echo "adjustment-authority-drill passed: proof fields immutable, active key uniqueness enforced, rotation, registry evidence, and intent digest binding verified"
