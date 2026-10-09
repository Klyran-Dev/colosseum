-- Durable finalized account verification for ADR-0412 applicability bindings.
-- Raw chain event bytes remain authoritative; this table is an append-only
-- typed projection that may be created only after the raw event is present.

BEGIN;

CREATE TABLE IF NOT EXISTS applicability_binding_verifications (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    stream_key text NOT NULL REFERENCES chain_checkpoints(stream_key),
    signature text NOT NULL,
    event_index smallint NOT NULL CHECK (event_index >= 0),
    event_slot numeric(20, 0) NOT NULL CHECK (event_slot >= 0 AND event_slot <= 18446744073709551615),
    provider_slot numeric(20, 0) NOT NULL CHECK (provider_slot >= 0 AND provider_slot <= 18446744073709551615),
    receipt bytea NOT NULL CHECK (octet_length(receipt) = 32),
    binding bytea NOT NULL CHECK (octet_length(binding) = 32),
    merchant bytea NOT NULL CHECK (octet_length(merchant) = 32),
    mint bytea NOT NULL CHECK (octet_length(mint) = 32),
    context_id bytea NOT NULL CHECK (octet_length(context_id) = 16),
    attestation_digest bytea NOT NULL CHECK (octet_length(attestation_digest) = 32),
    event_evidence_digest bytea NOT NULL CHECK (octet_length(event_evidence_digest) = 32),
    verification_commitment text NOT NULL CHECK (verification_commitment = 'finalized'),
    verified_at timestamptz NOT NULL DEFAULT now(),
    CHECK (length(stream_key) BETWEEN 1 AND 200),
    CHECK (length(signature) BETWEEN 1 AND 128),
    CHECK (provider_slot >= event_slot),
    CHECK (receipt <> decode(repeat('00', 32), 'hex')),
    CHECK (binding <> decode(repeat('00', 32), 'hex')),
    CHECK (merchant <> decode(repeat('00', 32), 'hex')),
    CHECK (mint <> decode(repeat('00', 32), 'hex')),
    CHECK (context_id <> decode(repeat('00', 16), 'hex')),
    CHECK (attestation_digest <> decode(repeat('00', 32), 'hex')),
    CHECK (event_evidence_digest <> decode(repeat('00', 32), 'hex')),
    CHECK (uuid_extract_version(id) = 7),
    UNIQUE (stream_key, signature, event_index)
);

CREATE INDEX IF NOT EXISTS applicability_binding_verifications_receipt_idx
    ON applicability_binding_verifications (stream_key, receipt, event_slot, signature);

CREATE INDEX IF NOT EXISTS applicability_binding_verifications_binding_idx
    ON applicability_binding_verifications (stream_key, binding, provider_slot DESC);

COMMIT;
