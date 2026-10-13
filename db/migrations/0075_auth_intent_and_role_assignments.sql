-- ADR-0490: bind the requested auth journey to the one-time challenge and
-- keep elevated responsibilities durable and server-owned.
BEGIN;

ALTER TABLE auth_otp_challenges
    ADD COLUMN IF NOT EXISTS intent text NOT NULL DEFAULT 'signin';

ALTER TABLE auth_otp_challenges
    DROP CONSTRAINT IF EXISTS auth_otp_challenges_intent_check;

ALTER TABLE auth_otp_challenges
    ADD CONSTRAINT auth_otp_challenges_intent_check
    CHECK (intent IN ('signin', 'signup'));

CREATE TABLE IF NOT EXISTS auth_user_roles (
    user_id uuid NOT NULL REFERENCES auth_users(id) ON DELETE CASCADE,
    role text NOT NULL,
    status text NOT NULL DEFAULT 'active',
    assigned_at timestamptz NOT NULL DEFAULT now(),
    assigned_by uuid,
    PRIMARY KEY (user_id, role),
    CHECK (length(role) BETWEEN 1 AND 128),
    CHECK (status IN ('active', 'revoked'))
);

CREATE INDEX IF NOT EXISTS auth_user_roles_active_idx
    ON auth_user_roles (user_id, status, role);

COMMIT;
