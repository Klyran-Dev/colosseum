# ADR-0249: FX provider identity allowlist

## Status

Accepted for implementation.

## Context

ADR-0246 defined a vendor-neutral FX source port and ADR-0247/0248 made policy
selection immutable and registry-bound. The policy still did not identify the
provider identities that were authorized for a given policy. A caller or
adapter could therefore present a well-formed quote from an unapproved vendor
and receive signed evidence if all other numerical checks passed.

## Decision

Add a bounded, exact-match `allowed_providers` list to every
`FxProviderPolicy`. Provider identifiers are non-empty bounded strings and
must be unique within the policy. Candidate issuance fails closed unless the
candidate's provider identifier exactly matches one entry in the active
policy. The list is policy data, not a caller-controlled request field.

The initial bound is 16 provider identifiers. This is deliberately separate
from provider quorum or routing policy: availability, regional routing,
quorum, and commercial/legal ownership require a later decision with concrete
provider evidence. A provider that is not listed cannot issue evidence merely
because its signature or quote arithmetic is valid.

## Alternatives considered

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Trust the candidate provider string | No policy change | Any source can claim an accepted identity | Rejected |
| Global process-wide provider allowlist | Simple runtime lookup | Cannot express jurisdiction/product-specific approval and weakens replay | Rejected |
| Caller-supplied provider authorization | Flexible | Moves authority to an untrusted request boundary | Rejected |
| **Exact provider allowlist inside the immutable policy snapshot** | Tenant/policy scoped, replayable, bounded, and fail-closed | Requires governed policy updates for provider rotation | **Selected** |

## Invariants and failure policy

1. Every published policy contains at least one and at most 16 provider
   identifiers.
2. Provider identifiers are bounded, non-empty, and unique within a policy.
3. Candidate provider identity matching is exact and case-sensitive; aliases
   require a new governed policy entry.
4. An unapproved provider fails before signing and before evidence issuance.
5. Provider allowlisting does not claim provider availability, rate legality,
   regional coverage, quorum, or HSM/KMS custody.

## Verification

- Failing-first tests cover an unapproved provider, empty/duplicate allowlists,
  and bounded provider identifiers.
- Existing exact arithmetic, freshness, legal acceptance, signing, registry,
  and replay vectors remain required.
- Provider rotation requires a new immutable policy version and activation
  evidence; in-place mutation is not permitted.
