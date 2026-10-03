-- Durable Revenue Streaming projection.
-- Finalized settlement evidence remains the source of accrual truth and
-- finalized Solana evidence remains the source of token-movement truth.

BEGIN;

CREATE TABLE IF NOT EXISTS revenue_stream_plans (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    merchant_id uuid NOT NULL,
    settlement_route_id uuid NOT NULL,
    idempotency_key text NOT NULL,
    request_digest bytea NOT NULL,
    asset bytea NOT NULL,
    recipient bytea NOT NULL,
    plan_digest bytea NOT NULL,
    policy_digest bytea NOT NULL,
    start_slot numeric(20, 0) NOT NULL,
    end_slot numeric(20, 0),
    max_total_atomic numeric(20, 0) NOT NULL,
    max_claim_atomic numeric(20, 0) NOT NULL,
    status text NOT NULL DEFAULT 'proposed',
    proposer_subject text NOT NULL,
    approver_subject text,
    accrued_total_atomic numeric(20, 0) NOT NULL DEFAULT 0,
    released_total_atomic numeric(20, 0) NOT NULL DEFAULT 0,
    next_nonce numeric(20, 0) NOT NULL DEFAULT 1,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK (uuid_extract_version(id) = 7),
    CHECK (uuid_extract_version(settlement_route_id) = 7),
    CHECK (length(btrim(idempotency_key)) BETWEEN 1 AND 200),
    CHECK (octet_length(request_digest) = 32),
    CHECK (octet_length(asset) = 32 AND asset <> repeat(E'\\000', 32)::bytea),
    CHECK (octet_length(recipient) = 32 AND recipient <> repeat(E'\\000', 32)::bytea),
    CHECK (octet_length(plan_digest) = 32 AND plan_digest <> repeat(E'\\000', 32)::bytea),
    CHECK (octet_length(policy_digest) = 32 AND policy_digest <> repeat(E'\\000', 32)::bytea),
    CHECK (start_slot > 0 AND start_slot <= 18446744073709551615),
    CHECK (end_slot IS NULL OR (end_slot >= start_slot AND end_slot <= 18446744073709551615)),
    CHECK (max_total_atomic > 0 AND max_total_atomic <= 18446744073709551615),
    CHECK (max_claim_atomic > 0 AND max_claim_atomic <= max_total_atomic),
    CHECK (status IN ('proposed', 'approved', 'active', 'paused', 'cancelled', 'completed')),
    CHECK (length(btrim(proposer_subject)) BETWEEN 1 AND 256),
    CHECK (approver_subject IS NULL OR (length(btrim(approver_subject)) BETWEEN 1 AND 256 AND approver_subject <> proposer_subject)),
    CHECK (accrued_total_atomic >= 0 AND accrued_total_atomic <= max_total_atomic),
    CHECK (released_total_atomic >= 0 AND released_total_atomic <= accrued_total_atomic),
    CHECK (next_nonce > 0 AND next_nonce <= 18446744073709551615),
    UNIQUE (merchant_id, idempotency_key),
    UNIQUE (merchant_id, plan_digest),
    UNIQUE (id, merchant_id),
    FOREIGN KEY (settlement_route_id, merchant_id)
        REFERENCES settlement_routes(id, merchant_id)
);

CREATE INDEX IF NOT EXISTS revenue_stream_plans_status_idx
    ON revenue_stream_plans (merchant_id, status, updated_at DESC, id DESC);

CREATE TABLE IF NOT EXISTS revenue_stream_plan_events (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    plan_id uuid NOT NULL,
    merchant_id uuid NOT NULL,
    actor_subject text NOT NULL,
    action text NOT NULL CHECK (action IN ('propose', 'approve', 'activate', 'pause', 'resume', 'cancel', 'complete')),
    from_status text NOT NULL,
    to_status text NOT NULL,
    evidence jsonb NOT NULL DEFAULT '{}'::jsonb,
    occurred_at timestamptz NOT NULL DEFAULT now(),
    CHECK (uuid_extract_version(id) = 7),
    CHECK (length(btrim(actor_subject)) BETWEEN 1 AND 256),
    CHECK (jsonb_typeof(evidence) = 'object'),
    FOREIGN KEY (plan_id, merchant_id) REFERENCES revenue_stream_plans(id, merchant_id),
    UNIQUE (plan_id, id)
);

