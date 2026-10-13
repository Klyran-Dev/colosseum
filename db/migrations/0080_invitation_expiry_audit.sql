-- ADR-0499: record terminal invitation expiry during lifecycle projection or mutation.
BEGIN;

ALTER TABLE auth_audit_events
    DROP CONSTRAINT IF EXISTS auth_audit_events_event_type_check;

ALTER TABLE auth_audit_events
    ADD CONSTRAINT auth_audit_events_event_type_check
    CHECK (event_type IN (
        'otp_requested', 'otp_delivered', 'otp_delivery_failed',
        'otp_verified', 'otp_rejected', 'user_provisioned',
        'role_assigned', 'role_revoked', 'invitation_created',
        'invitation_revoked', 'invitation_accepted', 'invitation_expired'
    ));

COMMIT;
