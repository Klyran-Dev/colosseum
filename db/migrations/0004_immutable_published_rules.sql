-- Published rule releases are immutable evidence. Corrections are new versions.

CREATE EXTENSION IF NOT EXISTS btree_gist;

CREATE OR REPLACE FUNCTION prevent_rule_release_mutation()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF OLD.status IN ('published', 'retired') THEN
        IF NEW.external_id IS DISTINCT FROM OLD.external_id
            OR NEW.version IS DISTINCT FROM OLD.version
            OR NEW.jurisdiction IS DISTINCT FROM OLD.jurisdiction
            OR NEW.regime IS DISTINCT FROM OLD.regime
            OR NEW.rule_digest IS DISTINCT FROM OLD.rule_digest
            OR NEW.rule_document IS DISTINCT FROM OLD.rule_document
            OR NEW.provenance IS DISTINCT FROM OLD.provenance
            OR NEW.effective_from IS DISTINCT FROM OLD.effective_from
            OR NEW.effective_until IS DISTINCT FROM OLD.effective_until
        THEN
            RAISE EXCEPTION 'published and retired rule releases are immutable';
        END IF;
    END IF;

    IF OLD.status = 'published' AND NEW.status NOT IN ('published', 'retired') THEN
        RAISE EXCEPTION 'published rule releases may only be retired';
    END IF;

    IF OLD.status = 'retired' AND NEW.status <> 'retired' THEN
        RAISE EXCEPTION 'retired rule releases cannot be reactivated';
    END IF;

    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS rule_release_mutation_guard ON rule_sets;
CREATE TRIGGER rule_release_mutation_guard
    BEFORE UPDATE ON rule_sets
    FOR EACH ROW
    EXECUTE FUNCTION prevent_rule_release_mutation();

CREATE OR REPLACE FUNCTION prevent_rule_release_delete()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF OLD.status IN ('published', 'retired') THEN
        RAISE EXCEPTION 'published and retired rule releases cannot be deleted';
    END IF;
    RETURN OLD;
END;
$$;

DROP TRIGGER IF EXISTS rule_release_delete_guard ON rule_sets;
CREATE TRIGGER rule_release_delete_guard
    BEFORE DELETE ON rule_sets
    FOR EACH ROW
    EXECUTE FUNCTION prevent_rule_release_delete();

ALTER TABLE rule_sets
    DROP CONSTRAINT IF EXISTS published_rule_windows_no_overlap;

ALTER TABLE rule_sets
    ADD CONSTRAINT published_rule_windows_no_overlap
    EXCLUDE USING gist (
        external_id WITH =,
        jurisdiction WITH =,
        regime WITH =,
        tstzrange(effective_from, COALESCE(effective_until, 'infinity'::timestamptz), '[)') WITH &&
    )
    WHERE (status = 'published');
