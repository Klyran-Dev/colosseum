# ADR-0245: Server-owned signed FX evidence bounded context

## Status

Accepted for implementation.

## Context

ADR-0244 defines exact FX normalization but intentionally leaves provider
freshness, legal rate acceptance, and provider authenticity outside the pure
tax engine. A production global-tax system needs a separate trust boundary for
those concerns. Caller-provided rates cannot become authoritative, and a live
provider request inside tax calculation would make replay and low-latency
behavior nondeterministic.

## Decision

Create a dedicated `fx-evidence` Rust bounded context for signed provider
evidence. Its immutable envelope binds schema version, merchant identity, the
pre-FX commercial-facts digest, source/target currency, positive exact
rational rate, target atomic quantum, declared rounding mode, provider and
quote identity, observation time, expiry, evidence digest, and provider key
identity.

Providers sign the canonical payload with Ed25519. The verifier checks the
registered provider key, signature, payload digest, merchant/facts binding,
bounded validity window, exact arithmetic fields, and current observation
window. It then converts the verified envelope into the tax engine's
`FxConversionEvidence` without performing network, clock, or database work in
the domain crate.

Provider keys are held by an application adapter in an atomically replaceable,
bounded registry. Key reload is all-or-nothing; malformed or empty snapshots
leave the last valid snapshot active. Public quote payloads cannot carry this
envelope. A future authenticated provider adapter may resolve and attach it
server-side, after applying provider selection, freshness, legal acceptance,
rate deviation, and source-currency policy. Until that adapter exists,
cross-currency persisted quotes fail closed.

## Alternatives considered

See `docs/architecture/alternative-analysis.md`.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Reuse jurisdiction evidence with a generic payload | Less code | Weak domain typing, unclear rate semantics, accidental cross-context coupling | Rejected |
| Unsigned provider registry data | Simple and fast | A compromised API/operator can alter rates without cryptographic evidence | Rejected |
| Query an FX provider inside the tax engine | Fresh data | Nondeterministic latency, outage coupling, and non-replayable calculations | Rejected |
| Put FX rates on Solana | Strong chain visibility | Oracle lifecycle/cost, stale data, and unsuitable global legal-rate semantics | Rejected |
| **Signed FX evidence bounded context plus server-owned adapter** | Exact, auditable, replayable, bounded, and hot-path friendly | Requires key custody, provider policy, and operational rotation | **Selected** |

## Invariants and failure policy

1. The verifier never accepts an unknown provider key or caller-owned envelope.
2. The signature covers every rate, rounding, currency, identity, and validity
   field; no field is reconstructed from an unsigned request.
3. The envelope's facts digest is the digest of the raw facts before FX evidence
   is attached, preventing circular binding and rate substitution.
4. Observation and expiry are checked against an application-supplied clock;
   the domain verifier never reads system time.
5. Provider evidence lifetime and all strings/collections are bounded.
6. Invalid, stale, ambiguous, key-rotation, or policy-disallowed evidence
   fails closed before persistence or tax calculation.
7. Key reload is atomic and does not invalidate an in-flight verification
   snapshot; new requests use exactly one registry snapshot.

## Verification

- Failing-first Rust vectors cover canonical signing, tampered fields,
  merchant/facts substitution, expiry, future observation, unknown keys,
  malformed bounds, exact conversion mapping, and atomic key reload.
- The tax engine's no-network/no-clock contract remains unchanged.
- Public API tests continue to prove client FX evidence is rejected.
- Provider freshness, legal acceptance, production provider availability,
  HSM/KMS custody, and production SLO evidence remain independent release gates.
