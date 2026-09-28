# ADR-0103: Server-verified jurisdiction evidence at the quote boundary

- Status: Accepted
- Date: 2026-09-27
- Owners: Tax Platform / Compliance / Security / Data
- Scope: Provider evidence, jurisdiction policy lookup, and production quote calculation

## Context

The pure jurisdiction resolver intentionally accepts normalized candidates and
does not authenticate addresses, registrations, or provider responses. Passing
those candidates directly from a browser or merchant API caller would make an
evidence digest a bearer assertion and would violate the fail-closed authority
boundary.

The quote path also needs a low-latency, immutable policy lookup. Resolving a
policy from an unbounded external service in the settlement-adjacent quote path
would add availability and latency risk. A published tax-rule release already
defines the exact legal/tax regime and effective window selected for a quote;
the jurisdiction policy must be tenant-owned and bound to that release.

## Decision

1. Add a signed `JurisdictionEvidenceEnvelope` contract to the jurisdiction
   bounded context. It binds provider key ID, merchant identity digest, the
   canonical pre-resolution facts digest, jurisdiction/source, evidence digest,
   and an explicit bounded validity window.
2. The API accepts signed envelopes, never raw `JurisdictionCandidate` values.
   It verifies the envelope signature against a configured provider-key
   registry, merchant binding, facts binding, validity window, and evidence
   shape before deriving candidates. Provider keys are separate from API JWT
   keys and are rotated through an explicit registry change.
3. Persist a tenant-owned, immutable `jurisdiction_policies` row for each
   published rule release. The row stores the canonical `ResolutionPolicy`,
   digest, effective window, provenance, and review status. Quote calculation
   loads the policy in the same PostgreSQL transaction as the rule release and
   rejects a missing, stale, cross-tenant, or digest-mismatched policy.
4. Resolve verified candidates with the stored policy, require the selected
   jurisdiction to equal the authenticated quote selector and facts
   jurisdiction, and call `calculate_with_jurisdiction_resolution` before
   persisting facts/result evidence.
5. Include the signed evidence envelopes' bounded digests and the resulting
   policy/evidence resolution envelope in the idempotency request digest and
   persisted calculation facts. Raw address/KYC/registration payloads remain
   outside this quote contract.
6. Local development uses an isolated synthetic provider fixture and synthetic
   keys only. Production requires HSM/KMS-backed provider signing, registry
   governance, rotation/revocation, privacy review, and jurisdiction-specific
   legal/tax approval.

## Alternatives considered

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Accept raw candidates/digests from the caller | Minimal implementation and latency | Caller can forge source/evidence; no cryptographic provenance | Rejected |
| Resolve synchronously through a geocoder/tax SaaS | Broad provider coverage | Hot-path dependency, opaque precedence, network latency, outages, replay drift | Rejected |
| Put policy in Redis or process configuration | Fast lookup | Non-authoritative, weak auditability, unsafe tenant/config drift | Rejected |
| Global policy table independent of tax-rule releases | Reusable policies | Additional selection join and risk of policy/rule effective-window skew | Rejected |
| **Signed provider evidence plus immutable tenant policy bound to each published rule release** | Authenticated, replayable, bounded hot path, tenant-safe, effective-window aligned | Requires provider-key governance and explicit policy lifecycle | **Selected** |

## Invariants

- An accepted evidence envelope is signed by a configured provider key and is
  bound to the authenticated merchant and the exact pre-resolution facts
  digest.
- The provider key registry is not the API JWT registry and unknown/revoked
  keys fail closed.
- A quote cannot use a policy from another merchant or rule release.
- The selected resolution, requested jurisdiction, and facts jurisdiction are
  identical after canonical normalization.
- A policy/evidence change changes the idempotency digest and cannot replay a
  different calculation under the same key.
- No raw sensitive jurisdiction evidence is written to PostgreSQL or Solana by
  this boundary.

## Rollout and verification

The first slice adds the signed domain contract, policy schema, server-side
verification port, and synthetic fixture. The integration harness must prove
valid evidence, wrong-merchant/facts binding, unknown provider key, expired
evidence, policy digest tampering, cross-tenant policy lookup, idempotent
replay, and selected-jurisdiction mismatch. Live provider/HSM/KMS rotation,
privacy controls, legal/tax approval, and production coverage remain release
gates.

