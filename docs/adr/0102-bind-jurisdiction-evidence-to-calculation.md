# ADR-0102: Bind jurisdiction resolution evidence to tax facts

- Status: Accepted
- Date: 2026-09-27
- Owners: Tax Platform / Compliance / Security
- Scope: Calculation replay, quote evidence, and jurisdiction-resolution binding

## Context

ADR-0101 establishes deterministic jurisdiction resolution, but a standalone
resolver result is not sufficient evidence if calculation facts can still be
replayed from an unbound jurisdiction string. A calculation result must prove
which policy and evidence envelope selected the jurisdiction used for tax-rule
selection.

The binding must remain backward compatible for historical records while
making the absence of resolution evidence explicit. It must not accept a
client-supplied digest as proof that the resolver ran.

## Decision

1. Add an optional `jurisdiction_resolution` envelope to `TaxFacts` and
   `CalculationResult`. Its presence is encoded into the canonical facts
   digest and the complete calculation result.
2. Bind the policy ID/version/digest, evidence digest, selected source/code,
   candidate count, and bounded decision trace. The tax engine never trusts a
   client digest to create this envelope; an application adapter must obtain a
   result from the resolver and persist the server-owned envelope.
3. Preserve legacy facts without an envelope for historical replay. New
   globally supported quote paths must reject absent resolution evidence at the
   application boundary before production enablement; the pure tax crate keeps
   the legacy mode so old records can be audited and migrated deliberately.
4. Replay compares the complete envelope and facts digest. Tampering with a
   policy digest, evidence digest, selected source, trace, or candidate count
   fails closed.
5. Do not put raw address/KYC/registration evidence into Solana accounts or
   calculation facts. Only bounded digests and the minimum resolution metadata
   cross this domain boundary.

## Alternatives considered

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Keep only `TaxFacts.jurisdiction` | No schema changes | Cannot prove jurisdiction provenance or replay the decision | Rejected |
| Store only a caller-supplied evidence digest | Small payload | Digest is not proof of resolver execution or selected source | Rejected |
| Store raw external evidence in every calculation | Maximum local context | PII exposure, oversized facts, retention burden, and off-chain privacy risk | Rejected |
| Re-resolve during every replay | Fresh semantics | Historical policy/provider drift changes past results and creates network dependence | Rejected |
| **Persist a bounded server-owned resolution envelope in facts/results** | Replayable, privacy-minimized, digest-bound, backward compatible | Requires application-side resolver integration and a legacy migration policy | **Selected** |

## Invariants and rollout

- A bound envelope's selected code equals `TaxFacts.jurisdiction`.
- Policy/evidence digests are non-zero and the candidate count is bounded.
- The complete envelope participates in the canonical facts digest.
- Historical records without an envelope are visibly legacy and are not
  eligible for new globally supported quotes.
- The application must verify the envelope against the authenticated policy
  and normalized candidates before persistence; this crate-level binding is
  necessary but not sufficient for legal or provider validation.

## Verification

- tax-engine tests prove canonical digest changes when any envelope field
  changes and replay rejects envelope tampering;
- JSON compatibility tests prove absent envelopes deserialize as legacy facts;
- application integration tests will prove the server-owned resolver result is
  bound before quote persistence and attestation issuance;
- production release remains blocked until jurisdiction-specific legal/tax
  approval, provider authentication, and privacy controls are evidenced.

