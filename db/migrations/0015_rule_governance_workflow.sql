-- Tier-1 rule governance workflow. Existing published rows are retained as
-- legacy evidence; new control-plane transitions must populate distinct
-- author, reviewer, and approver subjects before publication.

ALTER TABLE rule_sets
    ADD COLUMN IF NOT EXISTS author_subject text,
    ADD COLUMN IF NOT EXISTS reviewer_subject text,
    ADD COLUMN IF NOT EXISTS approver_subject text,
    ADD COLUMN IF NOT EXISTS governance_revision bigint NOT NULL DEFAULT 0;

ALTER TABLE rule_sets
    DROP CONSTRAINT IF EXISTS rule_governance_subjects_bounded,
    DROP CONSTRAINT IF EXISTS rule_governance_subjects_distinct,
    DROP CONSTRAINT IF EXISTS rule_governance_revision_nonnegative;

ALTER TABLE rule_sets
    ADD CONSTRAINT rule_governance_subjects_bounded CHECK (
        (author_subject IS NULL OR length(btrim(author_subject)) BETWEEN 1 AND 256)
        AND (reviewer_subject IS NULL OR length(btrim(reviewer_subject)) BETWEEN 1 AND 256)
        AND (approver_subject IS NULL OR length(btrim(approver_subject)) BETWEEN 1 AND 256)
    ),
    ADD CONSTRAINT rule_governance_subjects_distinct CHECK (
        author_subject IS NULL OR reviewer_subject IS NULL OR author_subject <> reviewer_subject
    ),
    ADD CONSTRAINT rule_governance_revision_nonnegative CHECK (governance_revision >= 0);

CREATE TABLE IF NOT EXISTS rule_governance_events (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    rule_set_id uuid NOT NULL REFERENCES rule_sets(id),
    actor_subject text NOT NULL,
    action text NOT NULL CHECK (action IN ('simulate', 'review', 'approve', 'publish', 'retire')),
    from_status text NOT NULL,
    to_status text NOT NULL,
    evidence jsonb NOT NULL,
    occurred_at timestamptz NOT NULL DEFAULT now(),
    CHECK (uuid_extract_version(id) = 7),
    CHECK (length(btrim(actor_subject)) BETWEEN 1 AND 256),
    CHECK (jsonb_typeof(evidence) = 'object')
);

CREATE INDEX IF NOT EXISTS rule_governance_events_rule_idx
    ON rule_governance_events (rule_set_id, occurred_at DESC, id DESC);

CREATE OR REPLACE FUNCTION prevent_rule_governance_event_mutation()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    RAISE EXCEPTION 'rule governance events are append-only';
END;
$$;

DROP TRIGGER IF EXISTS rule_governance_event_mutation_guard ON rule_governance_events;
CREATE TRIGGER rule_governance_event_mutation_guard
    BEFORE UPDATE OR DELETE ON rule_governance_events
    FOR EACH ROW
    EXECUTE FUNCTION prevent_rule_governance_event_mutation();

CREATE OR REPLACE FUNCTION validate_rule_governance_transition()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NEW.status = OLD.status THEN
        IF OLD.status IN ('published', 'retired') AND (
            NEW.author_subject IS DISTINCT FROM OLD.author_subject
            OR NEW.reviewer_subject IS DISTINCT FROM OLD.reviewer_subject
            OR NEW.approver_subject IS DISTINCT FROM OLD.approver_subject
            OR NEW.governance_revision IS DISTINCT FROM OLD.governance_revision
        ) THEN
            RAISE EXCEPTION 'published and retired rule governance fields are immutable';
        END IF;
        RETURN NEW;
    END IF;

    IF NOT (
        (OLD.status = 'draft' AND NEW.status IN ('simulated', 'blocked'))
        OR (OLD.status = 'simulated' AND NEW.status IN ('reviewed', 'blocked'))
        OR (OLD.status = 'reviewed' AND NEW.status IN ('approved', 'blocked'))
        OR (OLD.status = 'approved' AND NEW.status IN ('published', 'blocked'))
        OR (OLD.status = 'published' AND NEW.status = 'retired')
    ) THEN
        RAISE EXCEPTION 'invalid rule governance status transition from % to %', OLD.status, NEW.status;
    END IF;

    IF NEW.status IN ('simulated', 'reviewed', 'approved', 'published', 'retired')
        AND NEW.author_subject IS NULL
    THEN
        RAISE EXCEPTION 'rule governance requires an author before simulation';
    END IF;

    IF NEW.status IN ('reviewed', 'approved', 'published', 'retired')
        AND NEW.reviewer_subject IS NULL
    THEN
        RAISE EXCEPTION 'rule governance requires a reviewer before approval';
    END IF;

    IF NEW.status IN ('approved', 'published', 'retired')
        AND NEW.approver_subject IS NULL
    THEN
        RAISE EXCEPTION 'rule governance requires an approver before publication';
    END IF;

    IF NEW.author_subject IS NOT NULL AND NEW.reviewer_subject IS NOT NULL
        AND NEW.author_subject = NEW.reviewer_subject
    THEN
        RAISE EXCEPTION 'rule governance author and reviewer must be distinct';
    END IF;
    IF NEW.author_subject IS NOT NULL AND NEW.approver_subject IS NOT NULL
        AND NEW.author_subject = NEW.approver_subject
    THEN
        RAISE EXCEPTION 'rule governance author and approver must be distinct';
    END IF;
    IF NEW.reviewer_subject IS NOT NULL AND NEW.approver_subject IS NOT NULL
        AND NEW.reviewer_subject = NEW.approver_subject
    THEN
        RAISE EXCEPTION 'rule governance reviewer and approver must be distinct';
    END IF;

    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS rule_governance_transition_guard ON rule_sets;
CREATE TRIGGER rule_governance_transition_guard
    BEFORE UPDATE ON rule_sets
    FOR EACH ROW
    EXECUTE FUNCTION validate_rule_governance_transition();
