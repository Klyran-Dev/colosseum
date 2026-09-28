-- Finalized typed evidence for SplitSettlementExecuted. The raw Anchor event
-- remains in raw_chain_evidence; this table is its bounded operational index.

CREATE TABLE IF NOT EXISTS indexed_split_settlements (
    stream_key text NOT NULL REFERENCES chain_checkpoints(stream_key),
    signature text NOT NULL,
    slot numeric(20, 0) NOT NULL CHECK (slot >= 0 AND slot <= 18446744073709551615),
    receipt bytea NOT NULL CHECK (octet_length(receipt) = 32),
    payer bytea NOT NULL CHECK (octet_length(payer) = 32),
    merchant bytea NOT NULL CHECK (octet_length(merchant) = 32),
    mint bytea NOT NULL CHECK (octet_length(mint) = 32),
    nonce numeric(20, 0) NOT NULL CHECK (nonce >= 0 AND nonce <= 18446744073709551615),
    gross_atomic numeric(20, 0) NOT NULL CHECK (gross_atomic > 0 AND gross_atomic <= 18446744073709551615),
    tax_atomic numeric(20, 0) NOT NULL CHECK (tax_atomic >= 0 AND tax_atomic <= 18446744073709551615),
    net_atomic numeric(20, 0) NOT NULL CHECK (net_atomic >= 0 AND net_atomic <= 18446744073709551615),
    split_plan_version numeric(20, 0) NOT NULL CHECK (split_plan_version > 0 AND split_plan_version <= 18446744073709551615),
    split_plan_digest bytea NOT NULL CHECK (octet_length(split_plan_digest) = 32),
    policy_digest bytea NOT NULL CHECK (octet_length(policy_digest) = 32),
    evidence_digest bytea NOT NULL CHECK (octet_length(evidence_digest) = 32),
    indexed_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (stream_key, signature),
    CHECK (length(signature) BETWEEN 1 AND 128),
    CHECK (tax_atomic + net_atomic = gross_atomic)
);

CREATE INDEX IF NOT EXISTS indexed_split_settlements_slot_idx
    ON indexed_split_settlements (stream_key, slot, signature);

CREATE INDEX IF NOT EXISTS indexed_split_settlements_plan_idx
    ON indexed_split_settlements (stream_key, merchant, mint, split_plan_version, split_plan_digest);
