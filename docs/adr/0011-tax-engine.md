# ADR-0011: Add a Tax Engine bounded context

- Status: Accepted
- Date: 2026-09-25

## Context

The settlement protocol needs to support real tax complexity: progressive rates, tax-on-tax, thresholds, exemptions, min/max rules, truncation, rounding, and jurisdiction-specific sequencing. A single signed percentage cannot represent these cases safely.

## Decision

Create a dedicated Tax Engine bounded context implemented as a pure, deterministic Rust domain library plus a warm calculation service. It owns normalized tax facts, jurisdiction resolution inputs, compiled rule graphs, calculation traces, evidence digests, and calculation attestations. The Solana program verifies the attested result and enforces settlement invariants; it does not execute the full rule graph.

## Consequences

- Complex rules can be tested and evolved without increasing Solana compute or account requirements.
- The engine must have strict rule governance, versioning, replay, and auditability.
- The protocol must clearly distinguish automated calculation from legal responsibility.

