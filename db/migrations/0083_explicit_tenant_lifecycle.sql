-- ADR-0527: make the tenant/workspace lifecycle explicit and auditable.
-- This migration is additive. Existing auth users are backfilled into one
-- explicit tenant row per merchant_id before the foreign key is installed.

BEGIN;

ALTER TABLE auth_audit_events
    DROP CONSTRAINT IF EXISTS auth_audit_events_event_type_check;

ALTER TABLE auth_audit_events
    ADD CONSTRAINT auth_audit_events_event_type_check
    CHECK (event_type IN (
        'otp_requested', 'otp_delivered', 'otp_delivery_failed',
        'otp_verified', 'otp_rejected', 'user_provisioned',
        'role_assigned', 'role_revoked', 'invitation_created',
        'invitation_revoked', 'invitation_accepted', 'invitation_expired',
        'tenant_profile_updated', 'administrator_promoted'
    ));

CREATE TABLE IF NOT EXISTS auth_tenants (
    id uuid PRIMARY KEY,
    display_name text NOT NULL DEFAULT 'Untitled workspace',
    status text NOT NULL DEFAULT 'active',
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK (uuid_extract_version(id) = 7),
    CHECK (length(btrim(display_name)) BETWEEN 1 AND 120),
    CHECK (display_name = btrim(display_name)),
    CHECK (status IN ('active', 'suspended'))
);

INSERT INTO auth_tenants (id)
SELECT DISTINCT merchant_id
FROM auth_users
ON CONFLICT (id) DO NOTHING;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conrelid = 'auth_users'::regclass
          AND conname = 'auth_users_merchant_tenant_fk'
    ) THEN
        ALTER TABLE auth_users
            ADD CONSTRAINT auth_users_merchant_tenant_fk
            FOREIGN KEY (merchant_id) REFERENCES auth_tenants(id);
    END IF;
END;
$$;

CREATE INDEX IF NOT EXISTS auth_tenants_status_idx
    ON auth_tenants (status, created_at DESC, id DESC);

CREATE TABLE IF NOT EXISTS auth_tenant_profile_mutations (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    idempotency_key uuid NOT NULL UNIQUE,
    actor_id uuid NOT NULL REFERENCES auth_users(id),
    merchant_id uuid NOT NULL REFERENCES auth_tenants(id),
    display_name text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    CHECK (uuid_extract_version(id) = 7),
    CHECK (uuid_extract_version(idempotency_key) = 7),
    CHECK (uuid_extract_version(merchant_id) = 7),
    CHECK (length(btrim(display_name)) BETWEEN 1 AND 120),
    CHECK (display_name = btrim(display_name))
);

CREATE INDEX IF NOT EXISTS auth_tenant_profile_mutations_tenant_created_idx
    ON auth_tenant_profile_mutations (merchant_id, created_at DESC, id DESC);

CREATE TABLE IF NOT EXISTS auth_administrator_mutations (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    idempotency_key uuid NOT NULL UNIQUE,
    actor_id uuid NOT NULL REFERENCES auth_users(id),
    target_id uuid NOT NULL REFERENCES auth_users(id),
    merchant_id uuid NOT NULL REFERENCES auth_tenants(id),
    action text NOT NULL CHECK (action = 'promote'),
    created_at timestamptz NOT NULL DEFAULT now(),
    CHECK (uuid_extract_version(id) = 7),
    CHECK (uuid_extract_version(idempotency_key) = 7),
    CHECK (uuid_extract_version(merchant_id) = 7)
);

CREATE INDEX IF NOT EXISTS auth_administrator_mutations_tenant_created_idx
    ON auth_administrator_mutations (merchant_id, created_at DESC, id DESC);

COMMIT;
