-- Immutable raw Anchor event bytes retained beside the normalized receipt.
-- This supports forensic replay without trusting a provider's re-encoding.

CREATE TABLE IF NOT EXISTS raw_chain_evidence (
    stream_key text NOT NULL REFERENCES chain_checkpoints(stream_key),
    signature text NOT NULL,
    slot numeric(20, 0) NOT NULL CHECK (slot >= 0 AND slot <= 18446744073709551615),
    event_index smallint NOT NULL CHECK (event_index >= 0),
    event_discriminator bytea NOT NULL CHECK (octet_length(event_discriminator) = 8),
    event_data bytea NOT NULL CHECK (octet_length(event_data) BETWEEN 8 AND 1048576),
    evidence_digest bytea NOT NULL CHECK (octet_length(evidence_digest) = 32),
    captured_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (stream_key, signature, event_index),
    CHECK (length(signature) BETWEEN 1 AND 128),
    CHECK (substring(event_data from 1 for 8) = event_discriminator)
);

CREATE INDEX IF NOT EXISTS raw_chain_evidence_slot_idx
    ON raw_chain_evidence (stream_key, slot, signature, event_index);
