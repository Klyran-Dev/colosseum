# ADR-0100: Server-owned calculation-attestation ingestion

- Status: Accepted
- Date: 2026-09-27

## Context

The policy bounded context defines a signed `CalculationAttestation`, but the
API previously exposed only a generic verification endpoint. That endpoint
accepted the binding and verification time from the caller and did not persist
the verified evidence. A caller could therefore present a valid signature that
was bound to a different merchant, quote, asset, or validity window and still
receive a successful verification response.

Settlement needs an immutable, replayable attestation bound to the
authenticated merchant's persisted calculation and the selected, tenant-owned
settlement route. The API must never hold an issuer private key; signing is an
external issuer/HSM/KMS responsibility.

## Decision

Add an authenticated quote-bound attestation-ingestion endpoint:

`POST /v1/tax/quotes/{calculation_id}/attestation/verify`

The request contains only the selected `route_id` and externally signed
`CalculationAttestation`. The server derives the merchant binding, quote facts
and rule digests, route mint, amount tuple, and current UTC time. It verifies
the signature, validity window, and complete binding before atomically:

1. storing the immutable attestation document under the authenticated tenant;
2. promoting the calculation to `attested`; and
3. creating the first `tax.calculation.attested` outbox event.

An identical repeated ingestion is an exact replay; a different attestation
for the same tenant/calculation is rejected. The existing generic verification
route remains an isolated cryptographic utility and is not settlement
authority. Production issuers remain external and must use HSM/KMS/multisig
controls; no API environment variable contains a signing seed.

Settlement compilation will require the persisted attestation in non-simulation
operation and will compare its payload digest to the requested policy digest.

## Alternatives considered

| Alternative | Decision |
| --- | --- |
| Let the caller provide merchant, amount, mint, and current time | Rejected: those are authority-sensitive values and must come from authenticated server-owned evidence. |
| Sign attestations inside the API process | Rejected: it creates private-key custody in the web/API path and weakens HSM/KMS isolation. |
| Verify but do not persist the attestation | Rejected: settlement and replay require durable evidence and idempotent lifecycle behavior. |
| Store only the payload digest | Rejected: audit and independent replay require the bounded signed document and issuer key. |
| Persist evidence and publish later | Rejected: the first outbox publication record must be committed atomically with the evidence. |

## Invariants and evidence

- No caller-controlled merchant identity, amount, rule/facts digest, mint, or
  clock value participates in the quote-bound binding.
- The attestation must verify at server time and bind to the exact persisted
  calculation and active tenant route.
- The stored document, calculation status transition, and first outbox event
  commit together.
- Replays return the same payload digest; conflicting attestations fail closed.
- Settlement compilation cannot use an unattested quote in non-simulation mode.
- The required evidence matrix covers binding substitution, expired/future
  attestations, idempotent replay, conflicting evidence, tenant isolation, and
  compile rejection; unit coverage is present for cryptographic binding and
  compilation enforcement. The clean-stack Docker harness additionally proves
  an external, isolated issuer fixture can ingest evidence into PostgreSQL,
  replay it exactly, reject conflicts and cross-tenant use, and unlock
  non-simulation single and Split Plan compilation. Live HSM/KMS issuer
  integration, live-chain registration, and validator-backed evidence remain
  release gates.

Evidence: migration `0021_calculation_attestations.sql`, the storage adapter,
the authenticated API route, policy tests, compile-path tests, the isolated
`attestation-fixture` crate and pinned non-root image, and
`make integration-check`.
