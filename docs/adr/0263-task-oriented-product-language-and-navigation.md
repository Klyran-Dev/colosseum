# ADR-0263: Task-oriented product language and navigation

## Status

Accepted. This decision refines ADR-0262 without changing security,
authorization, evidence, or settlement authority.

## Context

The first production shell grouped real workflows correctly, but its visible
language was written for engineers: “control plane,” “Move funds,” “wallet
bytes,” “policy decision point,” and raw RBAC/PBAC/ABAC terminology appeared in
the primary journey. That makes a money and tax product feel like an MVP or
internal console, increases cognitive load, and forces operators to translate
implementation concepts before they can act.

## Decision

The primary navigation and page chrome will describe user outcomes and work
queues:

- **Daily work** — Tax quotes, Invoice tax, Payment review, Document payment.
- **Rules & access** — Tax rules, Access & approvals.
- **Money movement** — Treasury, Balance checks, Adjustments, Refund requests,
  Refund cancellations, Refund payments.
- **Records & issues** — Exceptions & recovery, with saved evidence available
  inside each record and in administrator details.

The following language rules apply to all new UI work:

1. Lead with the user’s task, object, or decision; do not lead with an
   implementation mechanism.
2. Use plain language in headings, buttons, empty states, errors, and status
   badges. Explain “what happened, what it means, and what to do next.”
3. Keep legal, financial, identity, and chain boundaries explicit, but explain
   them in user terms: “The server checked your access” before “RBAC/PBAC/ABAC.”
4. Put protocol identifiers, scopes, role names, digests, raw evidence, and
   API/chain terminology behind progressive disclosure labelled “Technical
   details” or “For administrators.”
5. Never hide an unavailable or unauthorized action. Show the action with a
   specific reason and a safe next step, while the server remains the
   authority.
6. Avoid dashboard theater. A summary is shown only when it is backed by a
   live API response; otherwise show a clearly labelled empty, loading, or
   unavailable state.

### Money-movement copy contract

Refunds and treasury releases are high-consequence workflows, so their primary
surfaces use the operator's decision language: “Prepare refund request”,
“Prepare refund release”, “Create release request”, “Recipient account”,
“Release expiry”, and “Record next step”. Internal terms such as UUIDv7, PDA,
atomic units, idempotency, and server-owned artifact are not used as primary
headings, buttons, or field labels. Exact addresses, amounts, signatures, and
evidence remain reviewable because they are part of the authorization decision;
their surrounding labels and help text explain what the value means before
exposing its protocol representation. Recovery payloads and raw evidence are
labelled as advanced data and are kept out of the main decision narrative.

This contract also applies to tax-rule authoring, invoice calculation, payment
review, balance checks, and exception recovery. Those workspaces lead with
the business action (“Review and publish a rule”, “Calculate tax for an
invoice”, “Check tax balances”, and “Recovery queue”). Specialist values and
chain identifiers remain available where a reviewer must inspect or reproduce
an outcome, but they do not define the primary journey.

## Alternatives considered

| Alternative | Strengths | Risks | Decision |
| --- | --- | --- | --- |
| Keep engineering terminology visible | Precise for developers | High cognitive load and poor fit for finance/tax operators | Rejected |
| Remove all technical language | Friendly first impression | Hides important security, evidence, and chain boundaries | Rejected |
| Use task-oriented primary language with progressive technical disclosure | Accessible for operators, preserves audit and support detail, works for mixed teams | Requires a maintained content model and terminology review | **Selected** |
| Build separate operator and developer applications | Can optimize each audience independently | Duplicated workflows, policy drift, and operational cost | Deferred; one product surface remains the source of truth |

## Consequences

The product will feel like a serious operations SaaS application rather than a
developer demo. Support, compliance, and finance users can navigate by the
outcome they need. Engineers and auditors retain the exact identifiers and
authorization evidence when they open technical details.

This requires a content review for every new workspace, a consistent status
vocabulary, localized copy, and end-to-end review with finance/tax operators.
The server authorization and Solana evidence boundaries are unchanged and
remain mandatory release gates.

## Verification

- Navigation tests assert task-oriented labels and the absence of engineering
  terminology from the primary shell.
- Access-control tests assert plain-language summaries with technical policy
  details available progressively.
- Existing focus, RTL, axe-core, workflow, and fail-closed tests remain green.
- Refund request, refund release, and treasury release tests assert the plain
  language contract while preserving unchanged API payloads and fail-closed
  wallet/evidence behavior.
- Adjustment review and refund cancellation tests assert that the decision and
  next step lead the flow while exact references and recovery data remain
  available for controlled review.
- Browser workspace and accessibility journeys assert the same language
  treatment for rule review, invoice tax, payment review, balance checks, and
  exception recovery.
