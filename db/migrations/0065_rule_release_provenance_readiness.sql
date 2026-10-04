-- ADR-0316: a rule may be authored with working provenance, but publication
-- requires an exact, bounded, versioned release-provenance envelope.

CREATE OR REPLACE FUNCTION validate_published_rule_release_provenance()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NEW.status = 'published' THEN
        IF jsonb_typeof(NEW.provenance) <> 'object'
            OR (SELECT count(*) FROM jsonb_object_keys(NEW.provenance)) <> 7
            OR NOT (NEW.provenance ?& ARRAY[
                'schema', 'source_reference', 'source_digest_hex',
                'legal_tax_owner', 'jurisdiction_scope',
                'reviewed_at_unix', 'review_reference'
            ])
        THEN
            RAISE EXCEPTION 'published rule provenance must be the exact tax-rule-provenance/v1 envelope';
        END IF;

        IF NEW.provenance->>'schema' <> 'tax-rule-provenance/v1' THEN
            RAISE EXCEPTION 'published rule provenance schema is unsupported';
        END IF;

        IF jsonb_typeof(NEW.provenance->'source_reference') <> 'string'
            OR octet_length(NEW.provenance->>'source_reference') NOT BETWEEN 1 AND 512
            OR btrim(NEW.provenance->>'source_reference') = ''
        THEN
            RAISE EXCEPTION 'published rule source_reference is invalid';
        END IF;

        IF jsonb_typeof(NEW.provenance->'source_digest_hex') <> 'string'
            OR NEW.provenance->>'source_digest_hex' !~ '^[0-9A-Fa-f]{64}$'
        THEN
            RAISE EXCEPTION 'published rule source_digest_hex must contain 64 hexadecimal characters';
        END IF;

        IF jsonb_typeof(NEW.provenance->'legal_tax_owner') <> 'string'
            OR octet_length(NEW.provenance->>'legal_tax_owner') NOT BETWEEN 1 AND 256
            OR btrim(NEW.provenance->>'legal_tax_owner') = ''
        THEN
            RAISE EXCEPTION 'published rule legal_tax_owner is invalid';
        END IF;

        IF jsonb_typeof(NEW.provenance->'jurisdiction_scope') <> 'string'
            OR octet_length(NEW.provenance->>'jurisdiction_scope') NOT BETWEEN 1 AND 64
            OR btrim(NEW.provenance->>'jurisdiction_scope') = ''
            OR NEW.provenance->>'jurisdiction_scope' <> NEW.jurisdiction
        THEN
            RAISE EXCEPTION 'published rule jurisdiction_scope does not match the rule jurisdiction';
        END IF;

        IF jsonb_typeof(NEW.provenance->'reviewed_at_unix') <> 'number'
            OR NEW.provenance->>'reviewed_at_unix' !~ '^[0-9]+$'
            OR (NEW.provenance->>'reviewed_at_unix')::numeric NOT BETWEEN 1 AND 4102444800
        THEN
            RAISE EXCEPTION 'published rule reviewed_at_unix must be between 1970 and 2100';
        END IF;

        IF jsonb_typeof(NEW.provenance->'review_reference') <> 'string'
            OR octet_length(NEW.provenance->>'review_reference') NOT BETWEEN 1 AND 512
            OR btrim(NEW.provenance->>'review_reference') = ''
        THEN
            RAISE EXCEPTION 'published rule review_reference is invalid';
        END IF;
    END IF;

    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS published_rule_provenance_readiness ON rule_sets;
CREATE TRIGGER published_rule_provenance_readiness
    BEFORE INSERT OR UPDATE OF status, provenance, jurisdiction
    ON rule_sets
    FOR EACH ROW
    EXECUTE FUNCTION validate_published_rule_release_provenance();
