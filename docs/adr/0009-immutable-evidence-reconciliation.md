# ADR-0009: Preserve immutable evidence and rebuildable projections

- Status: Accepted
- Date: 2026-09-25

## Context

Tax and revenue systems require audit evidence. WebSocket notifications and mutable read models are insufficient evidence.

## Decision

Persist transaction signatures, slots, receipt addresses, policy digests, relevant program events, and raw transaction metadata in append-only storage. Derive operational projections from that evidence. Rebuild and compare projections in scheduled reconciliation jobs.

## Consequences

- Storage and retention costs are explicit.
- Audit and incident investigation are possible without trusting a single database row.
- Privacy controls must prevent sensitive customer evidence from being written into public logs.

