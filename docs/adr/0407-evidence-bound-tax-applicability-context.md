# ADR-0407: Evidence-bound tax applicability context

- Status: Accepted
- Date: 2026-10-06
- Deciders: Principal Low-Latency Architecture, Tax Domain, Compliance, Security
- Scope: Product classification, taxpayer status, taxability, and rule-selection context

## Context

The arithmetic Tax Engine is intentionally strong and bounded: it evaluates
exact rates, brackets, dependencies, thresholds, exemptions, clamps, credits,
rounding, truncation, and residual allocation. The current quote boundary also
has a server-verified jurisdiction-resolution envelope. Those concerns must not
be confused with tax applicability.

Global tax treatment additionally depends on facts such as product or service
category, supply type, customer and merchant classification, registration
status, reverse-charge or withholding treatment, and evidence effective at the
transaction time. The current `regime` selector is a rule-release identity; it
is not a sufficient product-category or applicability model. Treating a free
form UI value as either would make the calculation appear more global than its
evidence supports.

## Decision

Create a separate **Tax Applicability and Classification** bounded context.
It will resolve a bounded, versioned `TaxApplicabilityContext` before the
arithmetic engine runs. The context owns:

- product/service category and supply type;
- merchant registration and collection status;
- customer classification and registration evidence where legally permitted;
- transaction treatment such as taxable, exempt, zero-rated, reverse-charge,
  withholding, out-of-scope, or unresolved;
- required facts, evidence references, effective windows, provenance, and
  accountable tax owner; and
- a deterministic decision trace and signed applicability attestation.

The context is an evidence-bound decision, not a legal opinion. It must never
infer a taxpayer status from a wallet address, amount, locale, or browser input.
An unresolved or stale applicability decision fails closed or enters an
explicit merchant-review state; it never silently produces a zero tax result.

The attestation will bind:

```text
merchant identity
pre-resolution facts digest
jurisdiction-resolution digest
product category / supply type
applicability policy ID and version
decision and treatment
evidence digest and validity window
engine-compatible rule release
```

The arithmetic engine will consume the verified context as an immutable input.
It will continue to own only deterministic amount evaluation. A taxable or
withholding treatment may evaluate the rule graph; an explicitly approved
zero-rated or exempt treatment may produce a zero component with a trace; an
unresolved treatment is rejected. Reverse-charge and reporting-only outcomes
remain distinct from a client-visible zero tax amount.

The hot path remains bounded: the API resolves the context from an immutable,
tenant-scoped registry and verified evidence before entering the pure engine.
No tax SaaS call, geocoder, database scan, or arbitrary predicate graph is
allowed inside the calculation function. Policies are compiled into bounded
immutable decision graphs with explicit maximum nodes, facts, and evidence
references. The provider and legal/tax owner remain deployment decisions, not
hard-coded vendor assumptions.

### Current implementation slice

The repository now contains the bounded `tax-applicability` domain crate and
an additive Tax Engine binding. Aggregate and document facts may carry an
optional `ApplicabilityDecision`; the decision is validated, bound to the
pre-applicability canonical facts digest, included in the final facts digest,
echoed in calculation results, and checked during replay. Historical facts
without the field retain their existing digest. The calculation boundary
currently permits only `Taxable`; all other treatments, including
`Withholding`, are rejected with typed fail-closed errors until their distinct
arithmetic and reporting semantics are approved.

The policy boundary now also contains the first signed
`ApplicabilityAttestation` contract. Its canonical domain binds the full
applicability-facts digest, decision digest, jurisdiction-resolution digest,
evidence validity, merchant identity, rule release, issuer key, sequence, and
validity window. The signer cannot issue an attestation whose lifetime exceeds
the source evidence window. This is a reusable verification kernel only: no
HTTP, PostgreSQL, quote, or settlement route accepts caller-authored
applicability yet.

The existing `regime` field remains the published rule-release selector for
backward compatibility. It must not be renamed to or treated as product
category. New production quote contracts will carry an explicit applicability
context reference and attestation; historical calculations remain replayable
under their existing schema and are never silently reinterpreted.

## Alternatives considered

| Alternative | Benefits | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Add arbitrary applicability fields directly to `TaxFacts` | Small apparent change | Mixes legal classification with arithmetic facts, expands canonical digest ad hoc, and makes evidence ownership unclear | Rejected |
| Overload `regime` as product category | No schema change | Confuses tax regime, product category, and treatment; prevents reliable global coverage claims | Rejected |
| Encode applicability in an unbounded rule-expression language | Maximum apparent flexibility | Non-deterministic complexity, unsafe latency, difficult legal review, and poor replay guarantees | Rejected |
| Delegate all applicability to an external tax SaaS | Broad initial coverage | Hot-path dependency, opaque policy/versioning, provider lock-in, and weak replay authority | Rejected |
| **Separate evidence-bound applicability context with bounded policy graph** | **Clear ownership, deterministic replay, explicit legal scope, bounded latency, and compatible arithmetic engine** | **Requires new schema, evidence governance, and jurisdiction-specific release work** | **Selected** |

## Invariants

- A calculation cannot claim global tax applicability from a jurisdiction code or
  free-form category alone.
- Every non-historical production calculation binds a verified applicability
  context, policy digest, evidence digest, effective window, and accountable
  owner.
- `taxable`, `zero_rated`, `exempt`, `reverse_charge`, `withholding`,
  `out_of_scope`, and `unresolved` are distinct outcomes in the trace and API;
  none is inferred from a zero arithmetic result.
- Applicability policy changes create new immutable versions and new evidence;
  they never mutate a published calculation.
- The browser may select a governed reference, but it cannot author treatment,
  evidence, merchant identity, registration status, or a tax result.
- The arithmetic engine remains pure and independent of HTTP, storage, clocks,
  RPC, wallets, and external providers.
- Solana settlement accepts only an attestation whose treatment, rule digest,
  amount bounds, merchant, asset, and expiry match the settlement intent.

## Implementation and verification plan

1. Define versioned `TaxApplicabilityContext` and attestation schemas with
   canonical serialization and decimal-string-safe browser representations.
2. Add a pure Rust applicability crate with bounded predicates, explicit
   decision outcomes, cycle/size/evidence validation, replay, and property tests.
3. Add PostgreSQL 18 tenant-scoped immutable policy/evidence projections with
   UUIDv7 identifiers, effective-window constraints, and append-only events.
4. Extend quote and document-quote contracts additively with a server-owned
   applicability reference; reject client-supplied treatment and evidence
   claims. Preserve historical replay adapters.
5. Add golden vectors for taxable, exempt, zero-rated, reverse-charge,
   withholding, out-of-scope, unresolved, stale-evidence, and conflicting-fact
   outcomes, including tax-on-tax and document residual interactions.
6. Add API, storage, attestation, settlement, UI progressive-disclosure, and
   accessibility tests before enabling any production route.
7. Publish a jurisdiction/product coverage record only after legal/tax owner
   review, provenance, effective-window evidence, and staging rehearsal.

This ADR freezes the boundary and implementation sequence. It does not claim
that any jurisdiction, product category, provider, or treatment is currently
production-supported.
