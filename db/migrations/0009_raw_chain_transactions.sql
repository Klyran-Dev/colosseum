-- Bounded full transaction payloads retained for finalized projection rebuilds.
-- The normalized receipt and decoded Anchor event remain independently
-- validated; this table preserves the provider transaction object as evidence.

CREATE TABLE IF NOT EXISTS raw_chain_transactions (
    stream_key text NOT NULL REFERENCES chain_checkpoints(stream_key),
    signature text NOT NULL,
    slot numeric(20, 0) NOT NULL CHECK (slot >= 0 AND slot <= 18446744073709551615),
    transaction_index numeric(20, 0) NOT NULL CHECK (transaction_index >= 0 AND transaction_index <= 4294967295),
    payload bytea NOT NULL CHECK (octet_length(payload) BETWEEN 1 AND 524288),
    payload_digest bytea NOT NULL CHECK (octet_length(payload_digest) = 32),
    captured_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (stream_key, signature, transaction_index),
    CHECK (length(signature) BETWEEN 1 AND 128)
);

CREATE INDEX IF NOT EXISTS raw_chain_transactions_slot_idx
    ON raw_chain_transactions (stream_key, slot, signature, transaction_index);
