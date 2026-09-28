-- Expand phase for authoritative tax-calculation replay.
-- Existing rows remain intentionally non-replayable until an evidence-backed
-- backfill is reviewed; no synthetic facts are inferred from result totals.
ALTER TABLE tax_calculations
    ADD COLUMN IF NOT EXISTS facts_document jsonb;

ALTER TABLE tax_calculations
    DROP CONSTRAINT IF EXISTS tax_calculations_facts_document_object_check;

ALTER TABLE tax_calculations
    ADD CONSTRAINT tax_calculations_facts_document_object_check
    CHECK (facts_document IS NULL OR jsonb_typeof(facts_document) = 'object');

COMMENT ON COLUMN tax_calculations.facts_document IS
    'Bounded normalized TaxFacts input used for deterministic replay; NULL means legacy evidence is not replayable.';
