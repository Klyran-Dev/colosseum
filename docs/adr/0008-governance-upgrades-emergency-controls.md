# ADR-0008: Separate governance, upgrades, and emergency controls

- Status: Accepted
- Date: 2026-09-25

## Context

Programs handling tax reserves need operational controls, but a single admin key or unrestricted pause/upgrade authority is a rug-pull and availability risk.

## Decision

Use separate governed authorities for program upgrades, policy issuer management, merchant configuration, and vault releases. Protect privileged changes with multisig and timelock where compatible with incident response. Emergency pause is fail-closed for new settlements and cannot rewrite existing receipts. Every authority change emits an event and is subject to monitoring.

## Consequences

- Incident response must balance containment with user availability.
- Key ceremony, recovery, and change-management procedures are required before mainnet.
- Governance latency is accepted for changes to money-moving authority.

