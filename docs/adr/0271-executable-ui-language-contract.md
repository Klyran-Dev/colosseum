# ADR-0271: Executable UI language contract

## Status

Accepted. This decision extends ADR-0263 and does not change authorization,
financial authority, API contracts, or chain evidence.

## Context

The product serves finance, tax, treasury, support, and incident operators.
Protocol concepts such as UUIDv7, PDA, atomic units, nonce, digest, and server
ownership are necessary for evidence review, but they are poor primary labels
for an operator deciding what to do next. A one-time copy review is not enough:
new workspaces can silently regress into engineering-console language.

## Decision

The web product has two language layers:

1. **Primary journey language** uses the user's task, outcome, decision, or
   business object in navigation labels, page headings, primary buttons,
   empty states, status summaries, and first-level field labels.
2. **Technical review language** preserves exact identifiers, addresses,
   digests, protocol counters, evidence payloads, and implementation terms in
   explicit `Technical details`, `Advanced`, `Recovery data`, or equivalent
   progressive-disclosure sections.

The primary navigation and workspace headings are checked by
`scripts/verify_ui_language_contract.sh`. The check rejects forbidden protocol
terms in the navigation catalog and asserts the approved operator labels for
the high-consequence workspaces. It runs through `make check` and is also part
of the public-repository contract.

This is a content-safety check, not a translation system. Locale, legal-copy,
screen-reader, and finance/tax operator review remain required release gates.
Technical terms may still appear in source code, tests, API errors, and
explicitly disclosed specialist sections.

## Alternatives considered

| Alternative | Strengths | Risks | Decision |
| --- | --- | --- | --- |
| Rely on design review only | No build tooling | Regressions are easy to merge and hard to detect across lazy workspaces | Rejected |
| Ban technical words from all frontend source | Simple grep | Prevents accurate evidence, tests, and advanced review details | Rejected |
| Separate operator and engineering applications | Clear audience separation | Duplicated workflows and policy/accessibility drift | Deferred |
| **Executable primary-copy contract plus progressive disclosure** | Prevents primary-surface regressions while preserving exact evidence | Requires a maintained label catalog and periodic language review | **Selected** |

## Invariants

- Primary navigation contains no protocol-only terms.
- Primary workspace headings and actions describe an operator outcome.
- Exact evidence remains reachable when required for authorization, support, or
  audit review.
- The check never authorizes, hides, or mutates a financial operation.
- Failure of the contract blocks the repository quality gate.

## Consequences

Product copy becomes an enforceable part of the architecture rather than an
unreviewed styling preference. New workspaces must add an operator label and a
focused contract test. Specialist users retain the precision needed to inspect
chain and evidence details, while the first screen remains understandable to
the person responsible for the business decision.

## Verification

- `make ui-language-contract` passes against the current navigation and
  high-consequence workspace catalog.
- Unit and real-browser accessibility tests continue to validate the rendered
  labels, focus behavior, and failure states.
- ADR-0263 and the implementation plan retain the human finance/tax,
  localization, screen-reader, and legal review gates.
