# ADR-0289: Wallet-bound revenue-stream authorities

- Status: Accepted
- Date: 2026-10-04
- Owners: Settlement, Identity, and Revenue Operations

## Context

The revenue-stream program requires distinct proposer, approver, operator,
and governance public keys. The current payout plan stores the proposer from
the authenticated session and the approver after the four-eyes transition,
but it does not identify the operator responsible for preparing releases.
Allowing a browser to submit those public keys would let account topology
become caller-controlled. Reusing a public key from an earlier workflow would
also create a cross-context authorization ambiguity.

## Decision

Every newly proposed payout plan must persist an immutable tenant-scoped
`operator_subject`. The browser submits a subject identifier only; it never
submits an operator, approver, governance, merchant, vault, mint, token
program, or program public key. Legacy rows without an operator subject remain
readable but are ineligible for chain compilation until explicitly migrated.

The existing wallet-proof persistence boundary is extended with
context-qualified revenue-stream roles (`revenue_stream_proposer`,
`revenue_stream_approver`, `revenue_stream_operator`, and
`revenue_stream_governance`). The challenge message remains tenant, subject,
role, challenge, and expiry bound; the database consumes it once and rotates
one active binding per subject/role. The API exposes revenue-stream aliases
for this boundary, while the storage and cryptographic proof code remains
shared and role-validated.

A future chain compiler may load the plan's proposer, approved approver, and
operator subjects, resolve their active wallet bindings server-side, and
persist an immutable compiler intent before returning unsigned bytes. It must
also require the separately verified settlement route and a canonical signed
message verifier. This ADR does not authorize submission or movement by
itself.

## Alternatives considered

1. Accept public keys in plan creation or claim requests. Rejected because the
   caller would control merchant-side signer topology.
2. Infer the operator from the current authenticated caller. Rejected because
   retries, hand-offs, and four-eyes review require a durable named subject.
3. Create a separate wallet-proof implementation and database for streaming.
   Rejected because duplicate challenge/signature logic increases replay and
   key-rotation risk.
4. Extend the shared wallet-proof boundary with context-qualified roles and
   persist the operator subject on the plan. Selected because it preserves one
   cryptographic proof contract while keeping role and tenant authorization
   explicit.

## Invariants

- New plans have a non-empty operator subject distinct from the proposer.
- Plan idempotency includes the operator subject through the immutable spec.
- A role binding is tenant-scoped, single-use challenged, and active only for
  the exact subject and context-qualified role.
- Public keys are resolved by the server and never accepted as authority input
  from the browser.
- Missing, revoked, conflicting, or unverified bindings fail closed.
- Legacy plans without an operator subject cannot reach chain compilation.
- Finalized route verification and exact signed-message verification remain
  separate prerequisites.

## Consequences

Plan creation gains one operational responsibility field and a migration that
is additive for existing rows. The UI can explain who prepares releases before
any wallet prompt. The compiler remains deliberately gated until the
authority, route, and proof adapters are implemented and tested together.