CREATE TABLE IF NOT EXISTS revenue_stream_accruals (
    stream_plan_id uuid NOT NULL,
    merchant_id uuid NOT NULL,
    source_signature bytea NOT NULL,
    finalized_slot numeric(20, 0) NOT NULL,
    receipt_digest bytea NOT NULL,
    plan_digest bytea NOT NULL,
    policy_digest bytea NOT NULL,
    asset bytea NOT NULL,
    recipient bytea NOT NULL,
    gross_atomic numeric(20, 0) NOT NULL,
    tax_atomic numeric(20, 0) NOT NULL,
    net_atomic numeric(20, 0) NOT NULL,
    recipient_atomic numeric(20, 0) NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (stream_plan_id, source_signature),
    CHECK (octet_length(source_signature) = 64),
    CHECK (finalized_slot > 0 AND finalized_slot <= 18446744073709551615),
    CHECK (octet_length(receipt_digest) = 32 AND receipt_digest <> repeat(E'\\000', 32)::bytea),
    CHECK (octet_length(plan_digest) = 32 AND plan_digest <> repeat(E'\\000', 32)::bytea),
    CHECK (octet_length(policy_digest) = 32 AND policy_digest <> repeat(E'\\000', 32)::bytea),
    CHECK (octet_length(asset) = 32 AND asset <> repeat(E'\\000', 32)::bytea),
    CHECK (octet_length(recipient) = 32 AND recipient <> repeat(E'\\000', 32)::bytea),
    CHECK (gross_atomic > 0 AND gross_atomic <= 18446744073709551615),
    CHECK (tax_atomic <= gross_atomic AND net_atomic <= gross_atomic),
    CHECK (recipient_atomic > 0 AND recipient_atomic <= net_atomic),
    CHECK (gross_atomic = tax_atomic + net_atomic),
    FOREIGN KEY (stream_plan_id, merchant_id) REFERENCES revenue_stream_plans(id, merchant_id),
    UNIQUE (stream_plan_id, receipt_digest)
);

CREATE TABLE IF NOT EXISTS revenue_stream_corrections (
    stream_plan_id uuid NOT NULL,
    merchant_id uuid NOT NULL,
    correction_signature bytea NOT NULL,
    source_signature bytea NOT NULL,
    finalized_slot numeric(20, 0) NOT NULL,
    plan_digest bytea NOT NULL,
    decrease boolean NOT NULL,
    amount_atomic numeric(20, 0) NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (stream_plan_id, correction_signature),
    CHECK (octet_length(correction_signature) = 64),
    CHECK (octet_length(source_signature) = 64),
    CHECK (finalized_slot > 0 AND finalized_slot <= 18446744073709551615),
    CHECK (octet_length(plan_digest) = 32 AND plan_digest <> repeat(E'\\000', 32)::bytea),
    CHECK (amount_atomic > 0 AND amount_atomic <= 18446744073709551615),
    FOREIGN KEY (stream_plan_id, merchant_id) REFERENCES revenue_stream_plans(id, merchant_id)
);

