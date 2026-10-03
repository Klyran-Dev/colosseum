-- Additive revenue-stream authority subject and shared wallet-proof roles.
-- Existing plans without an operator remain readable but are not eligible for
-- chain compilation until an explicit migration/approval workflow assigns one.

BEGIN;

ALTER TABLE revenue_stream_plans
    ADD COLUMN IF NOT EXISTS operator_subject text;

ALTER TABLE revenue_stream_plans
    ADD CONSTRAINT revenue_stream_plans_operator_subject_check
    CHECK (
        operator_subject IS NULL
        OR (
            length(btrim(operator_subject)) BETWEEN 1 AND 256
            AND operator_subject !~ '[[:cntrl:]]'
            AND operator_subject <> proposer_subject
        )
    );

CREATE INDEX IF NOT EXISTS revenue_stream_plans_operator_subject_idx
    ON revenue_stream_plans (merchant_id, operator_subject)
    WHERE operator_subject IS NOT NULL;

ALTER TABLE settlement_adjustment_authority_challenges
    DROP CONSTRAINT IF EXISTS settlement_adjustment_authority_challenges_role_check;
ALTER TABLE settlement_adjustment_authority_challenges
    ADD CONSTRAINT settlement_adjustment_authority_challenges_role_check
    CHECK (role IN (
        'proposer', 'approver', 'governance', 'refund_funder',
        'revenue_stream_proposer', 'revenue_stream_approver',
        'revenue_stream_operator', 'revenue_stream_governance'
    ));

ALTER TABLE settlement_adjustment_authority_bindings
    DROP CONSTRAINT IF EXISTS settlement_adjustment_authority_bindings_role_check;
ALTER TABLE settlement_adjustment_authority_bindings
    ADD CONSTRAINT settlement_adjustment_authority_bindings_role_check
    CHECK (role IN (
        'proposer', 'approver', 'governance', 'refund_funder',
        'revenue_stream_proposer', 'revenue_stream_approver',
        'revenue_stream_operator', 'revenue_stream_governance'
    ));

COMMIT;
