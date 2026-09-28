-- Finalized typed evidence for AdjustmentRefundExecuted.
-- Raw Anchor bytes remain authoritative evidence; this table is a bounded,
-- replay-safe operational projection and never a balance ledger.

CREATE TABLE IF NOT EXISTS indexed_adjustment_refund_executions (
    stream_key text NOT NULL REFERENCES chain_checkpoints(stream_key),
    signature text NOT NULL,
    slot numeric(20, 0) NOT NULL CHECK (slot >= 0 AND slot <= 18446744073709551615),
    event_index smallint NOT NULL CHECK (event_index >= 0),
    claim bytea NOT NULL CHECK (octet_length(claim) = 32),
    adjustment bytea NOT NULL CHECK (octet_length(adjustment) = 32),
    original_receipt bytea NOT NULL CHECK (octet_length(original_receipt) = 32),
    merchant bytea NOT NULL CHECK (octet_length(merchant) = 32),
    mint bytea NOT NULL CHECK (octet_length(mint) = 32),
    nonce numeric(20, 0) NOT NULL CHECK (nonce > 0 AND nonce <= 18446744073709551615),
    gross_magnitude numeric(20, 0) NOT NULL CHECK (gross_magnitude > 0 AND gross_magnitude <= 18446744073709551615),
    tax_magnitude numeric(20, 0) NOT NULL CHECK (tax_magnitude >= 0 AND tax_magnitude <= 18446744073709551615),
    net_magnitude numeric(20, 0) NOT NULL CHECK (net_magnitude >= 0 AND net_magnitude <= 18446744073709551615),
    refund_funder bytea NOT NULL CHECK (octet_length(refund_funder) = 32),
    refund_destination bytea NOT NULL CHECK (octet_length(refund_destination) = 32),
    approver bytea NOT NULL CHECK (octet_length(approver) = 32),
    reference_digest bytea NOT NULL CHECK (octet_length(reference_digest) = 32),
    evidence_digest bytea NOT NULL CHECK (octet_length(evidence_digest) = 32),
    indexed_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (stream_key, signature, event_index),
    CHECK (length(signature) BETWEEN 1 AND 128),
    CHECK (tax_magnitude + net_magnitude = gross_magnitude),
    CHECK (refund_funder <> refund_destination),
    CHECK (approver <> refund_funder),
    CHECK (approver <> refund_destination)
);

CREATE UNIQUE INDEX IF NOT EXISTS indexed_adjustment_refund_claim_idx
    ON indexed_adjustment_refund_executions (stream_key, claim);

CREATE INDEX IF NOT EXISTS indexed_adjustment_refund_slot_idx
    ON indexed_adjustment_refund_executions (stream_key, slot, signature, event_index);

CREATE INDEX IF NOT EXISTS indexed_adjustment_refund_adjustment_idx
    ON indexed_adjustment_refund_executions (stream_key, adjustment, slot DESC);
