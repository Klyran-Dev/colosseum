-- Extend the existing wallet-proof authority control plane with a distinct
-- governance role used by the non-custodial refund-claim cancellation path.

BEGIN;

ALTER TABLE settlement_adjustment_authority_challenges
    DROP CONSTRAINT IF EXISTS settlement_adjustment_authority_challenges_role_check;
ALTER TABLE settlement_adjustment_authority_challenges
    ADD CONSTRAINT settlement_adjustment_authority_challenges_role_check
    CHECK (role IN ('proposer', 'approver', 'governance'));

ALTER TABLE settlement_adjustment_authority_bindings
    DROP CONSTRAINT IF EXISTS settlement_adjustment_authority_bindings_role_check;
ALTER TABLE settlement_adjustment_authority_bindings
    ADD CONSTRAINT settlement_adjustment_authority_bindings_role_check
    CHECK (role IN ('proposer', 'approver', 'governance'));

COMMIT;
