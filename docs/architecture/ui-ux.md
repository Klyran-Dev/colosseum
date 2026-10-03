# Gold-standard UI/UX architecture

## Product experience principles

1. Explain money before asking for a signature.
2. Make tax calculation inspectable without requiring tax expertise.
3. Show the source, version, effective period, and status of every rule.
4. Separate “calculated,” “attested,” “settled,” and “filed”; never imply that settlement equals legal compliance.
5. Design every network, wallet, policy, and reconciliation failure as a recoverable state.
6. Prefer progressive disclosure: simple totals first, calculation trace on demand.
7. Use the same terminology and numeric semantics in checkout, simulator, API, and audit views.

## Personas and primary journeys

| Persona | Primary journey | Success signal |
| --- | --- | --- |
| Payer | Review amount, tax, recipient, policy status, and sign | Confident signature with no surprise deduction |
| Merchant operator | Configure products, jurisdictions, assets, splits, and tax behavior | Valid configuration with clear preview |
| Tax operator | Import/create rule, simulate, compare, review, approve, publish | Auditable rule release with no ambiguity |
| Finance/auditor | Trace settlement to calculation, policy, vault, and receipt | Reproducible evidence bundle |
| Incident operator | Detect stale policy, calculation mismatch, or vault anomaly | Fast containment without rewriting history |

## Information architecture

Every workspace has a stable, non-sensitive URL under the static SPA. Operators
can bookmark or refresh a review surface, and browser back/forward returns to
the prior workspace without placing credentials, financial facts, tenant IDs,
or authorization decisions in the URL. Unknown paths canonicalize to the quote
workspace; the NGINX edge serves the SPA fallback for direct links.

```text
Checkout
  - Quote summary
  - Tax breakdown
  - Why this tax?
  - Wallet/signature state
Merchant workspace
  - Overview
  - Products and tax categories
  - Jurisdictions and evidence
  - Tax rules
  - Split plans and assets
  - Simulations
  - Settlements and vaults
Tax operations
  - Rule library
  - Draft editor
  - Scenario builder
  - Diff and approval queue
  - Published versions
Audit
  - Calculation explorer
  - Evidence bundle
  - Settlement receipt
  - Reconciliation and exports
Incident operations
  - Replay-gap queue
  - Dead-letter review
  - Bounded projection repair
```

## Checkout UX

The primary checkout card shows:

- gross amount and currency;
- tax amount and net amount;
- tax components grouped by name and jurisdiction;
- split recipients only when the merchant chooses to disclose them;
- rule/policy version, effective timestamp, and attestation status;
- quote expiry and exact amount the wallet will authorize;
- link to “Why this tax?” and a downloadable calculation reference.

Before signing, the user sees a human-readable summary plus the exact token mint, recipient vault/addresses, network, and fee-payer policy. The UI must distinguish wallet fee from protocol fee and tax withheld.

## “Why this tax?” explanation

Use a layered explanation:

1. **Summary:** “Tax calculated for [jurisdiction] using [tax regime] at [effective version].”
2. **Inputs:** product category, customer/merchant jurisdiction facts, price basis, currency, and evidence status.
3. **Calculation:** ordered component cards showing base, rate/brackets, dependencies, credits, min/max, and rounding/truncation.
4. **Evidence:** rule digest, calculation digest, issuer, timestamps, and policy status.
5. **Limitations:** explicit note that the result is automated withholding evidence, not a legal filing determination.

## Tax rule authoring UX

The editor is a guided workflow, not a raw JSON form:

1. Select jurisdiction, tax regime, product categories, currencies, and effective window.
2. Define tax components and their dependency graph.
3. Choose calculation base: gross, net, line, invoice, prior component, or declared custom base.
4. Configure rates/brackets, thresholds, exemptions, credits, caps, floors, and rounding.
5. Run built-in boundary scenarios and compare against expected results.
6. Review a human-readable rule summary and machine-readable digest.
7. Submit for review; publish only after required approvals.

The UI must prevent invalid graphs, overlapping brackets, missing rounding semantics, ambiguous thresholds, and unpublished rule edits from entering the production registry.

## Simulator and scenario library

The simulator supports side-by-side comparison of rule versions with:

- sample transaction facts;
- edge cases generated from every bracket and threshold boundary;
- tax-on-tax dependency visualization;
- inclusive vs exclusive comparison;
- min/max and credit ordering inspection;
- rounding/truncation output at component/line/invoice/settlement scope;
- output diff, rule digest, and calculation trace.

Every saved scenario is versioned and can become a golden test vector.

## Design system and accessibility

- WCAG 2.2 AA target.
- Keyboard-first navigation and visible focus states.
- Screen-reader labels for all amounts, statuses, charts, and dependency graphs.
- Never use color alone for tax/policy status; pair color with text and icons.
- Locale-aware numbers and dates with an explicit display currency and timezone.
- RTL-compatible layout and translated explanation keys.
- Responsive layouts for mobile checkout and desktop tax operations.
- Reduced-motion support and resilient empty/loading/error states.
- Consistent status vocabulary: `Draft`, `Needs review`, `Approved`, `Published`, `Expired`, `Rejected`, `Reconciliation required`.

## Operational UX states

The UI must explicitly handle:

- quote expired before signing;
- policy missing or expired;
- tax calculation evidence unavailable;
- wallet rejected or disconnected;
- transaction submitted but status unknown;
- processed but not finalized;
- duplicate client nonce;
- RPC provider degradation;
- indexer lag or reconciliation required;
- vault release pending governance.

### Replay-gap operator workspace

The replay workspace is a system-operator surface, not a merchant dashboard.
It requires an in-memory bearer token, an explicit stream and bounded status
filter, and renders only the authenticated API projection. A dead-letter row
shows its slot range, observed finalized slot, retry count, reason, error code,
timestamps, and any immutable operator review. The operator must enter an
incident-backed note before acknowledgement or requeue; the UI labels
acknowledgement as review evidence and requeue as bounded projection repair.
After either mutation it reloads the server projection. `401`, `403`, `409`,
timeout, malformed response, and reload failure states remain visible with the
API correlation reference where available. No browser state advances a
checkpoint or claims chain finality.

Every state provides the next safe action and preserves the calculation/settlement correlation ID.

## Session and credential presentation

Protected workspaces use the authenticated shell session but do not render the
bearer credential. They show only bounded session state such as “Signed-in
session ready” or “Sign-in required”; request headers remain an in-memory
implementation detail. Local credential entry is available only through the
explicit local-development authentication control. See
[ADR-0280](../adr/0280-browser-never-renders-bearer-credentials.md).

Quote workspaces also keep idempotency references out of editable form fields.
They assign a secure in-memory retry reference automatically, keep it stable
for an unchanged request, and replace it when financial or rule facts change.
See [ADR-0281](../adr/0281-automatic-quote-retry-reference.md).

## Frontend architecture

```text
features/
  checkout/
  tax-breakdown/
  tax-rule-editor/
  simulator/
  approvals/
  settlements/
  audit/
shared/
  design-system/
  localization/
  money-formatting/
  api-contracts/
```

Use generated API/IDL types, a single money-formatting package, server-authoritative calculation results, and an optional Rust/WASM preview that is visibly labeled as a preview until the API attestation is returned.
