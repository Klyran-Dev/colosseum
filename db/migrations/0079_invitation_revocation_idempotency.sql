-- ADR-0492: tenant-scoped invitation revocation and idempotency.
BEGIN;

CREATE TABLE IF NOT EXISTS auth_invitation_mutations (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    idempotency_key uuid NOT NULL UNIQUE,
    actor_id uuid NOT NULL REFERENCES auth_users(id),
    invitation_id uuid NOT NULL REFERENCES auth_tenant_invitations(id),
    merchant_id uuid NOT NULL,
    action text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    CHECK (uuid_extract_version(id) = 7),
    CHECK (uuid_extract_version(idempotency_key) = 7),
    CHECK (action = 'revoke')
);

CREATE INDEX IF NOT EXISTS auth_invitation_mutations_tenant_created_idx
    ON auth_invitation_mutations (merchant_id, created_at DESC, id DESC);

COMMIT;
