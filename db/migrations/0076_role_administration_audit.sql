-- ADR-0491: atomic, tenant-scoped role administration and idempotency.
BEGIN;

ALTER TABLE auth_audit_events
    DROP CONSTRAINT IF EXISTS auth_audit_events_event_type_check;

ALTER TABLE auth_audit_events
    ADD CONSTRAINT auth_audit_events_event_type_check
    CHECK (event_type IN (
        'otp_requested', 'otp_delivered', 'otp_delivery_failed', 'otp_verified',
        'otp_rejected', 'user_provisioned', 'role_assigned', 'role_revoked'
    ));

CREATE TABLE IF NOT EXISTS auth_role_mutations (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    idempotency_key uuid NOT NULL UNIQUE,
    actor_id uuid NOT NULL REFERENCES auth_users(id),
    target_id uuid NOT NULL REFERENCES auth_users(id),
    merchant_id uuid NOT NULL,
    role text NOT NULL,
    action text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    CHECK (uuid_extract_version(id) = 7),
    CHECK (uuid_extract_version(idempotency_key) = 7),
    CHECK (length(role) BETWEEN 1 AND 128),
    CHECK (action IN ('assign', 'revoke'))
);

CREATE INDEX IF NOT EXISTS auth_role_mutations_tenant_created_idx
    ON auth_role_mutations (merchant_id, created_at DESC, id DESC);

COMMIT;
