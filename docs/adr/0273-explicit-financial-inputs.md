# ADR-0273: Financial workspaces start from explicit operator inputs

- Status: Accepted
- Date: 2026-10-03
- Owners: Product, Tax, Security

## Context

The quote, invoice quote, document-payment, and persisted-rule simulation
workspaces previously opened with example amounts, jurisdiction, rule
identifiers, and tax category values. Even when those values were editable, a
production operator could mistake them for current account data or submit them
without understanding their source. A money and tax product must not create a
plausible pre-filled financial state.

`AGENTS.md` prohibits hard-coded financial behavior and misleading UX. A
retry/idempotency reference is different: it is generated locally, contains no
financial meaning, and is intentionally stable while the operator retries the
same request.

## Decision

All financial workspaces start with explicit operator inputs. This includes
total before tax, amount subject to tax, document ID, document line IDs, line
amounts before and after tax, currency, jurisdiction, approved rule or scenario
rate, rule version, tax regime, and tax category where shown. Inputs retain
clear labels, bounded validation, and examples only in placeholders or help
text; placeholders are never submitted as values.

The client continues to generate one CSPRNG-backed request reference for quote
idempotency. It is not a financial default, authority, secret, or tax result.
The API remains authoritative for rule, evidence, tax, and settlement values.

## Alternatives considered

| Alternative | Result |
| --- | --- |
| Keep realistic sample financial values in the form | Rejected: risks accidental submission and implies a live tenant context. |
| Ship a separate demo mode in the production shell | Rejected: creates a second behavior path and blurs the production boundary. |
| Replace inputs with a fixed guided scenario | Rejected: still hides the source of financial facts and limits real operator work. |
| **Start financial fields empty and require explicit entry** | Selected: prevents accidental financial assumptions while keeping the real workflow intact. |

## Invariants and acceptance criteria

- No initial financial amount, line fact, rate, rule identifier, jurisdiction,
  document identifier, or tax category is submitted without explicit operator
  input.
- The quote and scenario submit controls remain disabled until service,
  authorization, and browser validation permit the request.
- The generated request reference remains available for same-request retries and
  does not contain financial or tenant data.
- Tests assert the initial empty state for aggregate, invoice, and persisted-rule
  simulation paths, and that request bodies contain only values entered by the
  operator.
- Empty/loading/error/success and expiry states remain honest and accessible.
