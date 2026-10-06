# ADR-0380: Policy issuer authority and asset-dependency evidence boundary

- Status: Accepted
- Date: 2026-10-05
- Deciders: Principal Low-Latency Blockchain Architecture, Settlement Security, Finance/Legal, and Product
- Scope: Policy attestations, stablecoin admission, route activation, dependency disclosure, and release evidence

## Context

The platform has several different authorities that can be confused when a
tax rule, a stablecoin route, or a settlement artifact is reviewed. A policy
issuer can attest to a versioned policy; it cannot upgrade the Solana program,
change a merchant route, or establish that an asset is legally redeemable.
Likewise, an enabled mint-scoped `AssetConfig` proves protocol admission
under the governed asset policy; it does not prove price stability, issuer
solvency, sanctions clearance, redemption availability, or tax compliance.

The implementation already separates these concerns in the Solana program,
server-owned registries, and dependency register. The missing decision was a
single evidence contract that makes the ownership and limitation visible to
operators, reviewers, release automation, and public documentation.

## Decision

Use separate typed authorities and an explicit dependency state for every
policy or asset route. No authority may be inferred from a database row, a
provider response, a UI label, or a successful quote alone.

### Authority matrix

| Authority | May do | May not do | Required evidence |
| --- | --- | --- | --- |
| Program upgrade governance | Approve a reviewed program binary and upgrade window | Issue tax policy, activate a merchant route, or move funds directly | Multisig/timelock decision, binary digest, release manifest, finalized program evidence |
| Policy issuer registry governance | Register, rotate, suspend, and revoke bounded policy issuer identities | Change rule meaning outside a versioned rule release or bypass merchant scope | Registry PDA/config digest, four-eyes transition, issuer key identity, effective window, revocation evidence |
| Policy issuer signer | Sign a canonical policy/calculation attestation within its registered scope | Register itself, alter facts, alter a rule digest, or authorize token movement | HSM/KMS key reference, signed canonical bytes, key version, merchant/asset/rule binding, expiry |
| Merchant configuration governance | Approve tenant-scoped route, destination, and product configuration | Override the Solana program, issuer registry, or finalized chain evidence | Authenticated principal, tenant scope, four-eyes record where required, route digest, activation evidence |
| Asset admission governance | Classify and enable a specific mint/program/decimal tuple as an allowed protocol asset | Claim redemption, solvency, price stability, sanctions, or tax status | Mint-scoped `AssetConfig`, finalized route verification, asset-policy review, suspension history |
| API/compiler/indexer | Prepare, verify, persist, and project bounded artifacts | Become a signer, source of funds, or chain authority | Tenant-scoped request, immutable artifact digest, signed bytes, finalized chain evidence |
| Stablecoin issuer/off-ramp | Supply external redemption, liquidity, and fiat services under its own terms | Authorize Solana settlement or tax policy | Contract/provider record, service region, freshness/status evidence, legal and finance review |

The Solana program remains the final authority for token movement. The policy
issuer, API, database, cache, event stream, and external issuer are all
non-authoritative for movement unless their evidence is checked by the
program's bounded account and attestation rules.

### Evidence envelope

Every active policy or asset dependency projection must carry a bounded,
versioned evidence envelope containing:

- `schema` and `evidence_type`;
- tenant/merchant scope where applicable;
- the policy, rule, asset, mint, and token-program identities;
- canonical content or configuration digest;
- source/provenance reference and accountable owner;
- effective start/end and observed-at timestamps;
- signer/registry key identity and key version where signatures apply;
- activation, suspension, revocation, or unknown state;
- a reference to finalized chain evidence when on-chain state is required;
- review status and next review date.

Raw credentials, private keys, seed phrases, unnecessary PII, and unbounded
provider payloads are never part of the envelope. Historical envelopes are
immutable; corrections are new versions with supersession links.

### Dependency state and product behavior

The bounded dependency state is one of `configured`, `verified`, `active`,
`suspended`, `revoked`, or `unknown`. State transitions are server-owned,
append-only, and evidence-bound. `unknown` is not equivalent to `active`.

- `configured` may be reviewed but cannot authorize signing or movement.
- `verified` has passed the required topology/provenance checks but is not
  necessarily enabled for traffic.
