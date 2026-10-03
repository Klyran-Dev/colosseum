# ADR-0279: Server-owned checkout policy binding

## Status

Accepted — implemented and locally verified; staging and production evidence
remain open.

## Context

The quote-bound settlement endpoints currently require the browser to submit a
policy sequence and a policy digest. Those values are already present in the
issuer-signed calculation attestation persisted for the authenticated tenant.
Asking a finance or tax operator to copy protocol identifiers into a payment
form creates an unnecessary failure mode, exposes technical terminology in the
primary workflow, and allows a caller to propose a policy binding that the
server must later reject.

The browser should select a quote, route, payer wallet, and bounded payment
scope. It must not select the policy authority for a quote. The signed
attestation and authenticated tenant are the source of truth for policy
binding; the API must use the same server-owned values when compiling the
instruction and creating the durable intent.

## Decision

Remove `policy_sequence` and `policy_digest` from the public request bodies for
aggregate, document, and Split Plan quote-bound compilation. The API will:

1. load the authenticated tenant's persisted calculation attestation;
2. verify its validity window before compilation;
3. use the attestation payload digest as the policy digest;
4. use the attestation sequence as the policy sequence; and
5. pass those values to the bounded settlement compiler and immutable intent
   record.

The request schemas remain `deny_unknown_fields`, so stale clients that send
the removed fields fail closed rather than being silently accepted. The
response continues to expose exact protocol evidence through the compiled
artifact and the existing technical-details disclosure. The primary UI no
longer exposes policy inputs; it presents a plain-language “Payment safeguards
selected from the verified quote” confirmation and keeps protocol evidence
available to reviewers.

The client nonce remains part of the protocol envelope because it is the
merchant-scoped replay identity for the intended payment, but it is generated
automatically by the browser from a cryptographically secure random source.
It uses the canonical decimal-string `u64` wire contract from ADR-0172 and is
not a user-facing financial or protocol input. Document idempotency keys remain
generated and retained in the browser session for safe retry, but are not a
user-facing protocol field.

## Invariants and failure behavior

- A quote-bound compile cannot succeed without a tenant-scoped persisted
  attestation.
- The compiled policy digest and sequence are byte-for-byte and integer-for-
  integer derived from that attestation; caller-provided replacements are not
  accepted.
- Aggregate, document, and split compilation use the same policy-source
  contract.
- Cross-tenant reads remain indistinguishable from missing resources.
- Expired, conflicting, malformed, or missing attestation evidence fails
  closed before a wallet artifact or durable settlement intent is returned.
- A missing secure random source fails closed before a payment can be prepared;
  the browser never substitutes a timestamp, JavaScript number, or predictable
  fallback for the replay nonce.
- The exact policy PDA and instruction bytes remain visible in the review
  disclosure and are still validated by the browser and API.

## Alternatives considered

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Keep policy fields in the primary form | No API change | Copy/paste errors, technical UX, and a caller-controlled authority proposal | Rejected |
| Hide the same caller fields in a collapsible advanced section | Smaller primary surface | Still permits stale or substituted policy inputs and leaves an unsafe contract | Rejected |
| Let the browser derive policy values from the attestation JSON | Removes manual typing | Browser becomes responsible for authority binding and can be fed stale or substituted evidence | Rejected |
| Add a separate policy lookup call and send its result back | Explicit API boundary | Extra round trip and a new race between lookup and compilation; still duplicates server authority | Rejected |
| **Derive policy values inside each authenticated compile boundary from persisted attestation evidence** | One authoritative source, fewer round trips, simpler UI, fail-closed replay semantics, and no hot-path external dependency | Requires a versioned API contract change and migration of callers | **Selected** |

## Verification

- Rust API tests reject removed fields and prove policy values are taken from
  persisted attestation evidence for all three compile variants.
- Docker integration tests compile aggregate, document, and split payments
  without caller policy fields and prove exact replay/conflict behavior.
- Browser tests prove the primary payment forms contain no policy input and
  still render the exact review evidence disclosure.
- The UI language contract continues to reject protocol terminology from the
  primary navigation and action surfaces.

## Consequences

This is a breaking request-schema change for pre-release clients. The API
contract and public examples must be updated together; no compatibility shim
will preserve an unsafe caller-controlled policy boundary. Existing persisted
intents and on-chain artifacts remain readable because the instruction and
intent wire formats are unchanged.
