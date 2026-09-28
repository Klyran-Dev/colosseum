-- Immutable tenant-owned jurisdiction policies bound to published rule releases.
-- A policy correction is a new published rule release and policy row; existing
-- calculation evidence is never rewritten.

BEGIN;

CREATE TABLE IF NOT EXISTS jurisdiction_policies (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    merchant_id uuid NOT NULL,
    rule_set_id uuid NOT NULL,
    policy_digest bytea NOT NULL,
    policy_document jsonb NOT NULL,
    provenance jsonb NOT NULL,
    status text NOT NULL CHECK (status IN ('draft', 'reviewed', 'approved', 'published', 'retired', 'blocked')),
    effective_from timestamptz NOT NULL,
    effective_until timestamptz,
    author_subject text,
    reviewer_subject text,
    approver_subject text,
    governance_revision bigint NOT NULL DEFAULT 0,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK (uuid_extract_version(id) = 7),
    CHECK (octet_length(policy_digest) = 32),
    CHECK (jsonb_typeof(policy_document) = 'object'),
    CHECK (jsonb_typeof(provenance) = 'object'),
    CHECK (effective_until IS NULL OR effective_until > effective_from),
    CHECK (governance_revision >= 0),
    CONSTRAINT jurisdiction_policy_governance_subjects_bounded CHECK ((author_subject IS NULL OR length(btrim(author_subject)) BETWEEN 1 AND 256)
        AND (reviewer_subject IS NULL OR length(btrim(reviewer_subject)) BETWEEN 1 AND 256)
        AND (approver_subject IS NULL OR length(btrim(approver_subject)) BETWEEN 1 AND 256)),
    CONSTRAINT jurisdiction_policy_governance_author_reviewer_distinct CHECK (author_subject IS NULL OR reviewer_subject IS NULL OR author_subject <> reviewer_subject),
    CONSTRAINT jurisdiction_policy_governance_author_approver_distinct CHECK (author_subject IS NULL OR approver_subject IS NULL OR author_subject <> approver_subject),
    CONSTRAINT jurisdiction_policy_governance_reviewer_approver_distinct CHECK (reviewer_subject IS NULL OR approver_subject IS NULL OR reviewer_subject <> approver_subject),
    UNIQUE (id, merchant_id),
    UNIQUE (merchant_id, rule_set_id),
    FOREIGN KEY (rule_set_id, merchant_id)
        REFERENCES rule_sets (id, merchant_id)
);

-- Expand/contract safety for installations that already created this table
-- before the governance columns were introduced. Existing published rows are
-- intentionally not backfilled with synthetic actors; only new transitions
-- can use the governed lifecycle.
ALTER TABLE jurisdiction_policies
    ADD COLUMN IF NOT EXISTS author_subject text,
    ADD COLUMN IF NOT EXISTS reviewer_subject text,
    ADD COLUMN IF NOT EXISTS approver_subject text,
    ADD COLUMN IF NOT EXISTS governance_revision bigint NOT NULL DEFAULT 0;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conrelid = 'jurisdiction_policies'::regclass
          AND conname = 'jurisdiction_policy_governance_subjects_bounded'
    ) THEN
        ALTER TABLE jurisdiction_policies
            ADD CONSTRAINT jurisdiction_policy_governance_subjects_bounded CHECK (
                (author_subject IS NULL OR length(btrim(author_subject)) BETWEEN 1 AND 256)
                AND (reviewer_subject IS NULL OR length(btrim(reviewer_subject)) BETWEEN 1 AND 256)
                AND (approver_subject IS NULL OR length(btrim(approver_subject)) BETWEEN 1 AND 256)
            );
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conrelid = 'jurisdiction_policies'::regclass
          AND conname = 'jurisdiction_policy_governance_author_reviewer_distinct'
    ) THEN
        ALTER TABLE jurisdiction_policies
            ADD CONSTRAINT jurisdiction_policy_governance_author_reviewer_distinct
            CHECK (author_subject IS NULL OR reviewer_subject IS NULL OR author_subject <> reviewer_subject);
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conrelid = 'jurisdiction_policies'::regclass
          AND conname = 'jurisdiction_policy_governance_author_approver_distinct'
    ) THEN
        ALTER TABLE jurisdiction_policies
            ADD CONSTRAINT jurisdiction_policy_governance_author_approver_distinct
            CHECK (author_subject IS NULL OR approver_subject IS NULL OR author_subject <> approver_subject);
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conrelid = 'jurisdiction_policies'::regclass
          AND conname = 'jurisdiction_policy_governance_reviewer_approver_distinct'
    ) THEN
        ALTER TABLE jurisdiction_policies
            ADD CONSTRAINT jurisdiction_policy_governance_reviewer_approver_distinct
            CHECK (reviewer_subject IS NULL OR approver_subject IS NULL OR reviewer_subject <> approver_subject);
    END IF;
END;
$$;

CREATE INDEX IF NOT EXISTS jurisdiction_policies_lookup_idx
    ON jurisdiction_policies (merchant_id, rule_set_id, status, effective_from DESC);

