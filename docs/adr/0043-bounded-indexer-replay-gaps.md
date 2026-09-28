# ADR-0043: Represent indexer gaps as bounded replay work

- Status: Accepted for the indexer domain
- Date: 2026-09-26
- Owners: Chain Data / SRE
- Scope: Slot-range gap detection, replay scheduling, and retry exhaustion

## Context

Subscriptions and provider pages can fail after a checkpoint is read but before
the page is durably committed. A production indexer must preserve the affected
range as explicit work. Retrying inline or silently advancing the cursor risks
unbounded work, retry storms, or an unrecoverable accounting omission.

## Decision

Add a pure `ReplayQueue` to the indexer bounded context. A replay gap contains a
validated slot range, a typed reason, the finalized watermark observed when the
gap was created, and a bounded attempt count. The queue coalesces overlapping
or adjacent ranges with the same reason, preserves different reasons as
separate work, caps the number of queued gaps at 1,024, and caps a gap at eight
attempts. Claims are bounded to `MAX_SCAN_RANGE_SLOTS`; a larger gap is split
without skipping slots. A failed claim is explicitly requeued, while a claim
that reaches the attempt limit returns an exhaustion error for operator
dead-letter handling. No method performs network, database, or clock I/O.

The queue is a scheduling primitive, not evidence authority. Only a finalized
page that passes the coordinator and serializable persistence invariants may
advance the checkpoint. A later adapter will persist queue state and expose
operator controls; this domain behavior is established first so those adapters
cannot invent retry semantics.

## Alternatives considered

- Inline retry in the provider call: rejected because it hides gaps, inflates
  request latency, and can create retry storms.
- Unbounded broker queue: rejected because user/provider state can exhaust
  memory and queue lag becomes opaque.
- Skip the range and rely on a periodic full rescan: rejected because it loses
  explicit recovery ownership and can hide a tax-relevant omission.
- One gap row per slot: rejected because it creates excessive storage and
  scheduling overhead; bounded coalesced ranges preserve the recovery set.

## Consequences

Positive:

- Every recovery unit is bounded, typed, observable, and testable without a
  live chain.
- Adjacent failures do not create one queue item per slot.
- Retry exhaustion is explicit rather than silently dropped.

Costs and follow-on gates:

- A PostgreSQL adapter must persist leases, attempts, dead-letter state, and
  operator acknowledgements before live replay is enabled.
- Gap creation must be wired to provider disagreement, incomplete pages,
  decoder failures, and projection commit failures.
- Replay completion still requires finalized evidence and reconciliation.
