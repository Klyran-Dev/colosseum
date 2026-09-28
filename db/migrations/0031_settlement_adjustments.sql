-- Tenant-scoped immutable financial adjustment proposals and append-only
-- governance evidence. This table does not authorize token movement.

BEGIN;

CREATE TABLE IF NOT EXISTS settlement_adjustments (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    merchant_id uuid NOT NULL,
    idempotency_key text NOT NULL,
    request_digest bytea NOT NULL,
    adjustment_digest bytea NOT NULL,
    original_settlement_id uuid NOT NULL,
    asset_id bytea NOT NULL,
    kind text NOT NULL,
    gross_delta numeric(20, 0) NOT NULL,
    tax_delta numeric(20, 0) NOT NULL,
    net_delta numeric(20, 0) NOT NULL,
    reason_code text NOT NULL,
    evidence_digest bytea NOT NULL,
    source_calculation_id uuid NOT NULL,
    replacement_calculation_id uuid,
    status text NOT NULL DEFAULT 'proposed',
    proposer_subject text NOT NULL,
    approver_subject text,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK (uuid_extract_version(id) = 7),
    CHECK (length(btrim(idempotency_key)) BETWEEN 1 AND 200),
    CHECK (octet_length(request_digest) = 32),
    CHECK (octet_length(adjustment_digest) = 32),
    CHECK (uuid_extract_version(original_settlement_id) = 7),
    CHECK (octet_length(asset_id) = 32 AND asset_id <> repeat(E'\\000', 32)::bytea),
    CHECK (kind IN ('refund', 'reversal', 'correction', 'chargeback', 'remittance_adjustment')),
    CHECK (gross_delta = tax_delta + net_delta),
    CHECK (gross_delta <> 0 OR tax_delta <> 0 OR net_delta <> 0),
    CHECK (gross_delta BETWEEN -18446744073709551615 AND 18446744073709551615),
    CHECK (tax_delta BETWEEN -18446744073709551615 AND 18446744073709551615),
    CHECK (net_delta BETWEEN -18446744073709551615 AND 18446744073709551615),
    CHECK (octet_length(reason_code) BETWEEN 1 AND 256),
    CHECK (octet_length(evidence_digest) = 32 AND evidence_digest <> repeat(E'\\000', 32)::bytea),
    CHECK (uuid_extract_version(source_calculation_id) = 7),
    CHECK (replacement_calculation_id IS NULL OR uuid_extract_version(replacement_calculation_id) = 7),
    CHECK (status IN ('proposed', 'approved', 'rejected', 'voided')),
    CHECK (length(btrim(proposer_subject)) BETWEEN 1 AND 256),
    CHECK (approver_subject IS NULL OR length(btrim(approver_subject)) BETWEEN 1 AND 256),
    CHECK (approver_subject IS NULL OR approver_subject <> proposer_subject),
    CHECK (kind NOT IN ('refund', 'reversal', 'chargeback') OR
           (gross_delta <= 0 AND tax_delta <= 0 AND net_delta <= 0 AND
            (gross_delta < 0 OR tax_delta < 0 OR net_delta < 0))),
    UNIQUE (merchant_id, idempotency_key),
    UNIQUE (id, merchant_id)
);

CREATE INDEX IF NOT EXISTS settlement_adjustments_status_idx
    ON settlement_adjustments (merchant_id, status, updated_at DESC, id DESC);
CREATE INDEX IF NOT EXISTS settlement_adjustments_original_idx
    ON settlement_adjustments (merchant_id, original_settlement_id, created_at ASC, id ASC);

CREATE TABLE IF NOT EXISTS settlement_adjustment_events (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    adjustment_id uuid NOT NULL,
    merchant_id uuid NOT NULL,
    actor_subject text NOT NULL,
    action text NOT NULL CHECK (action IN ('propose', 'approve', 'reject', 'void')),
    from_status text NOT NULL CHECK (from_status IN ('none', 'proposed', 'approved', 'rejected', 'voided')),
    to_status text NOT NULL CHECK (to_status IN ('proposed', 'approved', 'rejected', 'voided')),
    evidence jsonb NOT NULL DEFAULT '{}'::jsonb,
    occurred_at timestamptz NOT NULL DEFAULT now(),
    CHECK (uuid_extract_version(id) = 7),
    CHECK (length(btrim(actor_subject)) BETWEEN 1 AND 256),
    CHECK (jsonb_typeof(evidence) = 'object'),
    FOREIGN KEY (adjustment_id, merchant_id) REFERENCES settlement_adjustments(id, merchant_id),
    UNIQUE (adjustment_id, id)
);

CREATE INDEX IF NOT EXISTS settlement_adjustment_events_order_idx
    ON settlement_adjustment_events (adjustment_id, occurred_at ASC, id ASC);

CREATE OR REPLACE FUNCTION prevent_settlement_adjustment_event_mutation()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    RAISE EXCEPTION 'settlement adjustment events are append-only';
END;
$$;

DROP TRIGGER IF EXISTS settlement_adjustment_event_mutation_guard
    ON settlement_adjustment_events;
CREATE TRIGGER settlement_adjustment_event_mutation_guard
    BEFORE UPDATE OR DELETE ON settlement_adjustment_events
    FOR EACH ROW
    EXECUTE FUNCTION prevent_settlement_adjustment_event_mutation();

CREATE OR REPLACE FUNCTION validate_settlement_adjustment_transition()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NOT (
        (OLD.status = 'proposed' AND NEW.status IN ('approved', 'rejected'))
        OR (OLD.status = 'approved' AND NEW.status = 'voided')
        OR OLD.status = NEW.status
    ) THEN
        RAISE EXCEPTION 'invalid settlement adjustment transition from % to %', OLD.status, NEW.status;
    END IF;
    IF NEW.status <> OLD.status AND (
        NEW.merchant_id IS DISTINCT FROM OLD.merchant_id
        OR NEW.idempotency_key IS DISTINCT FROM OLD.idempotency_key
        OR NEW.request_digest IS DISTINCT FROM OLD.request_digest
        OR NEW.adjustment_digest IS DISTINCT FROM OLD.adjustment_digest
        OR NEW.original_settlement_id IS DISTINCT FROM OLD.original_settlement_id
        OR NEW.asset_id IS DISTINCT FROM OLD.asset_id
        OR NEW.kind IS DISTINCT FROM OLD.kind
        OR NEW.gross_delta IS DISTINCT FROM OLD.gross_delta
        OR NEW.tax_delta IS DISTINCT FROM OLD.tax_delta
        OR NEW.net_delta IS DISTINCT FROM OLD.net_delta
        OR NEW.reason_code IS DISTINCT FROM OLD.reason_code
        OR NEW.evidence_digest IS DISTINCT FROM OLD.evidence_digest
        OR NEW.source_calculation_id IS DISTINCT FROM OLD.source_calculation_id
        OR NEW.replacement_calculation_id IS DISTINCT FROM OLD.replacement_calculation_id
        OR NEW.proposer_subject IS DISTINCT FROM OLD.proposer_subject
    ) THEN
        RAISE EXCEPTION 'financial settlement adjustment fields are immutable';
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS settlement_adjustment_transition_guard
    ON settlement_adjustments;
CREATE TRIGGER settlement_adjustment_transition_guard
    BEFORE UPDATE ON settlement_adjustments
    FOR EACH ROW
    EXECUTE FUNCTION validate_settlement_adjustment_transition();

COMMIT;
