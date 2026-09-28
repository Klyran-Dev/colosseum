-- Finalized typed evidence for SettlementAdjustmentRecorded. Raw Anchor event
-- bytes remain authoritative and are persisted in raw_chain_evidence.

CREATE TABLE IF NOT EXISTS indexed_settlement_adjustments (
    stream_key text NOT NULL REFERENCES chain_checkpoints(stream_key),
    signature text NOT NULL,
    slot numeric(20, 0) NOT NULL CHECK (slot >= 0 AND slot <= 18446744073709551615),
    event_index smallint NOT NULL CHECK (event_index >= 0),
    adjustment bytea NOT NULL CHECK (octet_length(adjustment) = 32),
    original_receipt bytea NOT NULL CHECK (octet_length(original_receipt) = 32),
    merchant bytea NOT NULL CHECK (octet_length(merchant) = 32),
    mint bytea NOT NULL CHECK (octet_length(mint) = 32),
    nonce numeric(20, 0) NOT NULL CHECK (nonce > 0 AND nonce <= 18446744073709551615),
    kind smallint NOT NULL CHECK (kind BETWEEN 1 AND 5),
    gross_sign smallint NOT NULL CHECK (gross_sign IN (0, 1)),
    gross_magnitude numeric(20, 0) NOT NULL CHECK (gross_magnitude BETWEEN 0 AND 18446744073709551615),
    tax_sign smallint NOT NULL CHECK (tax_sign IN (0, 1)),
    tax_magnitude numeric(20, 0) NOT NULL CHECK (tax_magnitude BETWEEN 0 AND 18446744073709551615),
    net_sign smallint NOT NULL CHECK (net_sign IN (0, 1)),
    net_magnitude numeric(20, 0) NOT NULL CHECK (net_magnitude BETWEEN 0 AND 18446744073709551615),
    adjustment_digest bytea NOT NULL CHECK (octet_length(adjustment_digest) = 32),
    evidence_digest bytea NOT NULL CHECK (octet_length(evidence_digest) = 32),
    approver bytea NOT NULL CHECK (octet_length(approver) = 32),
    indexed_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (stream_key, signature, event_index),
    CHECK (length(signature) BETWEEN 1 AND 128),
    CHECK (gross_magnitude > 0 OR tax_magnitude > 0 OR net_magnitude > 0)
);

CREATE UNIQUE INDEX IF NOT EXISTS indexed_settlement_adjustment_identity_idx
    ON indexed_settlement_adjustments (stream_key, adjustment);

CREATE INDEX IF NOT EXISTS indexed_settlement_adjustment_slot_idx
    ON indexed_settlement_adjustments (stream_key, slot, signature, event_index);
