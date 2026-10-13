-- ADR-0491: a merchant tenant contains many authenticated users.
-- The original identity bootstrap constrained merchant_id to one user, which
-- made durable administrator/moderator administration impossible.
BEGIN;

ALTER TABLE auth_users
    DROP CONSTRAINT IF EXISTS auth_users_merchant_id_key;

CREATE INDEX IF NOT EXISTS auth_users_merchant_created_idx
    ON auth_users (merchant_id, created_at DESC, id DESC);

COMMIT;