CREATE TABLE IF NOT EXISTS revenue_stream_claims (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    stream_plan_id uuid NOT NULL,
    merchant_id uuid NOT NULL,
    idempotency_key text NOT NULL,
    amount_atomic numeric(20, 0) NOT NULL,
    nonce numeric(20, 0) NOT NULL,
    accrual_total_atomic numeric(20, 0) NOT NULL,
    plan_digest bytea NOT NULL,
    asset bytea NOT NULL,
    recipient bytea NOT NULL,
    claim_digest bytea NOT NULL,
    status text NOT NULL DEFAULT 'pending',
    release_signature bytea,
    finalized_slot numeric(20, 0),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK (uuid_extract_version(id) = 7),
    CHECK (length(btrim(idempotency_key)) BETWEEN 1 AND 200),
    CHECK (amount_atomic > 0 AND amount_atomic <= 18446744073709551615),
    CHECK (nonce > 0 AND nonce <= 18446744073709551615),
    CHECK (accrual_total_atomic > 0 AND accrual_total_atomic <= 18446744073709551615),
    CHECK (octet_length(plan_digest) = 32 AND plan_digest <> repeat(E'\\000', 32)::bytea),
    CHECK (octet_length(asset) = 32 AND asset <> repeat(E'\\000', 32)::bytea),
    CHECK (octet_length(recipient) = 32 AND recipient <> repeat(E'\\000', 32)::bytea),
    CHECK (octet_length(claim_digest) = 32 AND claim_digest <> repeat(E'\\000', 32)::bytea),
    CHECK (status IN ('pending', 'finalized', 'cancelled')),
    CHECK (release_signature IS NULL OR octet_length(release_signature) = 64),
    CHECK (finalized_slot IS NULL OR (finalized_slot > 0 AND finalized_slot <= 18446744073709551615)),
    CHECK ((status = 'finalized') = (release_signature IS NOT NULL AND finalized_slot IS NOT NULL)),
    FOREIGN KEY (stream_plan_id, merchant_id) REFERENCES revenue_stream_plans(id, merchant_id),
    UNIQUE (merchant_id, stream_plan_id, idempotency_key),
    UNIQUE (stream_plan_id, nonce),
    UNIQUE (stream_plan_id, claim_digest),
    UNIQUE (release_signature)
);

CREATE INDEX IF NOT EXISTS revenue_stream_claims_pending_idx
    ON revenue_stream_claims (stream_plan_id, status, created_at, id)
    WHERE status = 'pending';

CREATE TABLE IF NOT EXISTS revenue_stream_claim_events (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    claim_id uuid NOT NULL REFERENCES revenue_stream_claims(id),
    stream_plan_id uuid NOT NULL,
    merchant_id uuid NOT NULL,
    actor_subject text NOT NULL,
    action text NOT NULL CHECK (action IN ('prepared', 'finalized', 'cancelled', 'unknown')),
    evidence jsonb NOT NULL DEFAULT '{}'::jsonb,
    occurred_at timestamptz NOT NULL DEFAULT now(),
    CHECK (uuid_extract_version(id) = 7),
    CHECK (length(btrim(actor_subject)) BETWEEN 1 AND 256),
    CHECK (jsonb_typeof(evidence) = 'object'),
    FOREIGN KEY (stream_plan_id, merchant_id) REFERENCES revenue_stream_plans(id, merchant_id),
    UNIQUE (claim_id, id)
);

CREATE OR REPLACE FUNCTION prevent_revenue_stream_event_mutation()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    RAISE EXCEPTION 'revenue stream evidence events are append-only';
END;
$$;

DROP TRIGGER IF EXISTS revenue_stream_plan_event_mutation_guard ON revenue_stream_plan_events;
CREATE TRIGGER revenue_stream_plan_event_mutation_guard
    BEFORE UPDATE OR DELETE ON revenue_stream_plan_events
    FOR EACH ROW EXECUTE FUNCTION prevent_revenue_stream_event_mutation();

DROP TRIGGER IF EXISTS revenue_stream_claim_event_mutation_guard ON revenue_stream_claim_events;
CREATE TRIGGER revenue_stream_claim_event_mutation_guard
    BEFORE UPDATE OR DELETE ON revenue_stream_claim_events
    FOR EACH ROW EXECUTE FUNCTION prevent_revenue_stream_event_mutation();

COMMIT;
