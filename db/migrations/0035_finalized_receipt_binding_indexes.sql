-- Index the server-owned adjustment binding join without adding a mutable
-- receipt authority. The receipt projection remains finalized-chain evidence.

BEGIN;

CREATE INDEX IF NOT EXISTS settlement_intents_finalized_binding_idx
    ON settlement_intents (merchant_id, id, signature, slot)
    WHERE status = 'finalized' AND commitment = 'finalized'
      AND signature IS NOT NULL AND slot IS NOT NULL;

CREATE INDEX IF NOT EXISTS indexed_receipts_binding_lookup_idx
    ON indexed_receipts (signature, slot, mint);

COMMIT;
