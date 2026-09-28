-- Append-only access audit for sensitive normalized tax evidence.
-- The ledger stores operational identity and digests, never decrypted facts.
CREATE TABLE IF NOT EXISTS evidence_access_audit_events (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    merchant_id uuid NOT NULL,
    calculation_id uuid NOT NULL,
    actor_subject text NOT NULL,
    request_id uuid NOT NULL,
    purpose text NOT NULL,
    outcome text NOT NULL,
    facts_digest bytea,
    occurred_at timestamptz NOT NULL DEFAULT now(),
    CHECK (uuid_extract_version(id) = 7),
    CHECK (length(actor_subject) BETWEEN 1 AND 256),
    CHECK (length(purpose) BETWEEN 1 AND 128),
    CHECK (length(outcome) BETWEEN 1 AND 64),
    CHECK (facts_digest IS NULL OR octet_length(facts_digest) = 32)
);

CREATE INDEX IF NOT EXISTS evidence_access_audit_lookup_idx
    ON evidence_access_audit_events (merchant_id, calculation_id, occurred_at DESC, id DESC);

COMMENT ON TABLE evidence_access_audit_events IS
    'Append-only operational access ledger for encrypted normalized tax evidence; facts remain off the audit record.';
