-- ADR-0441 -- Separate email-OTP identity service state.

BEGIN;

CREATE TABLE IF NOT EXISTS auth_users (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    email_normalized text NOT NULL UNIQUE,
    merchant_id uuid NOT NULL UNIQUE,
    status text NOT NULL DEFAULT 'active',
    created_at timestamptz NOT NULL DEFAULT now(),
    last_verified_at timestamptz NOT NULL DEFAULT now(),
    CHECK (uuid_extract_version(id) = 7),
    CHECK (uuid_extract_version(merchant_id) = 7),
    CHECK (length(email_normalized) BETWEEN 3 AND 320),
    CHECK (status IN ('active', 'disabled'))
);

CREATE TABLE IF NOT EXISTS auth_otp_challenges (
    id uuid PRIMARY KEY,
    email_normalized text NOT NULL,
    code_digest bytea NOT NULL,
    source_digest bytea NOT NULL,
    status text NOT NULL DEFAULT 'pending',
    attempts integer NOT NULL DEFAULT 0,
    max_attempts integer NOT NULL,
    expires_at timestamptz NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    delivered_at timestamptz,
    consumed_at timestamptz,
    CHECK (uuid_extract_version(id) = 7),
    CHECK (length(email_normalized) BETWEEN 3 AND 320),
    CHECK (octet_length(code_digest) = 32),
    CHECK (octet_length(source_digest) = 32),
    CHECK (status IN ('pending', 'sent', 'delivery_failed', 'consumed', 'expired', 'exhausted')),
    CHECK (attempts >= 0 AND attempts <= max_attempts),
    CHECK (max_attempts BETWEEN 1 AND 10),
    CHECK (expires_at > created_at),
    CHECK ((status = 'consumed') = (consumed_at IS NOT NULL))
);

CREATE INDEX IF NOT EXISTS auth_otp_challenges_email_created_idx
    ON auth_otp_challenges (email_normalized, created_at DESC);

CREATE INDEX IF NOT EXISTS auth_otp_challenges_source_created_idx
    ON auth_otp_challenges (source_digest, created_at DESC);

CREATE TABLE IF NOT EXISTS auth_audit_events (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    event_type text NOT NULL,
    subject_id uuid,
    challenge_id uuid,
    outcome text NOT NULL,
    occurred_at timestamptz NOT NULL DEFAULT now(),
    metadata jsonb NOT NULL DEFAULT '{}'::jsonb,
    CHECK (uuid_extract_version(id) = 7),
    CHECK (event_type IN ('otp_requested', 'otp_delivered', 'otp_delivery_failed', 'otp_verified', 'otp_rejected', 'user_provisioned')),
    CHECK (outcome IN ('accepted', 'rejected', 'failed')),
    CHECK (jsonb_typeof(metadata) = 'object')
);

CREATE INDEX IF NOT EXISTS auth_audit_events_occurred_idx
    ON auth_audit_events (occurred_at DESC, id DESC);

COMMIT;
