-- ADR-0492: durable tenant invitations consumed by signup OTP verification.
BEGIN;

ALTER TABLE auth_audit_events
    DROP CONSTRAINT IF EXISTS auth_audit_events_event_type_check;

ALTER TABLE auth_audit_events
    ADD CONSTRAINT auth_audit_events_event_type_check
    CHECK (event_type IN (
        'otp_requested', 'otp_delivered', 'otp_delivery_failed',
        'otp_verified', 'otp_rejected', 'user_provisioned',
        'role_assigned', 'role_revoked', 'invitation_created',
        'invitation_revoked', 'invitation_accepted'
    ));

CREATE TABLE IF NOT EXISTS auth_tenant_invitations (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    idempotency_key uuid NOT NULL UNIQUE,
    merchant_id uuid NOT NULL,
    email_normalized text NOT NULL,
    role text NOT NULL DEFAULT 'merchant_operator',
    status text NOT NULL DEFAULT 'pending',
    invited_by uuid NOT NULL REFERENCES auth_users(id),
    accepted_user_id uuid REFERENCES auth_users(id),
    created_at timestamptz NOT NULL DEFAULT now(),
    expires_at timestamptz NOT NULL,
    accepted_at timestamptz,
    revoked_at timestamptz,
    CHECK (uuid_extract_version(id) = 7),
    CHECK (uuid_extract_version(idempotency_key) = 7),
    CHECK (uuid_extract_version(merchant_id) = 7),
    CHECK (length(email_normalized) BETWEEN 3 AND 320),
    CHECK (role IN ('merchant_operator', 'moderator')),
    CHECK (status IN ('pending', 'accepted', 'revoked', 'expired')),
    CHECK (expires_at > created_at),
    CHECK ((status = 'accepted') = (accepted_user_id IS NOT NULL AND accepted_at IS NOT NULL)),
    CHECK ((status = 'revoked') = (revoked_at IS NOT NULL))
);

CREATE UNIQUE INDEX IF NOT EXISTS auth_tenant_invitations_pending_email_key
    ON auth_tenant_invitations (merchant_id, email_normalized)
    WHERE status = 'pending';

CREATE INDEX IF NOT EXISTS auth_tenant_invitations_tenant_created_idx
    ON auth_tenant_invitations (merchant_id, created_at DESC, id DESC);

COMMIT;
