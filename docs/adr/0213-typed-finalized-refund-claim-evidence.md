# ADR-0213: Typed finalized refund-claim evidence at the API and UI boundary

## Status

Accepted for implementation. The server response, strict browser parser, and
wallet workspace proof display are implemented. Live-validator, live-wallet,
and production-provider evidence remain release gates.

## Context

ADR-0180 makes a refund claim `finalized` only after the persisted signature,
server-derived artifact, and finalized `AdjustmentRefundClaimCreated` event
have been compared and recorded. Returning only a lifecycle status, slot, and
signature leaves the browser unable to show the exact event proof. It also
creates a UX risk: a generic finalized label can be mistaken for a tax,
compliance, or legal conclusion.

The browser must display useful evidence without becoming an authority. It
must not accept caller-provided event fields, reconstruct amounts from an
untrusted event, or treat a provider acknowledgement as finality.

## Decision

Extend `RefundClaimPreparationResponse` with nullable
`finalized_evidence`. The API populates it only for a finalized intent by
loading the immutable raw event evidence from the configured targeted-recovery
store and converting it to a versioned typed projection. The server verifies
that the evidence signature and slot match the durable lifecycle and that the
stored event matches the immutable artifact before returning it.

The response contract is deliberately explicit:

- event type/version, discriminator, finalized commitment, signature, slot,
  and event index identify the exact chain event;
- claim, adjustment, receipt, merchant, mint, authorities, reference digest,
  execution window, and evidence digest preserve the reviewed binding;
- nonce and gross/tax/net amounts are canonical decimal strings at the JSON
  boundary; and
- the browser parser rejects finalized claims without evidence, evidence on a
  non-finalized lifecycle, mismatched signature/slot/claim, malformed byte
  arrays, non-canonical counters, invalid windows, and conservation failures.

The wallet workspace renders the proof behind progressive disclosure and
states explicitly that finalized movement evidence is not a legal, tax, or
compliance determination. It shows the event identity, finalized slot, event
index, event signature, conserved amounts, and an evidence-digest preview.

## Alternatives considered

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Return lifecycle status, signature, and slot only | Small response and simple client | The UI cannot display the event proof and may overstate what finality proves | Rejected |
| Return raw event bytes and let the browser decode them | Preserves bytes | Couples the browser to ABI/layout, increases parsing risk, and encourages client-side authority decisions | Rejected |
| Accept event evidence supplied by the browser | Flexible recovery UX | Enables evidence substitution and breaks server ownership of finality | Rejected |
| Return a generic untyped evidence map | Easy to extend | Weak schema guarantees and silent field drift at a money-moving boundary | Rejected |
| **Return a server-derived, versioned typed projection with strict client validation** | Evidence is displayable, schema-stable, bound to durable state, and fail-closed | Adds a response contract and parser tests | **Selected** |

## Invariants and failure policy

- `finalized_evidence` is absent for every non-finalized lifecycle.
- A finalized lifecycle without retrievable, matching evidence is an API
  failure, never a successful response.
- Client-visible evidence is a projection; the Solana event and server-side
  finalized comparison remain authoritative.
- All chain counters use canonical decimal strings in JSON and remain `u64` in
  Rust.
- Amounts must conserve exactly and cannot be replaced by browser input.
- The UI must distinguish finalized movement evidence from compliance,
  reporting, filing, or tax advice.

## Verification

- Rust API compilation and response-conversion tests preserve event binding and
  canonical JSON encoding.
- Browser parser tests cover valid proof, missing proof, lifecycle mismatch,
  signature/slot/claim mismatch, malformed bytes, and amount conservation.
- The wallet workspace test proves the proof is rendered with event identity,
  slot, event index, conserved amounts, and evidence digest.
- Full frontend typecheck, component accessibility tests, and the pinned
  NGINX/Playwright journey now pass; the journey covers proof disclosure, axe
  validation, reload, and durable intent recovery using synthetic API
  evidence. Live wallet, validator, and production-provider evidence remain
  release gates.

## Consequences

Operators can inspect exactly why a claim is considered finalized without
making the browser part of the trust boundary. The API response is larger and
must evolve through an explicit versioned contract, but evidence replay,
support diagnostics, and UI honesty improve materially.
