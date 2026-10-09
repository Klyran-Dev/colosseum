-- ADR-0409 -- Immutable, tenant-scoped applicability contexts and governance
-- evidence. Treatment and evidence remain server-owned; browser requests may
-- reference a published context but cannot author these documents.

BEGIN;

CREATE TABLE IF NOT EXISTS tax_applicability_contexts (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    merchant_id uuid NOT NULL,
    policy_id text NOT NULL,
    policy_version integer NOT NULL,
    policy_digest bytea NOT NULL,
    facts_digest bytea NOT NULL,
    decision_digest bytea NOT NULL,
    jurisdiction_resolution_digest bytea NOT NULL,
    evidence_digest bytea NOT NULL,
    attestation_digest bytea NOT NULL,
    facts_document jsonb NOT NULL,
    decision_document jsonb NOT NULL,
    attestation_document jsonb NOT NULL,
    status text NOT NULL CHECK (status IN ('draft', 'review', 'approved', 'published', 'rejected', 'retired')),
    effective_from timestamptz NOT NULL,
    effective_until timestamptz,
    evidence_expires_at timestamptz NOT NULL,
    supersedes_id uuid,
    provenance jsonb NOT NULL,
    author_subject text,
    reviewer_subject text,
    approver_subject text,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK (uuid_extract_version(id) = 7),
    CHECK (policy_version > 0),
    CHECK (length(policy_digest) = 32),
    CHECK (length(facts_digest) = 32),
    CHECK (length(decision_digest) = 32),
    CHECK (length(jurisdiction_resolution_digest) = 32),
    CHECK (length(evidence_digest) = 32),
    CHECK (length(attestation_digest) = 32),
    CHECK (btrim(policy_id) <> '' AND octet_length(policy_id) <= 128),
    CHECK (effective_until IS NULL OR effective_until > effective_from),
    CHECK (evidence_expires_at > effective_from),
    CHECK (author_subject IS NULL OR btrim(author_subject) <> ''),
    CHECK (reviewer_subject IS NULL OR btrim(reviewer_subject) <> ''),
    CHECK (approver_subject IS NULL OR btrim(approver_subject) <> ''),
    CHECK (author_subject IS NULL OR reviewer_subject IS NULL OR author_subject <> reviewer_subject),
    CHECK (author_subject IS NULL OR approver_subject IS NULL OR author_subject <> approver_subject),
    CHECK (reviewer_subject IS NULL OR approver_subject IS NULL OR reviewer_subject <> approver_subject),
    UNIQUE (id, merchant_id),
    UNIQUE (merchant_id, decision_digest)
);

CREATE INDEX IF NOT EXISTS tax_applicability_contexts_lookup_idx
    ON tax_applicability_contexts (merchant_id, status, effective_from DESC, id DESC);

CREATE INDEX IF NOT EXISTS tax_applicability_contexts_expiry_idx
    ON tax_applicability_contexts (merchant_id, evidence_expires_at, id);

CREATE TABLE IF NOT EXISTS tax_applicability_context_events (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    context_id uuid NOT NULL,
    merchant_id uuid NOT NULL,
    event_type text NOT NULL CHECK (event_type IN ('created', 'submitted', 'approved', 'published', 'rejected', 'retired', 'superseded')),
    actor_subject text NOT NULL,
    evidence jsonb NOT NULL,
    occurred_at timestamptz NOT NULL DEFAULT now(),
    CHECK (uuid_extract_version(id) = 7),
    CHECK (btrim(actor_subject) <> ''),
    UNIQUE (id, merchant_id),
    FOREIGN KEY (context_id, merchant_id)
        REFERENCES tax_applicability_contexts (id, merchant_id)
);

CREATE INDEX IF NOT EXISTS tax_applicability_context_events_context_idx
    ON tax_applicability_context_events (merchant_id, context_id, occurred_at ASC, id ASC);

CREATE OR REPLACE FUNCTION prevent_tax_applicability_context_event_mutation()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    RAISE EXCEPTION 'tax applicability context events are append-only';
END;
$$;

DROP TRIGGER IF EXISTS tax_applicability_context_event_mutation_guard
    ON tax_applicability_context_events;
CREATE TRIGGER tax_applicability_context_event_mutation_guard
    BEFORE UPDATE OR DELETE ON tax_applicability_context_events
    FOR EACH ROW
    EXECUTE FUNCTION prevent_tax_applicability_context_event_mutation();

