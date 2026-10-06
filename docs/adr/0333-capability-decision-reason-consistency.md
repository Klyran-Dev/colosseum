# ADR-0333: Capability decision and reason consistency

- Status: Accepted
- Date: 2026-10-05
- Deciders: Principal Low-Latency Architecture, Security, and Product UI
- Scope: React capability-snapshot parsing and authorization affordances

## Context

The browser capability parser already rejected unknown decisions and unknown
reason codes. It accepted a known decision and a known reason independently,
however, so a response such as `allow` with `role_missing` could enter React
state. That contradiction could produce misleading status copy and weaken the
fail-closed presentation boundary, even though the API remains the authority.

## Decision

The parser validates the allowed decision/reason matrix before creating a
`ServerCapabilityDecision`:

| Decision | Allowed reason code |
| --- | --- |
| `allow` | `scope_and_role_satisfied` |
| `deny` | `scope_missing`, `role_missing` |
| `indeterminate` | `resource_context_required` |

Unknown reason codes continue to fail through the existing bounded reason
mapping. Any known but contradictory pair fails closed before the snapshot can
be used by navigation or workspace controls.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| Accept any known decision/reason combination | Rejected | Allows contradictory server payloads to create misleading UI state. |
| Normalize contradictory pairs in the browser | Rejected | The browser must not repair authorization semantics or hide an API contract defect. |
| **Reject unknown and contradictory pairs before state creation** | **Selected** | Keeps the contract explicit, bounded, deterministic, and fail closed with no request-path dependency. |

## Invariants and failure policy

- Every parsed capability decision belongs to exactly one allowed matrix row.
- A contradictory pair cannot produce `allow`, `deny`, or `indeterminate` UI
  state.
- The browser does not infer a replacement reason or decision.
- API handlers continue to perform independent authorization and resource checks.
- The parser remains bounded by the canonical catalog and snapshot limits.

## Latency and operations

The matrix check is constant-time over fixed strings and adds no network,
database, RPC, or broker work. Parse failure is rendered through the existing
authenticated access-error boundary and cannot authorize money movement.

## Verification

- Vitest covers valid allow/deny/indeterminate pairs, unknown reasons, and all
  three contradictory combinations.
- `make web-check` and `make web-browser-check` remain required.
- The canonical API catalog and server-side re-authorization contract remain
  unchanged.

