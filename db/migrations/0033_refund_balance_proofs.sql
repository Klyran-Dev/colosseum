-- Immutable transaction-bound refund balance proofs.
-- This is evidence only; it does not add a paid or chain_executed state.

BEGIN;

CREATE TABLE IF NOT EXISTS refund_balance_proofs (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    stream_key text NOT NULL,
    signature text NOT NULL,
    event_index smallint NOT NULL CHECK (event_index >= 0),
    slot numeric(20, 0) NOT NULL CHECK (slot > 0 AND slot <= 18446744073709551615),
    claim bytea NOT NULL CHECK (octet_length(claim) = 32),
    adjustment bytea NOT NULL CHECK (octet_length(adjustment) = 32),
    original_receipt bytea NOT NULL CHECK (octet_length(original_receipt) = 32),
    merchant bytea NOT NULL CHECK (octet_length(merchant) = 32),
    mint bytea NOT NULL CHECK (octet_length(mint) = 32),
    token_program bytea NOT NULL CHECK (octet_length(token_program) = 32),
    tax_vault bytea NOT NULL CHECK (octet_length(tax_vault) = 32),
    refund_source bytea NOT NULL CHECK (octet_length(refund_source) = 32),
    payer_destination bytea NOT NULL CHECK (octet_length(payer_destination) = 32),
    vault_authority bytea NOT NULL CHECK (octet_length(vault_authority) = 32),
    refund_funder bytea NOT NULL CHECK (octet_length(refund_funder) = 32),
    payer bytea NOT NULL CHECK (octet_length(payer) = 32),
    nonce numeric(20, 0) NOT NULL CHECK (nonce > 0 AND nonce <= 18446744073709551615),
    gross_atomic numeric(20, 0) NOT NULL CHECK (gross_atomic > 0 AND gross_atomic <= 18446744073709551615),
    tax_atomic numeric(20, 0) NOT NULL CHECK (tax_atomic >= 0 AND tax_atomic <= 18446744073709551615),
    net_atomic numeric(20, 0) NOT NULL CHECK (net_atomic >= 0 AND net_atomic <= 18446744073709551615),
    tax_vault_before_atomic numeric(20, 0) NOT NULL CHECK (tax_vault_before_atomic <= 18446744073709551615),
    tax_vault_after_atomic numeric(20, 0) NOT NULL CHECK (tax_vault_after_atomic <= 18446744073709551615),
    refund_source_before_atomic numeric(20, 0) NOT NULL CHECK (refund_source_before_atomic <= 18446744073709551615),
    refund_source_after_atomic numeric(20, 0) NOT NULL CHECK (refund_source_after_atomic <= 18446744073709551615),
    payer_destination_before_atomic numeric(20, 0) NOT NULL CHECK (payer_destination_before_atomic <= 18446744073709551615),
    payer_destination_after_atomic numeric(20, 0) NOT NULL CHECK (payer_destination_after_atomic <= 18446744073709551615),
    proof_digest bytea NOT NULL CHECK (octet_length(proof_digest) = 32),
    created_at timestamptz NOT NULL DEFAULT now(),
    CHECK (uuid_extract_version(id) = 7),
    CHECK (length(signature) BETWEEN 1 AND 128),
    CHECK (tax_atomic + net_atomic = gross_atomic),
    CHECK (tax_vault_before_atomic - tax_atomic = tax_vault_after_atomic),
    CHECK (refund_source_before_atomic - net_atomic = refund_source_after_atomic),
    CHECK (payer_destination_before_atomic + gross_atomic = payer_destination_after_atomic),
    CHECK (tax_vault <> refund_source AND tax_vault <> payer_destination AND refund_source <> payer_destination),
    CHECK (refund_funder <> refund_source AND payer <> payer_destination),
    FOREIGN KEY (stream_key, signature, event_index)
        REFERENCES indexed_adjustment_refund_executions(stream_key, signature, event_index)
        DEFERRABLE INITIALLY IMMEDIATE,
    UNIQUE (stream_key, signature, event_index)
);

CREATE INDEX IF NOT EXISTS refund_balance_proofs_claim_idx
    ON refund_balance_proofs (stream_key, claim, slot DESC);
CREATE INDEX IF NOT EXISTS refund_balance_proofs_adjustment_idx
    ON refund_balance_proofs (stream_key, adjustment, created_at DESC, id DESC);

CREATE OR REPLACE FUNCTION prevent_refund_balance_proof_mutation()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    RAISE EXCEPTION 'refund balance proofs are append-only';
END;
$$;

DROP TRIGGER IF EXISTS refund_balance_proof_mutation_guard
    ON refund_balance_proofs;
CREATE TRIGGER refund_balance_proof_mutation_guard
    BEFORE UPDATE OR DELETE ON refund_balance_proofs
    FOR EACH ROW
    EXECUTE FUNCTION prevent_refund_balance_proof_mutation();

COMMIT;
