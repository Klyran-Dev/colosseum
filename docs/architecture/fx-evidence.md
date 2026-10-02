# Server-owned FX evidence bounded context

The FX evidence context is the trust boundary between an external rate
provider and the deterministic Tier-1 Tax Engine. It authenticates provider
output and applies bounded freshness checks; it does not choose a legal rate,
call a provider from the tax engine, or perform settlement.

## Flow

1. An authenticated provider adapter receives a source/target request and raw
   commercial-facts digest.
2. The adapter verifies provider policy, freshness, deviation, currency, and
   legal acceptance before presenting a signed `FxEvidenceEnvelope`.
3. The `fx-evidence` crate verifies the registered Ed25519 key, canonical
   payload digest, merchant/facts binding, validity window, and exact fields.
4. The verified envelope maps to `tax-engine::FxConversionEvidence`.
5. ADR-0244 normalizes source amounts exactly once; the original evidence stays
   in the calculation digest/result for replay and explanation.

## Trust boundaries

The browser and public quote JSON cannot provide FX authority. Provider keys
are server-side configuration and must be rotated atomically through a bounded
snapshot. The application supplies the current time and merchant/facts
digests; the pure verifier does not read a clock or access a network. A bad
snapshot, unknown key, signature mismatch, stale quote, future observation,
or policy disagreement fails closed.

## Scope and release status

The signed envelope and atomic key-registry verifier are implemented as the
`fx-evidence` crate. The production provider adapter, HSM/KMS key custody,
freshness/deviation policy, legal acceptance, regional provider SLOs, and
cross-currency quote enablement remain separate release gates. Until those
gates are satisfied, the API rejects caller-supplied FX envelopes and does not
claim global legal or filing compliance.
