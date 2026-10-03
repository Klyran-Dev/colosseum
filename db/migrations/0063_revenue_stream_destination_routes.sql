-- Server-owned revenue payout destinations. A database row is configuration
-- only until the dedicated finalized route verifier records evidence.

BEGIN;

CREATE TABLE IF NOT EXISTS revenue_stream_destination_routes (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    merchant_id uuid NOT NULL,
    route_key text NOT NULL,
    version integer NOT NULL,
    status text NOT NULL DEFAULT 'pending_verification',
    recipient_owner text NOT NULL,
    mint text NOT NULL,
    destination_token_account text NOT NULL,
    token_program text NOT NULL,
    verification_digest bytea,
    verification_slot numeric(20, 0),
    verification_commitment text,
    verification_evidence jsonb,
    verifier_version text,
    verified_at timestamptz,
    effective_from timestamptz NOT NULL DEFAULT now(),
    effective_until timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK (uuid_extract_version(id) = 7),
    CHECK (length(btrim(route_key)) BETWEEN 1 AND 200),
    CHECK (version > 0),
    CHECK (status IN ('pending_verification', 'active', 'suspended', 'retired')),
    CHECK (length(btrim(recipient_owner)) BETWEEN 32 AND 44),
    CHECK (length(btrim(mint)) BETWEEN 32 AND 44),
    CHECK (length(btrim(destination_token_account)) BETWEEN 32 AND 44),
    CHECK (length(btrim(token_program)) BETWEEN 32 AND 44),
    CHECK (effective_until IS NULL OR effective_until > effective_from),
    CHECK (verification_digest IS NULL OR octet_length(verification_digest) = 32),
    CHECK (verification_slot IS NULL OR (verification_slot > 0 AND verification_slot <= 18446744073709551615)),
    CHECK (verification_commitment IS NULL OR verification_commitment IN ('finalized', 'local_synthetic')),
    CHECK (verification_evidence IS NULL OR jsonb_typeof(verification_evidence) = 'object'),
    CHECK (status <> 'active' OR (
        verification_digest IS NOT NULL
        AND verification_slot IS NOT NULL
        AND verification_commitment = 'finalized'
        AND verification_evidence IS NOT NULL
        AND verifier_version IS NOT NULL
        AND verified_at IS NOT NULL
    )),
    CHECK (destination_token_account <> mint),
    UNIQUE (merchant_id, route_key, version),
    UNIQUE (id, merchant_id)
);

CREATE UNIQUE INDEX IF NOT EXISTS revenue_stream_destination_routes_active_idx
    ON revenue_stream_destination_routes (merchant_id, recipient_owner, mint)
    WHERE status = 'active';

CREATE INDEX IF NOT EXISTS revenue_stream_destination_routes_verification_idx
    ON revenue_stream_destination_routes (status, updated_at, id)
    WHERE status = 'pending_verification';

CREATE TABLE IF NOT EXISTS revenue_stream_destination_route_verifications (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    route_id uuid NOT NULL,
    merchant_id uuid NOT NULL,
    route_version integer NOT NULL,
    verification_digest bytea NOT NULL,
    verification_slot numeric(20, 0) NOT NULL,
    verification_commitment text NOT NULL,
    evidence jsonb NOT NULL,
    verifier_version text NOT NULL,
    recorded_at timestamptz NOT NULL DEFAULT now(),
    CHECK (uuid_extract_version(id) = 7),
    CHECK (uuid_extract_version(route_id) = 7),
    CHECK (route_version > 0),
    CHECK (octet_length(verification_digest) = 32 AND verification_digest <> repeat(E'\\000', 32)::bytea),
    CHECK (verification_slot > 0 AND verification_slot <= 18446744073709551615),
    CHECK (verification_commitment IN ('finalized', 'local_synthetic')),
    CHECK (jsonb_typeof(evidence) = 'object'),
    CHECK (length(btrim(verifier_version)) BETWEEN 1 AND 100),
    FOREIGN KEY (route_id, merchant_id)
        REFERENCES revenue_stream_destination_routes(id, merchant_id),
    UNIQUE (route_id, verification_digest),
    UNIQUE (id, route_id)
);

CREATE OR REPLACE FUNCTION prevent_revenue_stream_destination_route_mutation()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF OLD.status IN ('active', 'retired') THEN
        RAISE EXCEPTION 'verified revenue stream destination routes are immutable; create a new version';
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS revenue_stream_destination_route_mutation_guard
    ON revenue_stream_destination_routes;
CREATE TRIGGER revenue_stream_destination_route_mutation_guard
    BEFORE UPDATE OR DELETE ON revenue_stream_destination_routes
    FOR EACH ROW EXECUTE FUNCTION prevent_revenue_stream_destination_route_mutation();

CREATE OR REPLACE FUNCTION prevent_revenue_stream_destination_verification_mutation()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    RAISE EXCEPTION 'revenue stream destination verification evidence is append-only';
END;
$$;

DROP TRIGGER IF EXISTS revenue_stream_destination_verification_mutation_guard
    ON revenue_stream_destination_route_verifications;
CREATE TRIGGER revenue_stream_destination_verification_mutation_guard
    BEFORE UPDATE OR DELETE ON revenue_stream_destination_route_verifications
    FOR EACH ROW EXECUTE FUNCTION prevent_revenue_stream_destination_verification_mutation();

COMMIT;