CREATE TABLE IF NOT EXISTS jurisdiction_policy_events (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    policy_id uuid NOT NULL REFERENCES jurisdiction_policies(id),
    merchant_id uuid NOT NULL,
    actor_subject text NOT NULL,
    action text NOT NULL CHECK (action IN ('create', 'review', 'approve', 'publish', 'retire', 'block')),
    from_status text NOT NULL,
    to_status text NOT NULL,
    evidence jsonb NOT NULL,
    occurred_at timestamptz NOT NULL DEFAULT now(),
    CHECK (uuid_extract_version(id) = 7),
    CHECK (length(btrim(actor_subject)) BETWEEN 1 AND 256),
    CHECK (jsonb_typeof(evidence) = 'object'),
    UNIQUE (policy_id, id),
    FOREIGN KEY (policy_id, merchant_id)
        REFERENCES jurisdiction_policies (id, merchant_id)
);

CREATE INDEX IF NOT EXISTS jurisdiction_policy_events_policy_idx
    ON jurisdiction_policy_events (policy_id, occurred_at ASC, id ASC);

CREATE OR REPLACE FUNCTION prevent_jurisdiction_policy_event_mutation()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    RAISE EXCEPTION 'jurisdiction policy events are append-only';
END;
$$;

DROP TRIGGER IF EXISTS jurisdiction_policy_event_mutation_guard ON jurisdiction_policy_events;
CREATE TRIGGER jurisdiction_policy_event_mutation_guard
    BEFORE UPDATE OR DELETE ON jurisdiction_policy_events
    FOR EACH ROW
    EXECUTE FUNCTION prevent_jurisdiction_policy_event_mutation();

CREATE OR REPLACE FUNCTION validate_jurisdiction_policy_transition()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NEW.status = OLD.status THEN
        IF OLD.status IN ('published', 'retired') AND (
            NEW.merchant_id IS DISTINCT FROM OLD.merchant_id
            OR NEW.rule_set_id IS DISTINCT FROM OLD.rule_set_id
            OR NEW.policy_digest IS DISTINCT FROM OLD.policy_digest
            OR NEW.policy_document IS DISTINCT FROM OLD.policy_document
            OR NEW.provenance IS DISTINCT FROM OLD.provenance
            OR NEW.effective_from IS DISTINCT FROM OLD.effective_from
            OR NEW.effective_until IS DISTINCT FROM OLD.effective_until
            OR NEW.author_subject IS DISTINCT FROM OLD.author_subject
            OR NEW.reviewer_subject IS DISTINCT FROM OLD.reviewer_subject
            OR NEW.approver_subject IS DISTINCT FROM OLD.approver_subject
            OR NEW.governance_revision IS DISTINCT FROM OLD.governance_revision
        ) THEN
            RAISE EXCEPTION 'published and retired jurisdiction policies are immutable';
        END IF;
        RETURN NEW;
    END IF;

    IF NOT (
        (OLD.status = 'draft' AND NEW.status IN ('reviewed', 'blocked'))
        OR (OLD.status = 'reviewed' AND NEW.status IN ('approved', 'blocked'))
        OR (OLD.status = 'approved' AND NEW.status IN ('published', 'blocked'))
        OR (OLD.status = 'published' AND NEW.status = 'retired')
    ) THEN
        RAISE EXCEPTION 'invalid jurisdiction policy transition from % to %', OLD.status, NEW.status;
    END IF;

    IF NEW.status IN ('reviewed', 'approved', 'published', 'retired') AND NEW.author_subject IS NULL THEN
        RAISE EXCEPTION 'jurisdiction policy requires an author before review';
    END IF;
    IF NEW.status IN ('approved', 'published', 'retired') AND NEW.reviewer_subject IS NULL THEN
        RAISE EXCEPTION 'jurisdiction policy requires a reviewer before approval';
    END IF;
    IF NEW.status IN ('published', 'retired') AND NEW.approver_subject IS NULL THEN
        RAISE EXCEPTION 'jurisdiction policy requires an approver before publication';
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS jurisdiction_policy_transition_guard ON jurisdiction_policies;
CREATE TRIGGER jurisdiction_policy_transition_guard
    BEFORE UPDATE ON jurisdiction_policies
    FOR EACH ROW
    EXECUTE FUNCTION validate_jurisdiction_policy_transition();

CREATE OR REPLACE FUNCTION prevent_published_jurisdiction_policy_mutation()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF OLD.status IN ('published', 'retired') AND (
        NEW.merchant_id IS DISTINCT FROM OLD.merchant_id
        OR NEW.rule_set_id IS DISTINCT FROM OLD.rule_set_id
        OR NEW.policy_digest IS DISTINCT FROM OLD.policy_digest
        OR NEW.policy_document IS DISTINCT FROM OLD.policy_document
        OR NEW.provenance IS DISTINCT FROM OLD.provenance
        OR NEW.effective_from IS DISTINCT FROM OLD.effective_from
        OR NEW.effective_until IS DISTINCT FROM OLD.effective_until
    ) THEN
        RAISE EXCEPTION 'published and retired jurisdiction policies are immutable';
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS jurisdiction_policy_mutation_guard ON jurisdiction_policies;
CREATE TRIGGER jurisdiction_policy_mutation_guard
    BEFORE UPDATE ON jurisdiction_policies
    FOR EACH ROW
    EXECUTE FUNCTION prevent_published_jurisdiction_policy_mutation();

COMMIT;
