# ADR-0457: Label settlement amounts as atomic units

- Status: Accepted
- Date: 2026-10-10
- Owners: Product engineering, payments, and accessibility

## Context

The prepared document payment artifact exposed `gross_atomic`, `tax_atomic`,
and `net_atomic` as bare integers. The artifact does not carry an authorized
display currency conversion, and the route may use a token precision that must
not be guessed in the browser. Bare values therefore looked like ordinary
currency amounts while remaining ambiguous to an operator.

## Decision

Format the exact integer values with locale-aware grouping and label each
summary field as “atomic units.” Do not convert, round, or infer a currency in
the payment review. Currency/asset display metadata must be added to the
versioned settlement contract before a human-unit conversion is introduced.

## Alternatives considered

1. Display bare integers. Rejected because the unit is ambiguous and the
   grouping is difficult to scan.
2. Convert to a guessed currency or token symbol in the browser. Rejected
   because display metadata is not currently part of the authorized artifact;
   guessing would be unsafe for money movement.
3. **Locale-format exact values and label them as atomic units.** Selected
   because it improves readability without changing settlement semantics.

## Acceptance criteria

- Gross, tax, and net fields are explicitly labeled as atomic units.
- Values use locale-aware integer grouping and remain exact strings.
- No currency conversion or floating-point arithmetic is introduced.
- Browser accessibility and Docker workflow gates remain green.
