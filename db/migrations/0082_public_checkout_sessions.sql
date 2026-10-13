-- ADR-0525: bounded guest checkout handoff.
-- The raw public token is never persisted; only its SHA-256 digest is stored.

BEGIN;

ALTER TABLE settlement_intents
    ADD COLUMN IF NOT EXISTS checkout_session_id uuid;

CREATE UNIQUE INDEX IF NOT EXISTS settlement_intents_checkout_session_idx
    ON settlement_intents (checkout_session_id)
    WHERE checkout_session_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS checkout_sessions (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    merchant_id uuid NOT NULL,
    calculation_id uuid NOT NULL,
    route_id uuid NOT NULL,
    token_digest bytea NOT NULL,
    status text NOT NULL DEFAULT 'active',
    expires_at timestamptz NOT NULL,
    settlement_intent_id uuid REFERENCES settlement_intents(id),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK (uuid_extract_version(id) = 7),
    CHECK (octet_length(token_digest) = 32),
    CHECK (status IN ('active', 'expired', 'closed')),
    CHECK (expires_at > created_at)
);

CREATE UNIQUE INDEX IF NOT EXISTS checkout_sessions_token_digest_idx
    ON checkout_sessions (token_digest);
CREATE INDEX IF NOT EXISTS checkout_sessions_public_lookup_idx
    ON checkout_sessions (id, status, expires_at);
CREATE INDEX IF NOT EXISTS checkout_sessions_merchant_created_idx
    ON checkout_sessions (merchant_id, created_at DESC, id);

COMMIT;
