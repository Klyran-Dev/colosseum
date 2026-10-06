# ADR-0400: No implicit asset precision in financial workspaces

- Status: Accepted
- Date: 2026-10-05
- Owners: Web platform and Tax Engine bounded-context owners
- Scope: quote, document quote, document simulation, and governed-rule simulation

## Context

Atomic display precision is part of the financial facts contract. A two-decimal
default is not a harmless presentation choice: it changes the interpretation of
atomic amounts for assets whose precision is zero, six, nine, or otherwise
different from two. The browser previously initialized several precision
controls to `2`, even though the repository contract requires quote and
simulation facts to begin empty and be supplied explicitly.

The risk is amplified by JavaScript coercion: `Number("")` is `0`. A required
HTML field helps ordinary browser submission, but it is not a sufficient
authority boundary for event-driven code, tests, or future integrations.

## Decision

All user-entered asset-precision controls start as an empty string. The browser
must parse the value through one bounded parser that accepts exactly one ASCII
digit (`0` through `9`) and rejects blank, signed, fractional, multi-digit, and
whitespace forms. The parser is used before building facts in aggregate quote,
document quote, document simulation, and governed-rule simulation.

Zero remains a valid explicit precision. No form infers precision from ISO
currency, locale, browser defaults, or a prior calculation. When a verified
settlement route is available, its server-owned finalized mint precision remains
the authority and the settlement compiler must compare it with the supplied
facts; this ADR does not widen that route boundary.

The control remains required and localized through the existing asset-precision
catalog. The empty initial state is not persisted, sent to an API, or converted
through `Number("")`.

## Alternatives considered

1. Keep the two-decimal default. Rejected because it silently biases asset
   interpretation and violates the explicit-facts contract.
2. Infer precision from currency or locale. Rejected because currencies and
   Solana asset mints are different authorities and the inference is not
   cryptographically bound to a route.
3. Fetch mint metadata automatically while rendering. Rejected for quote and
   simulation because it adds an RPC dependency and race to a non-settlement
   form, and cannot replace the finalized route comparison at settlement.
4. Leave HTML `required` as the only guard. Rejected because programmatic
   submission and future adapters could still coerce an empty string.
5. **Start blank and use a shared exact parser before fact construction.**
   **Selected for explicit operator intent, deterministic validation, reuse, and
   fail-closed behavior.**

## Invariants

- No financial workspace sends `atomic_decimals` without an explicit bounded
  operator value.
- `0` and `9` are accepted; blank, `10`, `1.0`, `-1`, and whitespace are
  rejected before network submission.
- No amount, rule, jurisdiction, currency, or asset precision is presented as
  belonging to the operator before entry or server evidence.
- Server-owned finalized route precision remains authoritative for settlement.
- The parser is pure and has no network, clock, wallet, or storage dependency.

## Verification

- Failing-first UI tests cover blank initial precision in all affected forms.
- Parser tests cover accepted endpoints and rejected coercion/format cases.
- Existing quote, document, governance, and simulation request tests provide
  explicit precision before submission.
- Full frontend, browser, accessibility, Material color, and repository gates
  remain required after the change.

