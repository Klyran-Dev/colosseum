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

## Production navigation contract

The application uses one stable operations shell with four task-oriented
groups. A user can open any workspace directly, refresh it, and return to it
with browser history without placing account, payment, or authorization data
in the URL.

| Navigation group | Workspaces | Primary outcome |
| --- | --- | --- |
| Daily work | Tax quotes, Tax simulator, Invoice tax, Payment review, Document payment | Review and prepare a customer transaction |
| Rules & access | Tax rules, Access & approvals | Govern tax behavior and understand who may act |
| Money movement | Treasury, Balance checks, Partner payouts, Adjustments, Refund requests, Refund cancellations, Refund payments | Control, reconcile, and review money movement |
| Records & issues | Exceptions & recovery | Investigate an exception and perform a bounded recovery action |

Navigation is capability-aware but never pretends to be the security boundary.
The shell shows the same information architecture to authorized and
unauthorized users, while each entry carries a clear state such as
“Available”, “Not available”, “Needs access check”, or “Sign in to check”.
The server returns the authoritative RBAC/PBAC/ABAC capability snapshot, and
the API repeats the decision for every protected request. A denied workspace
explains the safe next step; it is not silently removed, which prevents a
reviewer from confusing missing access with missing product functionality.

The shell uses native Material 3 Expressive disclosure groups to keep the
current task prominent without hiding production capability. Daily work is
open by default; lower-frequency groups remain one keyboard action away, and
the active group opens automatically for direct links and route changes. Group
state is presentation-only: it is never an authorization decision. The
Material baseline role palette (ADR-0422) is the only production palette, so
navigation does not add per-item colors or badges that compete with financial
and error semantics. See ADR-0323 for the composition decision and alternatives.

Every workspace follows the same review rhythm:

1. **Orient:** account, environment, network, service health, and current task.
2. **Review:** human-readable facts, amounts, status, expiry, and responsibility.
3. **Inspect:** “Why this tax?”, “Record details”, or “Technical details” only when needed.
4. **Act:** one clearly named next action with approval requirements beside it.
5. **Confirm:** server response, evidence reference, and the next safe action.

Protection context is intentionally compact. The page heading carries the
current access result and one closed “Protection details” disclosure explains
that decisions are recorded, sensitive actions may require approval, and
wallets remain with their owners. Runtime, network, service, and sign-in
diagnostics live in the separate system-status disclosure; account identity and
sign-out live in the account menu. The shell does not repeat these assurances
as a full-width banner or use them as a substitute for server authorization.

Sensitive actions use progressive disclosure and four-eyes language. For
example, Partner payouts begins with active plans, available balance, partner,
and limits; creation is labelled “Create a payout plan”; approval is separate;
and “Prepare verified release” explicitly states that it reserves an amount but
does not transfer money. Chain slots, fingerprints, account keys, and exact
serialization remain in the advanced record view.

Partner payout setup has a deliberate responsibility sequence:

1. Name the payout preparer by their tenant sign-in identity.
2. Show that a second authorized person must approve the plan.
3. Ask the named preparer to connect a wallet only in the separate wallet
   authority workspace, after the service presents a one-time signed message.
4. Show the connected wallet as “Verified for payout preparation” only after
   the server confirms the tenant, role, signature, expiry, and active binding.
5. Keep the wallet address, route binding, and exact instruction under
   “Record details”; never ask an operator to paste a merchant-side account
   topology into a payout form.

The UI may explain the responsibility in plain language, but the server is
always authoritative for RBAC, policy conditions, tenant scope, wallet proof,
and final chain status. A subject change creates a new governed plan or an
explicit migration decision; it never silently changes an active plan’s signer
topology.

The shell must preserve these states for every async action: loading, empty,
success, rejected, timed out, retrying, cancelled, stale, and reconciliation
required. A successful API response never becomes a claim that a wallet
signature, finalized chain transaction, tax filing, or legal conclusion exists.

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

## Primary language and progressive disclosure

First-view labels describe the operator's task, not the transport or storage
model. Use “Approved tax rule”, “Location verification”, “Review the tax
calculation”, and “Permission status” in the primary flow. Keep rule IDs,
fingerprints, engine versions, account keys, and protocol fields behind
accessible “Record details”, “Technical details”, or administrator disclosures.
Guided controls are the default; exact JSON remains an advanced exchange for
users who own that workflow. Provider-signed jurisdiction records are grouped
under an explicit “Location verification” disclosure and labelled as an
advanced exchange; the UI never presents a raw provider payload as ordinary
customer data. See [ADR-0282](../adr/0282-plain-language-progressive-disclosure-ui.md).

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

## Visual system and how it is verified

The SPA's single stylesheet (`apps/web/src/styles.css`) follows Material 3
Expressive with one repository-owned token set:

- **Palette:** generated from the Material baseline seed `#6750A4` (TonalSpot,
  pinned `@material/material-color-utilities`), with a harmonized amber warning
  role; light and dark roles are pinned by a generation test and a 4.5:1
  contrast contract (ADR-0422).
- **Shape, motion, type, state, elevation:** the ten-step corner scale, six spring
  motion tokens (sampled to CSS `linear()`), the M3 type scale with emphasized
  weights, hover/focus/pressed/dragged state layers, and the level 0-5 elevation
  scale. Spacing uses a repository-owned 4px-grid scale (`--app-space-*`), since
  Material defines none (ADR-0434). Components consume these tokens;
  the stylesheet has no hard-coded font sizes or corner radii, and a test fails
  if one is reintroduced (ADR-0428, ADR-0429).
- **Layout:** workspace layouts respond to the width the workspace has, via
  container queries, not to the viewport (ADR-0421).
- **Canonical blocks:** navigation, panels, topbar, form controls, and the
  access-control/evidence surfaces each have one canonical style block
  (ADR-0423 to ADR-0427) so a change is made in one place. Dead declarations and
  selectors are removed and guarded by tests (ADR-0424, ADR-0428).

A stylesheet change is verified by computed-style snapshots of 245 real-browser
states in both themes (journeys plus markup captured from unit tests), each also
recorded with hover, keyboard-focus, and pressed states forced and with every
disclosure open (about 1,400 files per run):

```sh
scripts/web_style_fixtures.sh              # capture unit-test markup (once per markup change)
scripts/web_style_snapshot.sh before       # snapshot the current stylesheet
# ...edit styles.css...
scripts/web_style_snapshot.sh after
node scripts/diff_style_snapshots.mjs test-results/style-snapshots/before test-results/style-snapshots/after --props
```

Identical CSS yields zero differences; an intentional change lists exactly which
properties moved. Set `STYLE_SNAPSHOT_NO_STATES=1` to skip the interactive
variants and `STYLE_SNAPSHOT_WORKERS=4` to run journeys and fixture states in parallel
(about 3 minutes for a full run); every state is compared in full
(ADR-0431, ADR-0432).
