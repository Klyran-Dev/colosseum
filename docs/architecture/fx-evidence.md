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

ADR-0246 adds the vendor-neutral `fx-provider` issuance boundary. It keeps
provider acquisition, exact policy evaluation, and signing behind explicit
ports while preserving the same fail-closed cross-currency gate. A concrete
provider and custody implementation must be approved and measured before it
can be enabled. Issued evidence derives its signed evidence digest from the
provider evidence, policy version, and approved legal-authority digest.

ADR-0247 provides the warm digest-keyed policy snapshot. Reload validation is
 atomic and invalid replacements retain the last known-good policy set.
Production issuance must resolve the exact approved digest through that
registry before creating an issuer; direct issuer construction is reserved for
isolated tests or explicitly reviewed adapters.
Each policy also carries a bounded exact-match provider allowlist. A quote from
an unapproved provider fails before signing; provider rotation requires a new
immutable policy version.
For policies requiring more than one provider, every candidate must pass the
same policy and exact economic terms must agree. The selector uses policy
priority rather than response order and fails closed on disagreement or
insufficient candidates.

ADR-0252 adds the bounded asynchronous acquisition boundary before that
selector. It validates request/source topology before network work, binds each
returned candidate to the source's server-owned provider identity, caps in-flight
calls at sixteen, applies per-provider and aggregate deadlines, propagates
cancellation, and returns bounded partial-failure classifications. Acquisition
results remain non-authoritative until ADR-0250 admission succeeds. No real
vendor adapter, retry policy, regional routing, or production SLO is implied by
the library implementation.

ADR-0254 normalizes successful candidates and bounded failure classifications
by the policy's provider priority after concurrent work completes. Response
timing therefore cannot change replay order, operational evidence order, or a
future canonical digest input.

ADR-0253 adds a non-blocking async signing port for HSM/KMS-compatible
adapters. The issuer applies an explicit signing deadline, signs one canonical
payload exactly once, and verifies the resulting envelope locally before
returning evidence. The synchronous signer remains a test/local adapter path;
key ceremony, rotation, revocation, recovery, and production custody evidence
remain release gates.
