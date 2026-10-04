# ADR-0315: Executable tax-responsibility language boundary

## Status

Accepted — the repository now guards the minimum user-facing distinction
between calculation, attestation, settlement evidence, and legal
responsibility. Counsel, jurisdiction owners, and merchant-term approval
remain external release gates.

## Context

Tax software can create material harm when a calculated amount, an issuer
attestation, a wallet signature, and a legal filing are presented as the same
thing. The UI already has context-specific explanations, but duplicated copy
can regress silently as new workspaces are added. A public repository needs a
small executable contract that makes the boundary reviewable and keeps the
language aligned with the product's actual authority.

## Decision

Extend `scripts/verify_ui_language_contract.sh` and the component suite with
an approved language contract. Production UI source must:

- describe the product as providing automated withholding and evidence, not
  tax filing, legal advice, or a compliance guarantee;
- distinguish calculated results that still need verification from published
  rules with issuer attestation;
- describe document quotes as calculated until attested;
- state that wallet authorization is not proof of finalized settlement;
- state that adjustment records do not transfer tokens, issue refunds, file
  taxes, or establish legal compliance; and
- state that an unattested simulation cannot create a quote or authorize
  settlement.

The contract also rejects a small set of positive compliance claims that would
be misleading in production UI source. It does not attempt to approve legal
copy, determine jurisdictional responsibility, or replace counsel review.
Those remain explicit release gates.

## Alternatives considered

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Rely on merchant terms and counsel review only | Flexible wording | UI regressions can imply filing or compliance before legal review | Rejected |
| Use one global disclaimer everywhere | Easy to scan statically | Too vague for calculation, attestation, settlement, and adjustment states | Rejected |
| Duplicate page copy without a contract | Local context is convenient | Drift, inconsistent claims, and weak review diagnostics | Rejected |
| **Executable contract plus context-specific copy and tests** | Reviewable, fail-closed, preserves plain language and progressive disclosure | Requires updating approved phrases when product semantics change | **Selected** |

## Invariants and failure policy

- No production UI state may imply that withholding evidence is a filing,
  legal advice, or guaranteed compliance.
- Calculated, attested, settled/finalized, and filed are distinct states; one
  must not silently upgrade another.
- A wallet signature is authorization evidence, not finalized network proof.
- UI copy never grants tax, settlement, or legal authority.
- Missing approved copy or a prohibited positive claim fails the check; it is
  never downgraded to a warning.
- Legal responsibility, rule ownership, provenance, and jurisdiction scope
  remain governed data and external approval responsibilities.

## Verification

- `apps/web/src/productShell-ui.test.tsx` proves the shell's explicit
  withholding/evidence boundary.
- `scripts/verify_ui_language_contract.sh` checks context-specific copy and
  rejects positive compliance claims.
- `make ui-language-contract`, `make web-check`, and the full repository check
  are required before release evidence can claim this boundary.
- Manual counsel, merchant-terms, localization, and assistive-technology
  review remain required before production launch.

## References

- [ADR-0016: Legal and product readiness](0016-legal-and-product-readiness.md)
- [ADR-0271: Executable UI language contract](0271-executable-ui-language-contract.md)
- [ADR-0282: Plain-language progressive disclosure UI](0282-plain-language-progressive-disclosure-ui.md)
- [ADR-0312: Outcome-first document payment language](0312-document-payment-outcome-first-language.md)
