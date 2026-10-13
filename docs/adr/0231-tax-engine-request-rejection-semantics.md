# ADR-0231: Explicit Tax Engine request-rejection semantics

- Status: Accepted
- Date: 2026-10-01
- Deciders: Principal Low-Latency Architecture, API, SRE
- Scope: Tax Engine HTTP request boundary
- Bounded contexts: Tax Engine, Service Runtime, API Contract

## Context

The Tax Engine uses Axum JSON extraction behind authentication and a bounded
request-body layer. Axum's default JSON extractor response is `422
Unprocessable Entity`, while this service's public contract distinguishes
syntactic/shape errors from a body that exceeds the configured 128 KiB limit.
The distinction matters for client behavior, overload protection, telemetry,
and the executable container contract. Mapping every extractor rejection to a
generic client error would accidentally turn a body-limit rejection into `400`
and obscure an admission-control event.

## Decision

Handle `JsonRejection` explicitly at both calculation endpoints:

- malformed JSON, missing JSON content type, and JSON deserialization errors
  return `400 Bad Request` with a bounded, non-sensitive error message;
- the `RequestBodyLimitLayer`'s `LengthLimitError` returns `413 Payload Too
  Large`;
- authentication remains the outer boundary and returns `401 Unauthorized`
  before request extraction;
- service overload and worker failure remain `503 Service Unavailable`.

The service does not expose Axum's default `422` semantics as part of this
contract. Rejection details never echo request bodies, tax facts, credentials,
or parser internals. The behavior is shared by aggregate and document
calculation endpoints and is verified by unit tests and the real pinned Docker
runtime contract.

## Alternatives considered

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Preserve Axum's default `422` response | No handler code | Diverges from the repository API contract and makes clients distinguish equivalent boundary errors inconsistently | Rejected |
| Map every JSON rejection to `400` | Simple and predictable | Masks the configured admission limit as an ordinary payload error and loses useful operational semantics | Rejected |
| Add a broad global rejection middleware | Centralized mapping | Couples unrelated routes, makes body-limit provenance less explicit, and increases cross-service behavior drift | Rejected |
| **Map `JsonRejection` at each typed calculation boundary and preserve `LengthLimitError` as `413`** | Explicit, bounded, testable, and keeps auth/body-limit ordering observable | Requires a small shared mapping function at the service boundary | **Selected** |

## Invariants and failure behavior

- Authentication is evaluated before calculation payload extraction.
- No malformed or oversized request reaches the tax domain engine.
- A request larger than `MAX_REQUEST_BODY_BYTES` never becomes a `400` due to
  JSON mapping.
- Rejection responses contain no raw body, rule set, tax fact, token, or
  sensitive parser detail.
- Both calculation endpoints preserve the same status semantics.
- Status mapping is not an authorization decision and cannot authorize
  calculation, attestation, or settlement.

## Verification and release impact

- Unit tests cover authenticated malformed JSON (`400`) and the configured
  body limit (`413`).
- `make tax-engine-runtime-contract` exercises both behaviors against the
  pinned service image and verifies deterministic calculation and metrics.
- API documentation records the status contract; any change requires a new
  compatibility review and contract-vector update.
- Production client retry policy must treat `400` and `413` as non-retryable
  unless a caller changes the request; `503` remains bounded-retry eligible
  under the service retry budget.
