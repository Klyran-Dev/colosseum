# ADR-0482: Share localized protocol status presentation across payment flows

- Status: Accepted
- Date: 2026-10-10
- Owners: Web platform

## Context

Refund claim, refund execution, and document payment surfaces each received
validated lifecycle values from the API. Several of them still rendered those
values directly or mechanically replaced underscores, producing inconsistent
operator language such as `prepared unsubmitted` or `submitted` beside
localized copy.

## Decision

Use one web presentation helper backed by the existing locale catalog. The
helper maps stable protocol values to localized operator labels and uses the
catalogued `unknown` label for unexpected values. It does not change API
contracts, status parsing, or audit evidence.

## Alternatives considered

1. Keep per-panel formatters. Rejected because labels drift and every new
   lifecycle value needs repeated updates.
2. Show raw values or replace underscores. Rejected because implementation
   vocabulary is not suitable for operators and is not localized.
3. Translate statuses in the API. Rejected because machine contracts must stay
   locale-neutral and clients may have different display locales.

## Consequences

- Refund and document-payment workspaces now use consistent status language.
- Unknown values fail safe at the presentation boundary without fabricating a
  new lifecycle state.
- The shared catalog remains the maintenance point for supported locales.

## Verification

- Unit tests cover localized known values and safe handling of an unknown value.
- Full frontend and Docker browser gates remain required before release.
