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
`fx-evidence` crate. The vendor-neutral HTTP transport adapter is implemented
as `fx-provider-http`; a concrete production provider adapter, HSM/KMS key custody,
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
returning evidence. The registry now exposes the same exact-policy binding for
the async issuer, so an HSM/KMS adapter cannot bypass the active policy
snapshot. The synchronous signer remains a test/local adapter path; key
ceremony, rotation, revocation, recovery, and production custody evidence
remain release gates.

ADR-0255 adds a vendor-neutral bounded HTTP source adapter. It makes endpoint
scheme, loopback development use, credential handling, timeout, body size,
strict JSON decoding, exact decimal parsing, configured provider identity, and
raw-response evidence hashing explicit. It does not select a provider or make
HTTP transport evidence sufficient for legal, regional, custody, or production
release approval.

ADR-0256 adds the corresponding vendor-neutral bounded HTTP signer transport.
It transmits one exact canonical payload with a server-owned key identity,
rejects redirects and non-canonical signatures, and never retries a remote
signing operation. The async issuer still performs the authoritative envelope
verification; HSM/KMS custody and key-ceremony evidence remain release gates.

ADR-0257 adds the server-owned `FxEvidenceCoordinator`. It captures the policy
and issuer from one registry snapshot before provider work, then composes
bounded acquisition, deterministic agreement, one async signature, and local
envelope verification. It is a reusable application boundary, not a legal,
tax, persistence, or Solana settlement authority.

ADR-0258 separates current issuance validity from historical replay integrity.
`FxEvidenceEnvelope::verify_integrity` proves the immutable merchant/facts
binding, canonical payload digest, and signature without applying the original
freshness window. The async signer port also supports `Arc` ownership so a
shared HSM/KMS adapter does not need to clone private-key state. Complete
envelope persistence and quote replay integration are implemented by ADR-0259
at the storage boundary. ADR-0260 composes that boundary into authenticated
aggregate and document quote handlers through a server-owned issuance port.

ADR-0259 makes that evidence durable inside the immutable quote
`evidence_refs` document. The storage adapter binds the complete envelope to
the pre-resolution facts digest, checks it against the exact tax-engine FX
projection, and revalidates it during tenant-scoped replay. Quotes without FX
remain unchanged; a quote that claims FX conversion without the complete
envelope fails closed. Same-currency requests bypass the issuer; cross-currency
requests fail closed when no approved issuer is configured. Quote idempotency
is based on pre-resolution facts and normalized source currency rather than
mutable provider quote metadata.

ADR-0261 composes the bounded HTTP provider and signer adapters once during
local API startup. The bootstrap uses strict policy/source/signer configuration
and secret indirection; it does not select a vendor or activate
staging/production while the Tier-1 decision register remains open.
