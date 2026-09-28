-- Extend the existing one-time wallet-proof authority boundary with the
-- explicit refund-funder role required by ADR-0175. This role is still
-- merchant-scoped and cannot be used as a proposer or approver implicitly.

BEGIN;

ALTER TABLE settlement_adjustment_authority_challenges
    DROP CONSTRAINT IF EXISTS settlement_adjustment_authority_challenges_role_check;
ALTER TABLE settlement_adjustment_authority_challenges
    ADD CONSTRAINT settlement_adjustment_authority_challenges_role_check
    CHECK (role IN ('proposer', 'approver', 'refund_funder'));

ALTER TABLE settlement_adjustment_authority_bindings
    DROP CONSTRAINT IF EXISTS settlement_adjustment_authority_bindings_role_check;
ALTER TABLE settlement_adjustment_authority_bindings
    ADD CONSTRAINT settlement_adjustment_authority_bindings_role_check
    CHECK (role IN ('proposer', 'approver', 'refund_funder'));

COMMIT;
