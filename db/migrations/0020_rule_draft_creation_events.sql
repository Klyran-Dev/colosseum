-- Rule authoring creation evidence. Draft creation is an append-only
-- governance event and is never a publication or legal approval.

ALTER TABLE rule_governance_events
    DROP CONSTRAINT IF EXISTS rule_governance_events_action_check;

ALTER TABLE rule_governance_events
    ADD CONSTRAINT rule_governance_events_action_check
    CHECK (action IN ('create', 'simulate', 'review', 'approve', 'publish', 'retire'));
