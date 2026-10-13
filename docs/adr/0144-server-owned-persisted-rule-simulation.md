# ADR-0144: Server-owned persisted rule simulation

- Status: Accepted
- Date: 2026-09-27
- Owners: Principal Low-Latency Architecture, Tax Domain, Governance, API
- Bounded contexts: Tax Rules, Tax Engine, Governance, Operator UI

## Context

The isolated `POST /v1/tax/calculate` preview intentionally accepts a complete
rule document only while simulation mode is enabled. It is useful for domain
development, but it is unsafe as the governance workspace's simulation
boundary: a browser could calculate one JSON document and then promote a
different persisted draft. A production governance flow must make the
persisted rule, tenant, identity, digest, and jurisdiction binding explicit.

## Decision

Add `POST /v1/tax/rules/{rule_set_id}/simulate` as the operator simulation
boundary.

- Require `tax:rule:read` and one of the four governance roles. Tenant scope is
  derived from the authenticated principal; the path UUID is never an
  authorization boundary.
- Accept only `{ "facts": TaxFacts }` with unknown fields rejected. The
  request cannot contain a rule document, rule digest, merchant ID, or
  jurisdiction-resolution evidence.
- Load the rule row by `(merchant_id, rule_set_id)`, reject retired rules,
  require the facts jurisdiction to equal the persisted rule jurisdiction,
  decode the versioned `RuleSetDocument`, and verify document identity and the
  canonical digest against PostgreSQL before calculating.
- Reuse the bounded warm `CompiledRuleCache` and pure Rust tax engine. The
  operation is deadline-bounded and ephemeral: it persists no calculation,
  quote, attestation, outbox event, or settlement authority.
- Return the existing calculation trace shape with an ephemeral UUIDv7 support
  reference and `simulated_unattested` status. The UI must state that this is
  not a quote, attestation, publication, filing, legal determination, or
  settlement authorization.
- Keep jurisdiction resolution server-owned. The production quote boundary
  remains responsible for provider evidence and persisted jurisdiction
  resolution; simulation cannot manufacture that evidence.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| Continue using caller-supplied `/v1/tax/calculate` in governance UI | Rejected | Browser can preview a document different from the governed draft; weak tenant and digest binding |
| Return the raw persisted rule and calculate in the browser | Rejected | Duplicates financial semantics, increases numeric/runtime drift, and exposes a client authority surface |
| Persist every simulation as a calculation/evidence row | Rejected | Adds hot-path write contention and audit noise without making an ephemeral simulation a settlement authority |
| **Server-owned persisted-rule endpoint** | **Selected** | Binds tenant, rule identity, canonical digest, jurisdiction, and warm deterministic calculation in one bounded service boundary |

## Invariants and failure behavior

- A successful result's `rule_digest` is the digest of the server-loaded rule,
  never a caller-provided value.
- Cross-tenant or missing UUIDs return not-found behavior without revealing
  another tenant's rule.
- Retired rules, digest/identity corruption, malformed documents, jurisdiction
  mismatch, malformed facts, missing storage, expired auth, and missing roles
  fail closed.
- No simulation response is accepted by settlement compilation; settlement
  requires the separately persisted quote, attestation, route, and finality
  contracts.

## Verification and follow-up

Rust API tests cover authentication, role gating, storage absence, and
client-supplied jurisdiction-evidence rejection. The Docker integration
harness (`make integration-check`) now covers tenant isolation, identity/digest
tampering, canonical recovery, retired-rule behavior, jurisdiction mismatch,
and nested unknown-field rejection against the pinned PostgreSQL 18/API stack.
The SPA validates the response schema and tests create → governance reload →
server-owned simulation. Cache contention/latency measurement, browser
WCAG/RTL/localization evidence, live regional latency evidence, and legal/tax
owner review remain release gates.
