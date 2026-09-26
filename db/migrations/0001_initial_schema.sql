-- PostgreSQL 18.x baseline.
-- Financial evidence is append-only; corrections create new records.

CREATE TABLE IF NOT EXISTS rule_sets (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    external_id text NOT NULL,
    version integer NOT NULL CHECK (version > 0),
    jurisdiction text NOT NULL,
    regime text NOT NULL,
    status text NOT NULL CHECK (status IN ('draft', 'simulated', 'reviewed', 'approved', 'published', 'retired', 'blocked')),
    rule_digest bytea NOT NULL,
    rule_document jsonb NOT NULL,
    provenance jsonb NOT NULL,
    effective_from timestamptz NOT NULL,
    effective_until timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK (uuid_extract_version(id) = 7),
    CHECK (effective_until IS NULL OR effective_until > effective_from),
    UNIQUE (external_id, version)
);

CREATE INDEX IF NOT EXISTS rule_sets_lookup_idx
    ON rule_sets (jurisdiction, regime, status, effective_from DESC);

CREATE TABLE IF NOT EXISTS tax_calculations (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    correlation_id uuid NOT NULL DEFAULT uuidv7(),
    rule_set_id uuid NOT NULL REFERENCES rule_sets(id),
    facts_digest bytea NOT NULL,
    engine_version text NOT NULL,
    currency char(3) NOT NULL,
    jurisdiction text NOT NULL,
    total_tax_atomic numeric(20, 0) NOT NULL CHECK (total_tax_atomic >= 0),
    total_net_atomic numeric(20, 0) NOT NULL CHECK (total_net_atomic >= 0),
    status text NOT NULL CHECK (status IN ('simulated_unattested', 'calculated_unattested', 'attested', 'expired', 'superseded')),
    result_document jsonb NOT NULL,
    evidence_refs jsonb NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    expires_at timestamptz,
    CHECK (uuid_extract_version(id) = 7),
    CHECK (uuid_extract_version(correlation_id) = 7)
);

CREATE INDEX IF NOT EXISTS tax_calculations_created_idx
    ON tax_calculations (created_at DESC, id DESC);

CREATE INDEX IF NOT EXISTS tax_calculations_correlation_idx
    ON tax_calculations (correlation_id);

CREATE TABLE IF NOT EXISTS idempotency_records (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    merchant_id uuid NOT NULL,
    idempotency_key text NOT NULL,
    request_digest bytea NOT NULL,
    response_status integer,
    response_document jsonb,
    created_at timestamptz NOT NULL DEFAULT now(),
    completed_at timestamptz,
    CHECK (uuid_extract_version(id) = 7),
    UNIQUE (merchant_id, idempotency_key)
);

CREATE TABLE IF NOT EXISTS outbox_events (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    aggregate_type text NOT NULL,
    aggregate_id uuid NOT NULL,
    event_type text NOT NULL,
    payload jsonb NOT NULL,
    occurred_at timestamptz NOT NULL DEFAULT now(),
    published_at timestamptz,
    attempt_count integer NOT NULL DEFAULT 0 CHECK (attempt_count >= 0),
    CHECK (uuid_extract_version(id) = 7)
);

CREATE INDEX IF NOT EXISTS outbox_unpublished_idx
    ON outbox_events (occurred_at, id)
    WHERE published_at IS NULL;

CREATE TABLE IF NOT EXISTS audit_events (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    actor_id uuid,
    action text NOT NULL,
    target_type text NOT NULL,
    target_id uuid,
    payload jsonb NOT NULL,
    occurred_at timestamptz NOT NULL DEFAULT now(),
    CHECK (uuid_extract_version(id) = 7)
);

CREATE INDEX IF NOT EXISTS audit_events_target_idx
    ON audit_events (target_type, target_id, occurred_at DESC);
