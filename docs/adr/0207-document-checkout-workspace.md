# ADR-0207: Document-bound checkout workspace

- Status: Accepted for implementation
- Date: 2026-09-30
- Owners: Web Platform, Tax Platform, Settlement Protocol, Security

## Context

The document quote, document attestation, and `settle_document` compiler are
now separate authority boundaries, but the SPA only exposes aggregate checkout.
Operators would otherwise have to use an undocumented HTTP client, increasing
the risk of submitting the wrong attestation, scope, policy digest, or route.
The UI must make the evidence chain visible without inventing document totals
or allowing the browser to become a tax or account authority.

## Decision

Add a dedicated lazy React document-checkout workspace. It accepts a persisted
document calculation ID, an externally signed `DocumentCalculationAttestation`
JSON document, an active route, a policy digest, a wallet nonce, and an
explicit full-document or complete-line-subset scope. The workspace:

1. verifies the attestation through the authenticated document-only API route;
2. compiles only through `/v1/settlement/document/compile-from-quote`;
3. parses and validates the typed server artifact before showing a wallet
   action;
4. signs the unchanged server artifact using the existing wallet boundary; and
5. reports durable intent, unknown-provider, retry, and reconciliation states.

The browser never accepts merchant-side token accounts, amounts, document
line amounts, or a private issuer key. A pasted attestation is displayed as
untrusted until the server returns `document_attested_unsettled`. The UI never
labels withholding as filing, reporting, legal compliance, or tax advice.

## Alternatives considered

| Alternative | Advantages | Rejection reason |
| --- | --- | --- |
| Extend aggregate checkout with a document mode | Reuses layout | Blurs aggregate/document authority and makes accidental aggregate compilation easier. |
| Build a separate operator-only HTTP tool | Small SPA change | Not a supportable product surface; poor accessibility and high operator error risk. |
| Let the browser calculate or sign the attestation | Fast interaction | Violates issuer custody, server-owned evidence, and deterministic authority boundaries. |
| **Dedicated document workspace over existing typed boundaries** | Explicit type firewall, evidence-first UX, shared wallet/reconciliation controls | Additional UI surface and test coverage; selected. |

## Invariants and budgets

- No wallet signature is enabled before server attestation success and durable
  document-payment compilation.
- The API remains authoritative for amounts, route accounts, attestation
  validity, scope digest, intent state, and chain outcome.
- UUIDv7 IDs, `u64` counters, and atomic amounts remain canonical strings at
  the browser boundary; policy and evidence digests are fixed 32-byte values.
- Every async action has loading, timeout, error, expired, unknown, retry, and
  reconciliation states. No polling loop is unbounded.
- The workspace is code-split and must not add work to the initial quote path;
  it uses the existing checkout transaction builder and WCAG 2.2 AA shell.
- The pinned real-browser journeys cover unavailable document quote and active
  route APIs; both fail closed, keep compilation/signing unavailable, and pass
  WCAG 2.2 A/AA axe scans. A success journey also proves the authenticated
  evidence-to-checkout handoff, server-owned authorization rendering, and
  signing availability only after attestation verification and durable
  compilation. Its API and wallet provider are synthetic test fixtures; live
  wallet, provider, and finalized-chain evidence remain release gates.

## Consequences

Document payment becomes discoverable and supportable as a Tax Engine workflow,
while the attestation issuer and Solana finality boundaries remain explicit.
The workspace does not broaden document semantics: quantity payment, partial
line re-rating, multi-document cart aggregation, and legal filing workflows
remain separate contracts.