- `active` may participate in its explicitly approved path.
- `suspended` and `revoked` fail closed for new work without rewriting history.
- `unknown` stops affected quotes, signing, route activation, or release work
  according to the dependency's safety policy.

The UI must name the dependency and its state in plain language, show the
last verified time and evidence reference, and explain the next safe action.
It must not display “compliant,” “redeemable,” “guaranteed,” or “available”
when the evidence only proves protocol admission or configuration.

## Invariants and failure policy

- A policy attestation is accepted only when issuer identity, merchant, asset,
  rule/policy digest, effective window, sequence, and expiry match the
  server-owned and on-chain scope.
- A stablecoin `AssetConfig` proves only the bounded protocol admission tuple;
  external issuer and off-ramp claims remain separate evidence domains.
- No database, cache, event, provider, or UI state can promote an unverified
  or unknown dependency to an active movement authority.
- Suspension/revocation blocks new affected work and preserves finalized
  receipts, historical evidence, and replayability.
- Evidence digests and key identities are canonical and bounded; no raw
  external payload is inserted into a settlement instruction or unbounded
  metric label.
- State evaluation performs bounded in-memory checks on the quote/compile hot
  path. External provider calls, legal review, route verification, and
  backfills are asynchronous or administrative workflows.
- Every denial or stop state has a UUIDv7 correlation/reference identifier and
  an auditable reason, without exposing secrets or sensitive evidence.

## Alternatives considered

| Alternative | Authority clarity | Latency and safety | Auditability | Decision |
| --- | --- | --- | --- | --- |
| One global governance authority for policy, assets, upgrades, and custody | Low; privileges become conflated | Small apparent control plane but high blast radius | Difficult to attribute decisions | Rejected |
| Database dependency rows as the authority | Low; projections can be stale or tampered with | Fast reads but unsafe activation | Weak chain and signer provenance | Rejected |
| External provider metadata as proof of stablecoin or tax status | Low; provider scope and legal meaning vary | Adds network dependency to hot paths | Non-deterministic and hard to replay | Rejected |
| One generic untyped evidence JSON for every dependency | Partial; flexible but ambiguous | Easy to ingest, difficult to validate safely | Weak schema evolution and review | Rejected |
| **Separate typed authorities plus a bounded versioned evidence envelope and dependency state** | **High** | **Bounded hot path; fail-closed unknown state** | **Canonical, replayable, and reviewable** | **Selected** |

## Operational and release consequences

This decision does not select a provider, stablecoin issuer, jurisdiction, or
legal owner. Those decisions remain in the decision register and require
contract, residency, custody, legal, finance, and SLO evidence before
activation. It does make the missing evidence explicit and prevents an
operator or reviewer from mistaking protocol admission for external
redemption, legal, or tax-compliance approval.

Provider replacement requires a new typed binding, contract vectors, security
review, retention review, failover/restore drill, and a new active evidence
envelope. Key rotation creates a new key version and overlap/revocation
evidence; it does not mutate historical signatures. Asset suspension is an
operational containment action and must not fabricate refunds or alter
finalized balances.

## Verification

- ADR-0004, ADR-0034, ADR-0320, ADR-0321, ADR-0325, and ADR-0316 provide the
  policy, issuer, asset, route, and rule-provenance implementation contracts.
- `docs/operations/dependency-register.md` records the provider-neutral exit
  paths and accountable roles.
- Rust/API/storage tests verify scope binding, canonical digests, immutable
  evidence, fail-closed unknown states, and tenant isolation.
- `make integration-check`, `make public-docs-check`, and the 48-journey
  browser gate provide current local evidence for the implemented boundaries.
- Concrete issuer/provider selection, legal approval, HSM/KMS ceremony,
  production residency/SLO evidence, and Linux validator execution remain
  external release gates.

## References

- [ADR-0004: Signed policy attestations](0004-signed-policy-attestations.md)
- [ADR-0034: Governed policy issuer registry](0034-governed-policy-issuer-registry.md)
- [ADR-0316: Versioned rule-release provenance](0316-versioned-rule-release-provenance.md)
- [ADR-0320: On-chain stablecoin asset admission](0320-on-chain-stablecoin-asset-admission.md)
- [External dependency register](../operations/dependency-register.md)
- [Decision register](../operations/decision-register.md)
