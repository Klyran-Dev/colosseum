DO $$
DECLARE
    test_context_id uuid;
    test_merchant_id uuid := uuidv7();
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_name = 'tax_calculations'
          AND column_name = 'applicability_context_id'
    ) OR NOT EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_name = 'tax_calculations'
          AND column_name = 'applicability_attestation_digest'
    ) OR NOT EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_name = 'tax_document_calculations'
          AND column_name = 'applicability_context_id'
    ) OR NOT EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_name = 'tax_document_calculations'
          AND column_name = 'applicability_attestation_digest'
    ) THEN
        RAISE EXCEPTION 'quote tables do not expose the applicability binding columns';
    END IF;

    INSERT INTO tax_applicability_contexts (
        merchant_id, policy_id, policy_version, policy_digest, facts_digest,
        decision_digest, jurisdiction_resolution_digest, evidence_digest,
        attestation_digest, facts_document, decision_document,
        attestation_document, status, effective_from, evidence_expires_at,
        provenance, author_subject
    ) VALUES (
        test_merchant_id, 'policy-test', 1, decode(repeat('01', 32), 'hex'),
        decode(repeat('02', 32), 'hex'), decode(repeat('03', 32), 'hex'),
        decode(repeat('04', 32), 'hex'), decode(repeat('05', 32), 'hex'),
        decode(repeat('06', 32), 'hex'), '{}'::jsonb, '{}'::jsonb, '{}'::jsonb,
        'draft', now(), now() + interval '1 hour', '{}'::jsonb, 'author'
    )
    RETURNING id INTO test_context_id;

    IF uuid_extract_version(test_context_id) <> 7 THEN
        RAISE EXCEPTION 'applicability context identity is not UUIDv7';
    END IF;

    INSERT INTO tax_applicability_context_events
        (context_id, merchant_id, event_type, actor_subject, evidence)
    VALUES (test_context_id, test_merchant_id, 'created', 'author', '{}'::jsonb);

    UPDATE tax_applicability_contexts AS c
    SET status = 'review', reviewer_subject = 'reviewer'
    WHERE c.id = test_context_id AND c.merchant_id = test_merchant_id;

    UPDATE tax_applicability_contexts AS c
    SET status = 'approved', approver_subject = 'approver'
    WHERE c.id = test_context_id AND c.merchant_id = test_merchant_id;

    UPDATE tax_applicability_contexts AS c
    SET status = 'published'
    WHERE c.id = test_context_id AND c.merchant_id = test_merchant_id;

    BEGIN
        UPDATE tax_applicability_contexts AS c
        SET decision_digest = decode(repeat('07', 32), 'hex')
        WHERE c.id = test_context_id AND c.merchant_id = test_merchant_id;
        RAISE EXCEPTION 'published applicability context mutation unexpectedly succeeded';
    EXCEPTION WHEN OTHERS THEN
        IF POSITION('published or retired applicability contexts are immutable' IN SQLERRM) = 0 THEN
            RAISE;
        END IF;
    END;

    BEGIN
        DELETE FROM tax_applicability_context_events
        WHERE tax_applicability_context_events.context_id = test_context_id;
        RAISE EXCEPTION 'applicability context event deletion unexpectedly succeeded';
    EXCEPTION WHEN OTHERS THEN
        IF POSITION('tax applicability context events are append-only' IN SQLERRM) = 0 THEN
            RAISE;
        END IF;
    END;
END $$;
