-- Durable finalized-chain projections and append-only reconciliation evidence.
-- Slot and amount fields use numeric(20,0) so the database does not narrow the
-- u64 domain used by Solana slots and atomic units.

CREATE TABLE IF NOT EXISTS chain_checkpoints (
    stream_key text PRIMARY KEY,
    next_slot numeric(20, 0) NOT NULL CHECK (next_slot >= 0 AND next_slot <= 18446744073709551615),
    finalized_slot numeric(20, 0) NOT NULL CHECK (finalized_slot >= 0 AND finalized_slot <= 18446744073709551615),
    generation numeric(20, 0) NOT NULL CHECK (generation >= 0 AND generation <= 18446744073709551615),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK (length(stream_key) BETWEEN 1 AND 200)
);

-- The finalized watermark may be ahead of the cursor during a bounded
-- historical backfill; the cursor must not be forced to catch up atomically.
ALTER TABLE chain_checkpoints
    DROP CONSTRAINT IF EXISTS chain_checkpoints_next_slot_check;

CREATE TABLE IF NOT EXISTS indexed_receipts (
    stream_key text NOT NULL REFERENCES chain_checkpoints(stream_key),
    signature text NOT NULL,
    slot numeric(20, 0) NOT NULL CHECK (slot >= 0 AND slot <= 18446744073709551615),
    gross_atomic numeric(20, 0) NOT NULL CHECK (gross_atomic > 0 AND gross_atomic <= 18446744073709551615),
    tax_atomic numeric(20, 0) NOT NULL CHECK (tax_atomic >= 0 AND tax_atomic <= 18446744073709551615),
    net_atomic numeric(20, 0) NOT NULL CHECK (net_atomic >= 0 AND net_atomic <= 18446744073709551615),
    evidence_digest bytea NOT NULL CHECK (octet_length(evidence_digest) = 32),
    indexed_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (stream_key, signature),
    CHECK (length(signature) BETWEEN 1 AND 128),
    CHECK (tax_atomic + net_atomic = gross_atomic)
);

CREATE INDEX IF NOT EXISTS indexed_receipts_slot_idx
    ON indexed_receipts (stream_key, slot, signature);

CREATE TABLE IF NOT EXISTS reconciliation_runs (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    stream_key text NOT NULL,
    expected_gross_atomic numeric(20, 0) NOT NULL CHECK (expected_gross_atomic >= 0 AND expected_gross_atomic <= 18446744073709551615),
    expected_tax_atomic numeric(20, 0) NOT NULL CHECK (expected_tax_atomic >= 0 AND expected_tax_atomic <= 18446744073709551615),
    observed_gross_atomic numeric(20, 0) NOT NULL CHECK (observed_gross_atomic >= 0 AND observed_gross_atomic <= 18446744073709551615),
    observed_tax_atomic numeric(20, 0) NOT NULL CHECK (observed_tax_atomic >= 0 AND observed_tax_atomic <= 18446744073709551615),
    status text NOT NULL CHECK (status IN ('matched', 'mismatch')),
    created_at timestamptz NOT NULL DEFAULT now(),
    CHECK (uuid_extract_version(id) = 7),
    CHECK (length(stream_key) BETWEEN 1 AND 200)
);

CREATE INDEX IF NOT EXISTS reconciliation_runs_stream_idx
    ON reconciliation_runs (stream_key, created_at DESC, id DESC);
