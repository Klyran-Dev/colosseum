# ADR-0246: Server-owned FX provider policy and issuance boundary

## Status

Accepted for implementation.

## Context

ADR-0245 authenticates signed FX evidence but deliberately does not select a
provider, decide whether a rate is legally acceptable, or define freshness and
deviation policy. Those decisions must be made before an envelope becomes
eligible for a quote. A provider HTTP client cannot be part of the pure tax
calculation path because it would add nondeterministic latency, external
failure, and replay drift.

The system therefore needs a production boundary that accepts a provider
candidate, evaluates bounded policy using exact arithmetic and an injected
clock, and issues evidence only through a server-owned signing port. The
boundary must remain vendor-neutral until provider contracts, legal ownership,
regional availability, and key-custody controls are approved.

## Decision

Add a pure Rust `fx-provider` bounded context with three explicit ports:

1. `FxQuoteSource` is an application-owned source port. Implementations may
   call a provider outside the tax engine, but must return a bounded,
   immutable candidate and remain outside the wallet-signing critical path.
2. `FxProviderPolicy` validates currency-pair allowlists, positive exact rates,
   bounded observation age/future skew/lifetime, legal-acceptance status, and
   optional deviation against a server-owned exact reference rate.
3. `FxEvidenceSigner` is a signing port. Production implementations are
   HSM/KMS-backed; test implementations may use an ephemeral Ed25519 key.

`FxEvidenceIssuer` composes the policy and signer ports. It binds merchant and
pre-FX facts digests, copies only validated candidate fields, computes the
canonical payload digest, and returns the signed `FxEvidenceEnvelope`. The
signed evidence digest is derived from the provider evidence digest, policy
digest, and approved legal-authority digest, so the accepted policy decision
cannot be detached from the envelope. It does not read a clock, make a network
request, persist data, or calculate tax.

Deviation is compared as exact rational cross-products against a basis-point
bound. Overflow, ambiguous reference rates, unsupported pairs, legal-policy
failure, stale/future observations, signer identity mismatch, or signature
shape mismatch fail closed. No fallback provider or unsigned quote is
permitted.

The first implementation is a vendor-neutral library boundary. A concrete
provider adapter, regional routing policy, legal approval record, HSM/KMS
implementation, and production SLO evidence remain separately governed
release gates. Until those gates are satisfied, cross-currency quote
enablement remains disabled.

## Alternatives considered

See `docs/architecture/alternative-analysis.md`.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Put provider HTTP and signing in the Tax Engine | Fewer modules | Impure calculation, unbounded external latency, difficult replay | Rejected |
| Let each provider decide freshness and legal acceptance | Provider-specific behavior | Inconsistent global semantics and weak governance | Rejected |
| Accept caller-supplied rate plus a later audit | Low initial latency | Forgery and settlement-boundary ambiguity | Rejected |
| Choose one vendor-specific adapter now | Immediate integration | Premature legal, regional, availability, and commercial coupling | Rejected |
| **Vendor-neutral source/policy/signer ports with exact issuance** | Deterministic, testable, low-latency, auditable, and replaceable | Requires explicit provider and custody gates | **Selected** |

## Invariants and failure policy

1. Only an authenticated server-owned adapter may invoke issuance.
2. Merchant identity and the pre-FX facts digest are signed bindings.
3. Every accepted rate and policy field is represented in the signed payload.
4. Exact arithmetic is used for rate validation and deviation checks; binary
   floating point is forbidden.
5. Provider candidates, reference rates, text, pair lists, and lifetime values
   are bounded before policy evaluation.
6. Policy failure, stale evidence, future observation, provider disagreement,
   signer failure, and overflow fail closed without partial issuance.
7. The issuer never persists, publishes, settles, or retries provider calls.

## Verification

- Failing-first tests cover pair policy, exact deviation boundaries, legal
  acceptance, freshness/future skew/lifetime, overflow, signer failure,
  merchant/facts binding, and no issuance on every rejection path.
- Property tests prove deviation comparison is independent of candidate input
  ordering and never accepts an overflowing product.
- The existing `fx-evidence` verifier and tax-engine replay vectors remain the
  downstream acceptance gates.
