# ADR-0360: Versioned document-simulation localization

- Status: Accepted
- Date: 2026-10-05
- Deciders: Principal UI and Low-Latency Architecture
- Scope: governance document/invoice simulation workbench

## Context

The governance workspace supports localized rule authoring and persisted-rule
simulation, but its document-facts simulation subsection still rendered
English-only labels, actions, and degraded states. That made a high-consequence
tax workflow inconsistent for multilingual operators and weakened the product
language contract. The workbench is unattested and must remain visibly
separate from quote, filing, legal-compliance, and settlement authority.

## Decision

Use an immutable, typed presentation catalog for English, German, Arabic RTL,
and Japanese. Localize the workbench heading, evidence explanation, form labels,
line actions, loading/error/unavailable copy, result labels, and accessible
region names. Keep document IDs, currencies, jurisdiction values, rule IDs,
calculation IDs, engine versions, digests, FX provider values, statuses, line
IDs, and server-returned component explanations unchanged.

The catalog is selected from the existing shell locale and falls back
deterministically to English. It never performs calculation, translates facts,
or changes request serialization. Service failure continues to render no local
tax total.

## Alternatives considered

### Leave the subsection in English

Rejected. It creates a mixed-language high-consequence workflow and fails the
localization contract for the governance surface.

### Runtime machine translation

Rejected. Tax, authority, and recovery wording must be deterministic,
reviewable, and release-controlled.

### Translate protocol and server-owned values

Rejected. IDs, statuses, digests, jurisdiction identifiers, and evidence are
audit and replay inputs, not presentation copy.

### Reuse the quote catalog wholesale

Rejected. Document simulation is unattested authoring support and has different
authority, recovery, and result semantics.

## Consequences

- The governance workbench has one deterministic language boundary across its
  authoring and document-simulation flows.
- No network, tax-engine, or settlement hot-path cost is added.
- Manual linguistic, RTL, screen-reader, and legal/tax-owner review remain
  release gates.

## Verification

- Catalog tests cover all supported locales and deterministic fallback.
- Component tests verify German labels, fail-closed unavailable state, and
  preservation of the `US-CA` jurisdiction value.
- Governance tests and the production web typecheck remain green.
