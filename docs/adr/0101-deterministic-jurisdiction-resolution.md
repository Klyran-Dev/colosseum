# ADR-0101: Deterministic jurisdiction-resolution bounded context

- Status: Accepted
- Date: 2026-09-27
- Owners: Tax Platform / Compliance / Security
- Scope: Jurisdiction and place-of-supply resolution before tax-rule selection

## Context

The tax engine must not treat a caller-provided jurisdiction string as
an authority boundary. Global commerce can provide multiple jurisdiction
signals—merchant registration, customer address, shipping destination, and
other verified evidence—with different legal precedence by product,
transaction type, and jurisdiction. A generic tax engine cannot safely infer
those legal rules from an address or provider confidence score.

The system therefore needs a bounded, replayable resolution context between
fact normalization and tax-rule selection. It must make the policy and all
evidence that influenced the decision explicit, reject ambiguity, and remain
usable in a low-latency hot path after policy compilation.

## Decision

1. Add `crates/jurisdiction` as a pure Rust domain bounded context. It has no
   HTTP, database, clock, RPC, wallet, or provider dependencies.
2. Represent evidence as typed candidates with a jurisdiction code, declared
   evidence source, and non-zero evidence digest. Provider adapters normalize
   and authenticate evidence before invoking this context.
3. Resolve candidates using an immutable, versioned `ResolutionPolicy` whose
   ordered source list is the only source of precedence. Caller-supplied
   confidence, array order, and provider preference are never precedence.
4. Support two explicit modes: `HighestPrecedence`, which selects the highest
   configured source and rejects disagreement at that rank, and
   `RequireAgreement`, which rejects any disagreement among supplied
   candidates.
5. Return a `JurisdictionResolution` containing the selected code/source,
   policy digest, evidence digest, and bounded decision trace. The result is
   immutable evidence and is required input for a production global tax quote.
6. Reject missing candidates, unknown/unconfigured sources, duplicate policy
   entries, duplicate evidence digests, zero digests, invalid codes, and
   ambiguous results. No fallback jurisdiction is permitted.
7. Keep legal coverage outside the generic resolver. Each supported
   jurisdiction/regime must publish a reviewed policy version, evidence
   contract, effective window, provenance, and tax-owner approval. A resolved
   jurisdiction is not a representation that filing, reporting, or legal
   compliance has been completed.
8. The API/application layer owns provider authentication, tenant scope,
   effective-window checks, and persistence. It must bind the resulting
   resolution digest into normalized tax facts and calculation attestations;
   this first slice establishes the domain contract and golden vectors before
   wiring live provider adapters.

## Alternatives considered

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Accept `jurisdiction` from the checkout caller | Minimal latency and API surface | Caller can substitute policy; no evidence or replay provenance | Rejected |
| Resolve in each provider adapter | Local flexibility | Duplicated precedence semantics, inconsistent conflict behavior, weak replay | Rejected |
| External tax SaaS as the authority | Broad apparent coverage | Opaque rules, provider outage/latency, weak protocol binding, vendor lock-in | Rejected |
| Put address/rule resolution on Solana | Strong settlement visibility | Unbounded data, privacy exposure, changing law, infeasible account/compute model | Rejected |
| **Pure Rust policy-driven resolver over authenticated evidence** | Deterministic, low latency after compilation, replayable, provider-agnostic, fail-closed | Requires reviewed policy/evidence governance and jurisdiction-specific adapters | **Selected** |

## Invariants

- A successful resolution has exactly one selected jurisdiction and source.
- Every selected result is bound to one policy digest and one evidence digest.
- Equal-precedence disagreement is never silently resolved.
- Policy and candidate order cannot change a digest or decision without changing
  the canonical inputs.
- No result is produced without at least one valid evidence candidate.
- The resolver never performs network I/O or consults the system clock.

## Compatibility and rollout

The crate is additive and initially ships behind the application boundary. The
existing legacy `TaxFacts.jurisdiction` field remains readable for replay of
historical records, but new globally supported quotes must carry a successful
resolution evidence envelope before production enablement. The API integration
will be a subsequent vertical slice with a versioned request schema and
storage migration; it must not silently treat legacy jurisdiction strings as
resolved evidence.

## Verification

- domain tests cover deterministic selection, same-rank conflict, strict
  agreement conflict, unknown source, duplicate evidence, malformed policy,
  zero digest, and replay/digest tampering;
- versioned golden vectors cover common precedence and failure cases;
- property tests prove permutation stability for candidate order and policy
  rejection for duplicate sources;
- integration tests will bind the resolution digest into persisted normalized
  facts and attestation payloads before the global-jurisdiction release gate is
  considered complete.

