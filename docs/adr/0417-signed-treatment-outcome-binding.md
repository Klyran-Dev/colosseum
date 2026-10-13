# ADR-0417 — Signed treatment-outcome binding

- Status: Accepted — implementation gate
- Date: 2026-10-06
- Decision owners: Tax Engine, Policy, Solana Settlement, Security

## Context

ADR-0416 makes applicability treatment explicit in the pure Tax Engine. The
calculation result now distinguishes arithmetic tax from collected,
self-assessed, and withheld channels. The current calculation and document
attestation payloads, however, sign gross, settlement tax, and net only. The
applicability attestation signs the classification and evidence but not the
numeric treatment outcome.

That distinction is unsafe for a Tax Engine evidence product: an external issuer
could sign a payload whose settlement amounts are conserved while the
self-assessed or withheld amount shown in the explanation is not part of the
signed authority. A replay can detect the discrepancy off-chain, but the
attestation itself is not yet a complete commitment to the displayed outcome.

## Decision

Before production enablement of any non-`Taxable` treatment, introduce a
versioned calculation-attestation v2 for aggregate and document calculations.
The signed payload must include a canonical `treatment_outcome_digest` over:

- treatment enum and semantic version;
- arithmetic tax, collected tax, self-assessed tax, withheld tax, and taxable
  base as signed decimal atomic amounts; and
- a domain separator distinct from the existing calculation payload.

The API must derive the expected digest from the server-owned calculation
result and verify it before persistence or settlement compilation. The
attestation payload digest remains the on-chain policy digest, so the existing
policy account and settlement instruction do not need an incompatible wire
layout change. Legacy v1 attestations remain readable for historical evidence,
but settlement submission must fail closed when a non-`Taxable` result is not
covered by v2 evidence. A v2 `Taxable` result is also preferred and becomes
mandatory when the treatment-outcome card is rendered as authoritative
evidence.

The implementation must preserve the existing v1 canonical vector and support
dual-read/explicit-version verification during rollout. It must add aggregate
and document golden vectors, tamper tests for every channel, API binding tests,
and a settlement-client test proving that the policy digest is the digest of
the signed v2 payload.

## Alternatives considered

1. **Leave the current payload unchanged.** Rejected: the numeric treatment
   channels remain unsigned evidence.
2. **Add the raw channels to the existing v1 payload.** Rejected: changing
   canonical bytes without a schema/version boundary breaks replay and issuer
   interoperability.
3. **Create a second on-chain outcome PDA/instruction.** Rejected: it adds
   account and transaction surface without improving issuer authenticity, and
   risks a settlement succeeding without the second binding.
4. **Put the complete rule graph on Solana.** Rejected: it violates bounded
   compute, deterministic account sizing, and low-latency settlement goals.
5. **Versioned signed outcome digest bound through the existing policy digest.**
   Selected: preserves deployed legacy layouts, makes the issuer commitment
   explicit, keeps settlement hot-path work bounded, and gives audit/replay a
   canonical cross-language contract.

## Consequences

- Non-taxable treatment support is locally implemented in the pure engine, but
  production settlement support remains gated until this ADR's v2 evidence is
  implemented and audited.
- The UI may show calculated channels as explanatory output, but must not call
  them signed or settlement-authoritative until the v2 digest is present.
- Issuers and fixtures need a controlled v1-to-v2 migration plan; no silent
  reinterpretation of historical v1 payloads is permitted.
- This ADR is a release blocker, not a reason to weaken the treatment engine
  or to invent legal semantics.