CREATE OR REPLACE FUNCTION validate_tax_applicability_context_transition()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF OLD.status IN ('published', 'retired') THEN
        IF (OLD.status = 'retired' AND NEW.status <> 'retired')
            OR (OLD.status = 'published' AND NEW.status NOT IN ('published', 'retired'))
            OR NEW.merchant_id IS DISTINCT FROM OLD.merchant_id
            OR NEW.policy_id IS DISTINCT FROM OLD.policy_id
            OR NEW.policy_version IS DISTINCT FROM OLD.policy_version
            OR NEW.policy_digest IS DISTINCT FROM OLD.policy_digest
            OR NEW.facts_digest IS DISTINCT FROM OLD.facts_digest
            OR NEW.decision_digest IS DISTINCT FROM OLD.decision_digest
            OR NEW.jurisdiction_resolution_digest IS DISTINCT FROM OLD.jurisdiction_resolution_digest
            OR NEW.evidence_digest IS DISTINCT FROM OLD.evidence_digest
            OR NEW.attestation_digest IS DISTINCT FROM OLD.attestation_digest
            OR NEW.facts_document IS DISTINCT FROM OLD.facts_document
            OR NEW.decision_document IS DISTINCT FROM OLD.decision_document
            OR NEW.attestation_document IS DISTINCT FROM OLD.attestation_document
            OR NEW.effective_from IS DISTINCT FROM OLD.effective_from
            OR NEW.effective_until IS DISTINCT FROM OLD.effective_until
            OR NEW.evidence_expires_at IS DISTINCT FROM OLD.evidence_expires_at
            OR NEW.supersedes_id IS DISTINCT FROM OLD.supersedes_id
            OR NEW.provenance IS DISTINCT FROM OLD.provenance
        THEN
            RAISE EXCEPTION 'published or retired applicability contexts are immutable';
        END IF;
    END IF;

    IF OLD.status = 'retired' AND NEW.status <> 'retired' THEN
        RAISE EXCEPTION 'retired applicability contexts cannot be reactivated';
    END IF;

    IF NEW.status = 'published' THEN
        IF OLD.status <> 'approved'
            OR NEW.author_subject IS NULL
            OR NEW.reviewer_subject IS NULL
            OR NEW.approver_subject IS NULL
        THEN
            RAISE EXCEPTION 'only approved contexts with four-eyes subjects may be published';
        END IF;
        IF NEW.effective_from >= NEW.evidence_expires_at THEN
            RAISE EXCEPTION 'published applicability context must have live source evidence';
        END IF;
    END IF;

    NEW.updated_at = now();
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS tax_applicability_context_transition_guard
    ON tax_applicability_contexts;
CREATE TRIGGER tax_applicability_context_transition_guard
    BEFORE UPDATE ON tax_applicability_contexts
    FOR EACH ROW
    EXECUTE FUNCTION validate_tax_applicability_context_transition();

ALTER TABLE tax_calculations
    ADD COLUMN IF NOT EXISTS applicability_context_id uuid,
    ADD COLUMN IF NOT EXISTS applicability_attestation_digest bytea,
    ADD CONSTRAINT tax_calculations_applicability_digest_shape
        CHECK (applicability_attestation_digest IS NULL OR length(applicability_attestation_digest) = 32);

ALTER TABLE tax_calculations
    ADD CONSTRAINT tax_calculations_applicability_context_fk
        FOREIGN KEY (applicability_context_id, merchant_id)
        REFERENCES tax_applicability_contexts (id, merchant_id);

CREATE INDEX IF NOT EXISTS tax_calculations_applicability_context_idx
    ON tax_calculations (merchant_id, applicability_context_id, id);

ALTER TABLE tax_document_calculations
    ADD COLUMN IF NOT EXISTS applicability_context_id uuid,
    ADD COLUMN IF NOT EXISTS applicability_attestation_digest bytea,
    ADD CONSTRAINT tax_document_calculations_applicability_digest_shape
        CHECK (applicability_attestation_digest IS NULL OR length(applicability_attestation_digest) = 32);

ALTER TABLE tax_document_calculations
    ADD CONSTRAINT tax_document_calculations_applicability_context_fk
        FOREIGN KEY (applicability_context_id, merchant_id)
        REFERENCES tax_applicability_contexts (id, merchant_id);

CREATE INDEX IF NOT EXISTS tax_document_calculations_applicability_context_idx
    ON tax_document_calculations (merchant_id, applicability_context_id, id);

COMMIT;
