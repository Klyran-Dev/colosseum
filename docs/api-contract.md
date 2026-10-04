# API contract baseline

The public JSON boundary uses decimal strings for all atomic amounts and
fixed-point rate numerators/denominators. JavaScript `number` cannot represent
all Solana `u64` values exactly, so numeric JSON for financial fields is
rejected instead of being rounded silently.

## Request correlation

Every API response, including authentication failures, validation failures,
health responses, and unknown routes, includes an `x-request-id` response
header containing an API-generated canonical UUIDv7. The API overwrites any
incoming value; clients must treat the value as a diagnostic support
reference, not as an authorization, idempotency, nonce, signature, or evidence
identifier. Clients should preserve the returned value when opening a support
case and may forward it to an explicitly approved downstream service.

## Calculation request

```json
{
  "facts": {
    "gross_atomic": "1100",
    "net_atomic": "1000",
    "currency": "USD",
    "jurisdiction": "US-CA"
  },
  "rule_set": {
    "id": "operator-simulator-flat-rate",
    "version": 1,
    "components": [
      {
        "id": "sales-tax",
        "label": "Sales tax",
        "base": { "sources": ["Net"] },
        "rate": { "Flat": { "numerator": "10", "denominator": "100" } },
        "price_mode": "Exclusive",
        "credit_atomic": "0",
        "floor_atomic": null,
        "cap_atomic": null,
        "rounding": { "mode": "HalfUp" }
      }
    ]
  }
}
```

Advanced published rules may additionally declare a component `threshold`
(`basis`, `mode`, and `amount_atomic`) and bounded `exemption_codes`. Facts may
carry `exemption_codes` and a `cumulative_period` snapshot. Cumulative rules
fail closed without a snapshot; the snapshot evidence digest is included in
the facts digest and must be preserved for replay.

Facts may also carry the server-owned `jurisdiction_resolution` envelope from
ADR-0102. It binds the selected jurisdiction to a versioned resolution policy,
evidence digest, selected source, candidate count, and bounded decision trace.
The compatibility contract accepts absent envelopes for historical replay, but
a new globally supported jurisdiction must not treat a caller's
`jurisdiction` string or digest as resolved evidence; authenticated provider
and resolver integration remains a release gate.

The calculation response returns `gross_atomic`, `total_tax_atomic`, `total_net_atomic`,
component amounts, and rational trace fields as decimal strings. It also
returns `facts_digest` and `rule_digest`; these are the values an issuer must
bind into a calculation attestation.

The calculation result also returns the exact `engine_version` that produced
the trace. Replay tooling must use the same engine version and compare the
complete result, not only aggregate tax totals.

## Dedicated warm Tax Engine service adapter

The separately deployable `tax-engine-service` shares the pure Rust engine and
uses the same `RuleSetDocument`, `TaxFacts`, `TaxDocumentFacts`,
`CalculationResult`, and `TaxDocumentResult` wire contracts. It is an internal
calculation adapter for bounded simulation and batch workloads; it is not a
settlement, attestation, or tenant-authorization boundary.

Protected endpoints require `Authorization: Bearer <service-token>`:

- `POST /v1/calculate` accepts `{ "rule_set": RuleSetDocument, "facts": TaxFacts }`.
- `POST /v1/calculate-document` accepts `{ "rule_set": RuleSetDocument, "facts": TaxDocumentFacts }`.
- `GET /readyz` reports authenticated service readiness.

`GET /healthz` and `GET /metrics` are available only on the private service
network. Metrics are fixed-cardinality Prometheus text and contain no tax facts,
tenant identifiers, rule contents, or credentials; the NGINX edge does not
publish this listener. Local Docker development uses plain HTTP on that private
network. Staging and production require Rustls mutual TLS with a configured
server certificate, private key, client-CA bundle, and a JSON
`TAX_ENGINE_MTLS_ALLOWED_CLIENT_URIS` allow-list containing one to 32 unique
`spiffe://` URI identities. The client certificate must contain exactly one
matching URI SAN; chain trust alone is insufficient. The bearer token remains
defense in depth. Request bodies are limited to 128 KiB, compiled-rule cache
capacity is bounded, and calculation admission has a 500 ms queue deadline.
Overload, invalid rules/facts, unsupported schemas, malformed decimal values,
and worker failures fail closed without partial results.
Malformed JSON, missing JSON content type, and JSON shape errors return `400`;
the request-body limit returns `413`; these responses do not echo request
content. The same rejection semantics apply to aggregate and document
calculation endpoints.

The API keeps quote, attestation, transaction compilation, and settlement
critical paths in-process. The service is not an authoritative settlement or
attestation boundary. Certificate issuance, rotation, revocation, identity
allow-list governance, and production secret delivery remain deployment gates and
must be evidenced by the relevant runbooks before production activation.

`make tax-engine-runtime-contract` builds the pinned service image and probes
the real listener for liveness, authenticated readiness, exact calculation,
malformed/oversized request rejection, and fixed-cardinality metrics. It is a
synthetic local/CI runtime contract and does not claim production identity,
mTLS, live provider, or settlement authority. The Tax Engine service accepts
either an inline bearer token or a bounded file-backed token source; the
sources are mutually exclusive, and Unix deployments reload the file-backed
token atomically on `SIGHUP` while retaining the last known-good value after a
failed validation.

On Unix staging and production deployments, an operator or secret-delivery
controller may send `SIGHUP` after atomically replacing the configured TLS
files. The service validates the complete replacement off the async executor
and atomically swaps it for new connections; malformed material is rejected
and the last known-good configuration remains active. This signal changes
transport identity only and is not an authorization boundary. Certificate
issuance, CA overlap/revocation, permissions, and alerting remain deployment
responsibilities.

`make tax-engine-mtls-runtime-contract` builds and runs the pinned service
image with synthetic staging certificates and verifies client-certificate
rejection, trusted mTLS health/readiness, bearer authentication, exact
calculation output, `h2` negotiation, and atomic invalid/valid CA replacement
across `SIGHUP`. This is a local/CI transport contract; production CA
issuance, identity rotation, revocation, secret delivery, regional trust
configuration, and operator evidence remain release gates.

## Document calculation simulation

`POST /v1/tax/calculate-document` accepts the versioned
`solana-tax-protocol/tax-document/v1` contract from ADR-0203. It is a bounded
simulation route for design and rule-vector validation; it is not an
authenticated production quote, does not persist a quote, does not issue an
attestation, and cannot authorize settlement. It is available only when the
API is running with `SIMULATION_MODE=true`; otherwise it returns
`501 PUBLISHED_RULE_REGISTRY_REQUIRED`.

The request contains the exact `schema` identifier, a `TaxDocumentFacts` value,
and a caller-supplied `RuleSet`. The document has one currency and jurisdiction, one to 256 lines,
unique line IDs, decimal-string atomic amounts, and optional line exemptions.
`RoundingScope::Line` rounds and allocates within each line. `Invoice` rounds
and allocates across the complete canonical `(line_id, component_id)` cell
set. Caller line order never controls the digest or residual tie-break.

The request envelope is:

```json
{
  "schema": "solana-tax-protocol/tax-document/v1",
  "facts": {
    "id": "invoice-1001",
    "currency": "USD",
    "jurisdiction": "US-CA",
    "jurisdiction_resolution": null,
    "cumulative_period": null,
    "lines": []
  },
  "rule_set": {}
}
```

The successful response has the following shape; all atomic amounts and
identifiers that can exceed JavaScript safe-integer precision remain decimal
strings:

```json
{
  "schema": "solana-tax-protocol/tax-document/v1",
  "calculation_id": "0199f8a8-0000-7abc-8def-0123456789ab",
  "status": "simulated_unattested",
  "result": {
    "document_id": "invoice-1001",
    "rule_set_id": "operator-simulator-flat-rate",
    "rule_set_version": 1,
    "engine_version": "tax-engine-v1",
    "currency": "USD",
    "jurisdiction": "US-CA",
    "facts_digest": [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    "rule_digest": [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    "gross_atomic": "3000",
    "total_tax_atomic": "247",
    "total_net_atomic": "2753",
    "lines": []
  }
}
```

Unknown fields, malformed decimal strings, duplicate line IDs, unsupported
document scopes, invalid evidence, and requests over the documented bounds
fail closed. A production document quote must introduce a separately
authorized route with published-rule selection, tenant scope, durable
idempotency, evidence persistence, replay, attestation, and settlement
binding; implementing this simulation route does not satisfy those gates.

## Persisted production quote

`POST /v1/tax/quote` is the production calculation boundary. It requires an
OIDC-compatible EdDSA JWT in the `Authorization: Bearer <token>` header, an
exact published rule selector, the `tax:quote` scope, and a merchant-scoped
idempotency key. The
authenticated `merchant_id` claim is used for persistence; it is deliberately
not accepted from the request body:

```json
{
  "idempotency_key": "checkout-01J8Q4J0M9M2G7YB8F5N3D6K1P",
  "rule_set_id": "us-ca-sales-tax",
  "rule_version": 7,
  "jurisdiction": "US-CA",
  "regime": "sales-tax",
  "facts": {
    "gross_atomic": "1100",
    "net_atomic": "1000",
    "currency": "USD",
    "jurisdiction": "US-CA"
  },
  "source_currency": null,
  "jurisdiction_evidence": [
    {
      "unsigned": {
        "schema_version": 1,
        "merchant_id": "<provider-bound-merchant-digest>",
        "facts_digest": "<pre-resolution-facts-digest>",
        "jurisdiction_code": "US-CA",
        "source": "ShippingDestination",
        "evidence_digest": "<provider-evidence-digest>",
        "issued_at_unix": 1790000000,
        "expires_at_unix": 1790000300,
        "provider_key_id": "<registered-provider-key-id>"
      },
      "provider_public_key": "<32-byte JSON array>",
      "payload_digest": "<32-byte JSON array>",
      "signature": "<64-byte JSON array>"
    }
  ]
}
```

Production quotes require at least one provider-signed jurisdiction evidence
envelope. The API verifies the registered provider key, signature, payload
digest, authenticated merchant binding, pre-resolution facts digest, validity
window, and bounded candidate set before resolving it against the immutable
tenant policy in `jurisdiction_policies`. The browser cannot promote a
jurisdiction string or digest into authoritative evidence. Synthetic local
development uses the isolated fixture issuer only; production provider
registries require governed key rotation and legal/tax ownership.
The byte fields in the envelope are serialized as JSON arrays with exact
lengths: 32-byte digests/public keys and a 64-byte signature. The
angle-bracket values above are documentation placeholders, not accepted
literal values.

For a cross-currency quote, `source_currency` is a bounded currency code such
as `USD` while `facts.currency` is the target calculation/settlement currency.
The API obtains the FX quote through its configured server-owned issuance port,
binds it to the pre-resolution facts digest, and persists the complete signed
envelope. The browser cannot submit a rate, provider, policy digest, signer
identity, or FX evidence. Omitting `source_currency` keeps the same-currency
path; setting it equal to `facts.currency` is rejected. If no approved issuer is
configured, a cross-currency request fails closed with `FX_ISSUANCE_UNAVAILABLE`.

The service selects a currently effective `published` row and its immutable
tenant policy from PostgreSQL,
recomputes the canonical rule digest, evaluates the pure engine, and commits
the encrypted normalized facts, calculation evidence, and idempotency response
in one transaction. Raw provider jurisdiction envelopes are verified in
memory, reduced to the bounded server-owned resolution envelope, and
discarded. A retry
with the same key and identical request returns the original calculation. A
different request using the same key returns `409 IDEMPOTENCY_KEY_REUSED`.
For FX requests, the idempotency identity includes the pre-resolution facts and
normalized source currency but excludes mutable provider quote metadata; the
complete server-issued FX envelope remains immutable evidence and is replay
validated separately.
Replay evidence is decrypted only after merchant authorization and every
replay attempt is recorded in the append-only evidence-access audit ledger;
the audit row contains no customer facts. New durable quote writes fail closed
unless the API has an evidence-key capability configured.
The local compatibility configuration uses `API_EVIDENCE_KEY_ID` plus
`API_EVIDENCE_KEY_B64`; rotation rehearsal may instead provide a bounded
`API_EVIDENCE_KEY_RING_JSON` document with one `current_key_id` and at most
eight total keys; when using the ring in Compose, the two legacy key variables
must be explicitly empty so configuration cannot be ambiguous. Production must inject an implementation backed by the
approved KMS/HSM boundary, not raw key material from a request or database.
Authentication failures are intentionally generic; the API does not disclose
whether a key, issuer, audience, merchant, or evidence record exists.

The deployment supplies `API_AUTH_ISSUER`, `API_AUTH_AUDIENCE`, and either
`API_AUTH_JWKS_JSON` or the bounded single-key pair
`API_AUTH_JWKS_KID`/`API_AUTH_JWKS_PUBLIC_KEY` together. The verifier accepts
Ed25519 (`EdDSA`) keys, requires `kid`, bounds token lifetime to one hour, and
applies a 30-second clock-skew window. Missing or partial authentication
configuration keeps the service unready or returns an unavailable response;
it never enables an unauthenticated quote fallback. Scopes are exact,
case-sensitive OAuth-style tokens in the JWT `scope` claim; an authenticated
principal without `tax:quote` receives `403 INSUFFICIENT_SCOPE`. Authorization
is enforced in the API even when an edge gateway also applies policy.

## Server-owned capability snapshot

`GET /v1/authorization/capabilities` requires a valid bearer token but no
workspace-specific scope. The API verifies the token, derives the tenant from
the authenticated principal, and returns the bounded policy catalog for the
product shell. The endpoint performs no database, RPC, broker, filesystem, or
identity-provider call.

The response schema is `authorization-capabilities/v1`:

```json
{
  "schema": "authorization-capabilities/v1",
  "snapshot_id": "019...",
  "subject": "operator-123",
  "tenant_id": "019...",
  "expires_at_unix": 1800000060,
  "capabilities": {
    "tax.quote.create": {
      "decision": "allow",
      "reason_code": "scope_and_role_satisfied"
    },
    "tax.rule.govern": {
      "decision": "indeterminate",
      "reason_code": "resource_context_required"
    }
  }
}
```

The snapshot lifetime is at most 60 seconds and never exceeds the verified
token expiry. `decision` is one of `allow`, `deny`, or `indeterminate`.
Missing scopes or roles produce `deny`; resource attributes that cannot be
known without a selected record produce `indeterminate`. The browser may use
the response for navigation and explanation only. Every protected API and
settlement operation repeats its exact scope, role, tenant, resource, and
chain-evidence checks. A missing, expired, malformed, or failed snapshot never
enables an action.

The capability catalog has one bounded primary entry for each production
workspace: `access.control`, `evidence.replay`, `refund.claim.cancel`,
`refund.claim.create`, `refund.payment.release`, `settlement.adjustment`,
`settlement.document.wallet`, `settlement.wallet`,
`tax.document.quote.create`, `tax.preview.run`, `tax.quote.create`,
`tax.rule.govern`, `vault.reconciliation`, and `vault.release`. These entries
describe the workspace affordance; they do not replace the handler-level
authorization checks for read, approval, submission, reconciliation, or
finalized chain evidence.

For deployment-owned key rotation, `API_AUTH_JWKS_FILE` may replace the
inline key representations. The API loads and fully validates the bounded
file at startup; inline and file representations are mutually exclusive. On
Unix, an operator or secret-delivery controller must atomically replace the
file and send `SIGHUP`. The API validates the complete replacement outside
the request executor and atomically swaps the immutable key snapshot. Invalid
or unreadable replacements retain the last known-good snapshot and never
enable an unauthenticated fallback. The JWT request path performs no file or
network I/O. Key overlap, secret-manager permissions, revocation timing,
process targeting, and alert/on-call evidence remain deployment gates.

`make api-auth-jwks-reload-contract` exercises the real API binary's valid
rotation, removed-key rejection, and invalid-replacement retention with
synthetic EdDSA material. It is a local/CI runtime contract and does not
prove production identity-provider or secret-manager ceremonies.

For deployment-owned jurisdiction-provider key rotation,
`API_JURISDICTION_PROVIDER_KEYS_FILE` may replace the structured inline
registry or compatibility key pair. The API fully validates the bounded file
at startup; file and inline forms are mutually exclusive. On Unix, an
operator or secret-delivery controller atomically replaces the file and sends
`SIGHUP`. The API parses the complete replacement outside the request
executor, then atomically swaps one immutable provider-key snapshot. Invalid,
unreadable, oversized, duplicate, or cryptographically invalid replacements
retain the last known-good snapshot. Provider evidence verification performs
no file or network I/O on the quote path. Key overlap, secret-manager
permissions, revocation timing, provider ownership, process targeting, and
alert/on-call evidence remain deployment gates.

`make api-jurisdiction-provider-reload-contract` exercises the real API
container with signed synthetic jurisdiction evidence before and after a
provider-key rotation and proves malformed-replacement retention. It is a
local/CI runtime contract and does not prove live provider custody, legal
authority, or production rotation ceremonies.
The response status is `calculated_unattested`; it is not an attestation,
wallet authorization, settlement, filing, or legal determination.

The response also contains `expires_at_unix`, a server-owned UTC Unix expiry.
Settlement compilation must use the quote-bound endpoint before this time;
the API rejects expired quotes and derives gross, tax, and net amounts from
the persisted calculation rather than accepting them from the browser. In a
non-simulation deployment, the calculation must additionally be `attested`
with a persisted server-verified attestation whose payload digest matches the
requested policy digest.

## Authenticated rule-draft authoring

`POST /v1/tax/rules` creates a tenant-scoped `draft` release. It requires the
exact `tax:rule:author` scope and `tax_rule_author` role. The request contains
an idempotency key, external rule identity, jurisdiction/regime, an effective
window expressed as bounded Unix seconds, the complete canonical `RuleSet`
document, and a provenance JSON object:

```json
{
  "idempotency_key": "rule-authoring-01",
  "external_id": "us-ca-sales-tax",
  "version": 8,
  "jurisdiction": "US-CA",
  "regime": "sales-tax",
  "effective_from_unix": 1798761600,
  "effective_until_unix": null,
  "rule_set": {
    "id": "us-ca-sales-tax",
    "version": 8,
    "components": [{
      "id": "sales-tax",
      "label": "Sales tax",
      "base": { "sources": ["Net"] },
      "rate": { "Flat": { "numerator": "10", "denominator": "100" } },
      "price_mode": "Exclusive",
      "credit_atomic": "0",
      "floor_atomic": null,
      "cap_atomic": null,
      "rounding": { "mode": "HalfUp" },
      "threshold": null,
      "exemption_codes": []
    }]
  },
  "provenance": { "source": "counsel-review-ticket-123" }
}
```

The server requires a valid non-empty Tier-1 rule graph. `rule_set.id` and `rule_set.version`
must match the top-level identity. The API derives the authenticated merchant
scope, validates and compiles the graph through the Rust tax engine, computes
the canonical digest, and commits the draft row, `create` governance event,
idempotency response, and `tax.rule.draft.created` outbox record in one
serializable PostgreSQL transaction. A replay with the same key and exact
input returns the same immutable draft evidence; a different input or an
existing tenant/external-id/version conflicts. Creation does not simulate,
publish, attest, certify, or authorize settlement.

The persisted rule document is the versioned `RuleSetDocument` envelope
`{"schema":"solana-tax-protocol/tax-rule/v1","rule_set":{...}}`. The
request keeps the stable `rule_set` API shape for compatibility; the server
wraps every new publication before persistence. Replay accepts the historical
bare `RuleSet` shape only for immutable legacy rows and never writes it again.
An unsupported document schema fails closed before calculation or settlement.

## Tax-rule governance transition

`POST /v1/tax/rules/{rule_set_id}/transition` is the authenticated control-plane
boundary for rule promotion. It requires an exact scope and role pair based on
the requested `to_status`:

| Target | Scope | Role |
| --- | --- | --- |
| `simulated` | `tax:rule:author` | `tax_rule_author` |
| `reviewed` | `tax:rule:review` | `tax_rule_reviewer` |
| `approved` | `tax:rule:approve` | `tax_rule_approver` |
| `published`, `retired` | `tax:rule:publish` | `tax_rule_approver` |

The accepted lifecycle is `draft → simulated → reviewed → approved →
published → retired`. The authenticated subject is recorded as author,
reviewer, or approver at its gate; the three subjects must be distinct, and
only the recorded approver can publish or retire. The endpoint accepts bounded
object evidence and returns the new status and governance revision:

```json
{
  "to_status": "reviewed",
  "evidence": {
    "review_ticket": "TAX-1234",
    "decision": "approved-for-publication-review"
  }
}
```

The API and PostgreSQL trigger enforce the same transition state machine. A
serializable transaction locks the rule, fences the governance revision, and
appends an immutable `rule_governance_events` record atomically with the
status update. Conflicts, replayed transitions, missing duties, role failures,
and non-object/oversized evidence fail closed. This workflow records product
governance evidence; it does not represent legal advice, tax filing, or
jurisdictional certification.

## Jurisdiction-policy governance

`POST /v1/tax/jurisdiction-policies` creates a tenant-scoped `draft` policy
bound to an existing rule release. It requires `tax:jurisdiction:author` and
the `tax_jurisdiction_author` role. The policy document is a bounded,
versioned `ResolutionPolicy`; the API computes its canonical digest and writes
the draft, creation event, idempotency response, and outbox event in one
serializable transaction:

```json
{
  "idempotency_key": "jurisdiction-policy-authoring-01",
  "rule_set_id": "01992f1b-4a70-7e30-8ea2-b6e37a9d9d20",
  "policy": {
    "id": "digital-commerce",
    "version": 2,
    "ordered_sources": ["ShippingDestination", "CustomerResidence"],
    "conflict_mode": "HighestPrecedence"
  },
  "effective_from_unix": 1798761600,
  "effective_until_unix": null,
  "provenance": { "review_ticket": "JURISDICTION-123" }
}
```

`POST /v1/tax/jurisdiction-policies/{policy_id}/transition` uses the
four-eyes lifecycle `draft → reviewed → approved → published → retired`.
`blocked` is available for an operator-approved stop. The exact authorization
pair is:

| Target | Scope | Role |
| --- | --- | --- |
| `reviewed` | `tax:jurisdiction:review` | `tax_jurisdiction_reviewer` |
| `approved` | `tax:jurisdiction:approve` | `tax_jurisdiction_approver` |
| `published`, `retired`, `blocked` | `tax:jurisdiction:publish` or `tax:jurisdiction:approve` as applicable | `tax_jurisdiction_approver` |

The author, reviewer, and approver subjects must be distinct. Publication also
requires the referenced rule release to be published and the effective windows
to be compatible. PostgreSQL transition guards and the service transaction
enforce the same state machine; policy fields and append-only
`jurisdiction_policy_events` are immutable after publication. Reused
idempotency keys replay the original draft, while different input conflicts.

`GET /v1/tax/jurisdiction-policies/{policy_id}/governance` requires
`tax:jurisdiction:read` and one of the jurisdiction governance roles. It
returns the tenant-scoped policy, canonical digest, current duty subjects,
revision, effective window, provenance, and bounded event history. It is an
evidence projection, not a permission grant, and it never accepts a caller
merchant identifier.

## Tax-rule governance read model

`GET /v1/tax/rules/{rule_set_id}/governance` is an authenticated, read-only
operator projection requiring `tax:rule:read` and one of the governance roles
`tax_rule_author`, `tax_rule_reviewer`, `tax_rule_approver`, or
`tax_rule_auditor`. The server derives tenant scope from the authenticated
principal; the path identifier is never an authorization boundary.

The response is loaded in a PostgreSQL repeatable-read transaction and contains
the immutable rule identity, current lifecycle status, governance revision,
effective window, duty subjects, and the ordered append-only transition event
history. Event evidence is returned as bounded JSON objects. A missing rule is
`404 RULE_GOVERNANCE_RULE_NOT_FOUND`; unavailable persistence is
`503 RULE_GOVERNANCE_STORAGE_UNAVAILABLE`; authentication and role failures are
returned before storage access. The response is an evidence projection, not a
client-authoritative permission grant: the transition endpoint rechecks the
current status, revision, exact scope, and exact role on every mutation.

## Server-owned persisted-rule simulation

`POST /v1/tax/rules/{rule_set_id}/simulate` is the governance workbench's
server-owned simulation boundary. It requires `tax:rule:read` and one of
`tax_rule_author`, `tax_rule_reviewer`, `tax_rule_approver`, or
`tax_rule_auditor`. The authenticated merchant is the tenant boundary; the
UUID path value is never a caller-supplied merchant authority.

The request contains only facts. Unknown fields are rejected and the client
cannot provide a rule document, rule digest, merchant identifier, or
jurisdiction-resolution evidence:

```json
{
  "facts": {
    "gross_atomic": "1100",
    "net_atomic": "1000",
    "currency": "USD",
    "jurisdiction": "US-CA"
  }
}
```

The service loads the immutable tenant-owned row, rejects retired rules,
requires the facts jurisdiction to match the persisted rule, decodes the
versioned `RuleSetDocument`, verifies its external identity/version and
canonical digest against PostgreSQL, then calculates with the bounded warm Rust
compiled-rule cache. The response uses the calculation trace shape and
`simulated_unattested` status with an ephemeral UUIDv7 support reference. It
does not persist a calculation, quote, attestation, outbox event, or settlement
authority. Client-supplied jurisdiction resolution is rejected because
production jurisdiction evidence is server-owned at the quote boundary.

This endpoint is not a quote, publication, legal determination, filing,
attestation, or settlement authorization. Digest/identity corruption fails
closed as `500 RULE_SIMULATION_EVIDENCE_CORRUPT`; missing tenant-scoped rules
return `404 RULE_SIMULATION_RULE_NOT_FOUND`; a jurisdiction mismatch returns
`422 RULE_SIMULATION_JURISDICTION_MISMATCH`; retired rules return
`409 RULE_SIMULATION_UNAVAILABLE`.

## Authoritative document quote

`POST /v1/tax/document-quote` is the authenticated production document quote
boundary introduced by ADR-0204. It requires `tax:quote`, an exact
`solana-tax-protocol/tax-document/v1` schema, a merchant-scoped idempotency key,
the published rule selector, and provider-signed jurisdiction evidence. The
merchant is derived from the JWT; it is never accepted from the request body.
The route resolves jurisdiction server-side, calculates with the warm compiled
Rust rule graph, persists the immutable document result and encrypted normalized
facts in `tax_document_calculations`, and publishes the first outbox record in
the same serializable transaction. A repeated idempotency key returns the exact
stored response; a different request for that key returns a conflict.

The response includes the document schema, UUIDv7 `calculation_id`, expiry,
status, complete line/component trace, facts digest, rule digest, and all atomic
amounts as decimal strings. This route does not authorize Solana settlement:
the current attestation and settlement contracts remain aggregate-quote-only
until a separate document-to-payment binding ADR is accepted.

Document quotes use the same optional `source_currency` and server-owned FX
composition contract as aggregate quotes. The source currency is applied to
the document's raw line amounts before tax evaluation; the signed envelope is
bound to the document's pre-resolution facts digest and persisted with the
immutable calculation. A missing issuer fails closed for cross-currency
requests.

`GET /v1/tax/document-quotes/{calculation_id}/replay` is an authenticated,
read-only replay operation. It tenant-scopes the lookup, decrypts the normalized
facts using the stored key reference, recomputes facts/rule/result digests, and
compares the complete document trace under the exact engine version. Any missing
key, tampered ciphertext, schema mismatch, rule mismatch, or replay mismatch
fails closed and is recorded as a denied evidence-access audit event.

`POST /v1/tax/document-quotes/{calculation_id}/attestation/verify` is the
document-only attestation boundary from ADR-0205. It requires
`tax:attestation:verify`, a tenant-scoped active `route_id`, and a signed
`DocumentCalculationAttestation` with its own canonical schema/domain. The
server derives document identity, engine/rule/facts digests, amounts, merchant
identity, and route mint from PostgreSQL; caller-supplied binding values are
not authority inputs. The document attestation, document status transition,
and `tax.document_calculation.attested` outbox event commit atomically in the
dedicated document-attestation table.

The aggregate route
`/v1/tax/quotes/{calculation_id}/attestation/verify` accepts only the aggregate
`CalculationAttestation` contract and `tax_calculations` rows. Aggregate
settlement compilers likewise read only aggregate quote rows. A document ID is
not coerced into an aggregate quote or accepted by aggregate attestation or
settlement authority.

`GET /v1/tax/quotes/{calculation_id}/replay` is an authenticated, read-only
evidence operation requiring `tax:quote`. It loads the calculation by the
authenticated merchant, verifies the normalized `TaxFacts` document, facts
digest, immutable published rule release, rule digest, engine version, and
complete result trace, then returns `replay_verified` with the evidence
digests. Legacy rows without normalized facts return `409
QUOTE_REPLAY_UNAVAILABLE`; corrupt or mismatched evidence fails closed and is
not treated as a successful replay. This operation never authorizes settlement.

## Quote-bound calculation-attestation ingestion

`POST /v1/tax/quotes/{calculation_id}/attestation/verify` is the production
attestation boundary. It requires an EdDSA JWT with the exact
`tax:attestation:verify` scope and accepts only a tenant-scoped `route_id` and
an externally signed `CalculationAttestation`.

The server derives merchant identity, rule/facts digests, jurisdiction, asset
mint, and gross/tax/net amounts from the authenticated persisted quote and
route. It derives current UTC time for validity checking, verifies the Ed25519
signature and amount conservation, and persists the immutable attestation,
`attested` calculation status, and first `tax.calculation.attested` outbox
event in one serializable transaction. A private issuer key never enters the
API. Replays must contain the exact same signed document; a different document
conflicts.

The successful status is `attested_unsettled`; it does not sign, submit, or
finalize a Solana transaction. Quote-bound settlement compilers require this
persisted attestation and derive the policy digest and sequence from its
server-owned signed evidence. Caller-supplied policy fields are rejected. The generic
`POST /v1/tax/attestation/verify` route remains an isolated verification
utility and is not a persistence or settlement authorization boundary. See
ADR-0100.

## Deterministic settlement instruction compilation

`POST /v1/settlement/compile-from-quote` requires an EdDSA JWT with the exact
`tax:settlement:compile` scope. It accepts the persisted `calculation_id`, an
authenticated tenant-scoped `route_id`, the payer wallet public key, and a
positive payer nonce encoded as a canonical decimal JSON string. The API loads the authenticated tenant's persisted,
issuer-signed calculation attestation and derives the policy digest and
sequence from that evidence. `GET /v1/settlement/routes` exposes only active routes
for the authenticated tenant. Each route includes the server-owned
`mint_decimals` value (`0..=9`) obtained from finalized route-verification
evidence; clients must not infer a two-decimal default. The API loads the quote and route, verifies
their status/effective windows, derives the decimal-string amounts from
server-owned evidence, fixes the settlement program ID, and derives the
payer's canonical associated token account locally. Caller-supplied merchant,
mint, source, vault, destination, token-program, and program identifiers are
rejected. The Rust settlement-client bounded context validates conservation,
supported token-program scope, account aliasing, and policy identity, then
derives the protocol-config, vault-authority, policy-attestation, and receipt
PDAs.

The response contains the fixed Anchor account-meta order and base64-encoded
`settle` instruction data with status `prepared_unsubmitted`, plus a bounded
versioned `compute_budget_policy` containing the server-owned compute-unit
limit and micro-lamports-per-compute-unit bid. It is a wallet handoff artifact
only: it is not signed, submitted, simulated, confirmed, or finalized. The API
never receives a payer private key. The browser must serialize exactly the
policy's two compute-budget instructions before settlement; recent blockhash
and commitment status remain separate release boundaries.
On submission, a wallet-selected v0 message is admitted only through the
server-owned bounded ALT resolver from ADR-0161: the API reads the exact
referenced tables at finalized commitment, validates the canonical lookup-table
program owner and active state, expands writable/readonly addresses, and
compares the complete signed message with the server-created instruction.
Client-provided lookup-table contents or digests are never trusted. v1 remains
unsupported.

### Authenticated bounded Split Plan compilation

### Authenticated document-to-payment compilation

`POST /v1/settlement/document/compile-from-quote` is the document authority
boundary from ADR-0206. It requires `tax:settlement:compile` and accepts a
document `calculation_id`, active tenant-scoped `route_id`, explicit
`idempotency_key`, payer wallet, and a positive canonical decimal-string
nonce, plus one of these scopes:

```json
{ "kind": "full_document" }
```

or:

```json
{ "kind": "line_subset", "line_ids": ["line-a", "line-c"] }
```

The server loads and verifies the persisted document quote and signed
`DocumentCalculationAttestation`, binds it to the authenticated merchant and
active route mint, derives the policy sequence and policy digest from that
attestation, and derives gross/tax/net from the selected complete lines.
Line IDs are canonicalized, bounded, unique, and committed into a payment
scope digest. Full-line subsets must use `full_document`; quantity, partial
line, arbitrary proration, and client-supplied amounts are rejected because
progressive, cascading, tax-on-tax, inclusive, and rounded rules cannot be
scaled safely after calculation. A caller needing a partial line must request a
new document quote with facts for that payment.

The response is a wallet handoff artifact for the separate `settle_document`
instruction. It contains the server-derived amounts, document-attestation and
payment-scope digests, fixed account order, a `document-receipt` PDA, the
UUIDv7 settlement intent, and the immutable payment authorization record. The
intent and authorization are inserted atomically; replaying the same
idempotency key and exact request returns the same evidence, while a different
scope, route, nonce, or digest conflicts. The instruction carries document
provenance on-chain and the indexer retains the typed event as finalized raw
evidence. This is withholding/settlement authorization only, not tax advice,
filing, reporting, or legal compliance certification.

`POST /v1/settlement/split/compile-from-quote` is the additive Split Plan
contract from ADR-0092. It requires the same exact
`tax:settlement:compile` scope and accepts `calculation_id`, `route_id`,
`split_plan_id`, payer, and a positive canonical decimal-string nonce. Policy
sequence and policy digest are derived from the authenticated tenant's
persisted signed attestation and are not caller-owned fields. It never
accepts recipient accounts, basis points, gross, tax, or net amounts from the
caller. The API concurrently loads the authenticated tenant's quote, active
route, and active/effective persisted Split Plan, verifies the plan's canonical
digest, and rejects route/plan merchant or mint mismatches.

The response contains a versioned `settle_split` instruction with the immutable
Split Plan PDA and exactly one ordered writable recipient account per persisted
plan recipient. It includes `split_plan_id` and the server-derived
`split_plan_digest` in the evidence envelope. This endpoint proves the
tenant-scoped configuration and compiler boundary in local Docker integration;
on-chain plan initialization, finalized activation evidence, indexer decoding,
wallet UI review, and live CPI/fan-out performance remain production gates.

`GET /v1/settlement/split-plans` requires `tax:settlement:compile` and derives
tenant scope from the authenticated principal. It returns at most 64 active,
currently effective plans ordered by plan key and descending version. Each
plan's recipients, basis points, version, and canonical digest are revalidated
server-side before serialization. The endpoint accepts no merchant ID,
recipient list, amount, or caller-owned digest and is intended to provide the
checkout review evidence required by ADR-0094. Clients must treat the response
as untrusted input and enforce the same bounded recipient, public-key, digest,
and 10,000-basis-point conservation checks before enabling a signing flow.

When PostgreSQL is configured, quote-bound compilation also creates a UUIDv7 settlement
intent and returns `intent_id`, `request_digest`, and `instruction_digest`.
`POST /v1/settlement/submit` requires the exact `tax:settlement:submit` scope,
the intent ID, the unchanged compiled instruction artifact returned by
`/v1/settlement/compile-from-quote`, and canonical base64 wallet-signed
transaction bytes. The lower-level `/v1/settlement/compile` route remains an
explicitly amount-bearing internal/test boundary and is not the checkout path.
The server hashes the canonical instruction artifact and independently parses,
sanitizes, and verifies the signed Solana message before changing intent state
or calling RPC. It requires the exact compute-budget pair followed by one
settlement instruction and compares program IDs, canonical data, account order,
signer flags, writable flags, and (for v0) server-resolved address-table
expansion. It never accepts a private key. The coordinator derives the
native transaction signature from the verified bytes, and the intent must win
a durable `signed -> submitting` compare-and-set before RPC submission. A
provider response with a different signature fails closed. It uses bounded
provider failover, validates the signed message blockhash with
`isBlockhashValid` at confirmed commitment, and returns `submitted` only after
the `submitting -> submitted` transition succeeds. `GET /v1/settlement/intents/{intent_id}` requires
`tax:settlement:read` and exposes the durable lifecycle state. A provider
timeout is persisted as `unknown` (or remains visibly `submitting` after a
process crash) and requires reconciliation; it is never automatically
resubmitted. When a chain signature is known, `POST /v1/settlement/intents/{intent_id}/reconcile`
requires `tax:settlement:reconcile` and advances only from observed provider
commitment evidence; it cannot regress a finalized state.

The browser treats handing signed bytes to `POST /v1/settlement/submit` as an
irreversible observation boundary. After any unsuccessful submit response it
reloads the intent and never clears the intent ID. If the status reload also
fails, the checkout displays `unknown` and disables another signing attempt;
the user must recover the intent through the authenticated status/reconciliation
path rather than authorize a second transaction. See ADR-0098.

Intent reads and transitions are merchant-scoped in the storage predicate using
the authenticated principal. A missing or cross-merchant intent returns the same
404 `SETTLEMENT_INTENT_NOT_FOUND` response so identifiers cannot be used to
probe another merchant's settlement state.

ADR-0056 defines the non-custodial coordinator that accepts only canonical
wallet-signed transaction bytes and reports `submitted`, `processed`,
`confirmed`, `finalized`, `rejected`, or `unknown` from RPC evidence. ADR-0057
requires a durable UUIDv7 intent and append-only lifecycle event before the
`tax:settlement:submit` scope is enabled. A provider timeout after possible
acceptance is `unknown` and must be reconciled by signature or finalized
indexer evidence; it is never a permission to construct a second transaction.
ADR-0058 defines the cryptographic message-binding gate before either durable
submission state or an RPC call.

## Attestation verification

`POST /v1/tax/attestation/verify` verifies the canonical payload, Ed25519
signature, validity window, and exact calculation binding. Its successful
status is `verified_unsettled`; it does not authorize a transfer or claim that
the result is a tax filing or legal determination.

## Authenticated indexer replay-gap operations

Replay-gap operations are system-level recovery controls. They require an
OIDC-compatible EdDSA JWT with the exact `indexer_operator` role; a
`merchant_id` claim never scopes these endpoints.

`GET /v1/indexer/replay-gaps?stream_key=<key>&status=<status>&limit=<n>`
requires `tax:indexer:read`. `stream_key` is required and bounded, `status`
is one of `queued`, `leased`, `completed`, or `dead_lettered`, and `limit` is
bounded to 100 (the default is 50). The response is an operator projection of
durable gap state and includes lease/retry/dead-letter and acknowledgement
fields; it does not assert that a gap is repaired.

`POST /v1/indexer/replay-gaps/{gap_id}/acknowledge` requires
`tax:indexer:operate` and accepts only a bounded JSON note:

```json
{"note":"incident INC-REPLAY-001 reviewed by operator"}
```

Acknowledgement is idempotent for the same subject and note, leaves the gap
`dead_lettered`, and appends an immutable operator-action record. A conflicting
acknowledgement is `409 INDEXER_REPLAY_GAP_STATE_CONFLICT`.

`POST /v1/indexer/replay-gaps/{gap_id}/requeue` requires the same scope and
role, requires a prior acknowledgement, and atomically resets the bounded
retry/lease state to `queued` while appending a `requeue` action. A later
dead-letter event requires a new acknowledgement. The chain remains the
source of truth; neither operation advances a finalized checkpoint or marks
projection repair complete. Authentication, role, malformed-input, missing
gap, and invalid-state failures are explicit and fail closed.

### Targeted finalized signature recovery

`POST /v1/indexer/targeted-recovery` requires `tax:indexer:operate` and the
exact `indexer_operator` role. It accepts one operator-selected target and a
bounded page budget:

```json
{
  "address": "<base58-program-or-account-address>",
  "signature": "<base58-transaction-signature>",
  "note": "incident INC-REPLAY-002 targeted recovery",
  "max_pages": 8
}
```

The server owns the stream key and RPC provider configuration. It discovers the
signature with finalized `getSignaturesForAddress`, fetches exactly the bound
finalized transaction with `getTransaction`, verifies signature/slot/event
identity, and persists only through the projection-only replay boundary. The
request cannot submit a transaction, change a checkpoint, select a merchant,
or turn signature-list metadata into settlement evidence. `max_pages` is
optional and may only lower the configured server maximum of 64.

The API appends an immutable `requested` audit row before recovery and a
`recovered` or `failed` row after the attempt. These are deliberately separate
transactions: a missing terminal row is visible evidence of a crash or timeout
and requires reconciliation. A successful response includes the audit ID,
pages scanned, slot, and counts of persisted raw transaction/event/receipt
evidence and has `status: "recovered"`. Target absence, provider disagreement,
malformed evidence, decoder failure, and storage conflict are explicit
failures; none is reported as repair completion.

## Vault reconciliation operator read model

`GET /v1/vault/reconciliation/targets?limit=50` requires the exact
`tax:vault:read` scope and one of the governed vault read roles. The API derives
tenant scope from the authenticated principal and returns only active target
registrations for that merchant. `limit` is bounded to 1 through 64.

`GET /v1/vault/reconciliation/targets/{target_id}/runs?limit=50` returns the
bounded append-only history for a target after proving the target belongs to
the authenticated merchant. Unknown targets do not disclose cross-tenant
existence.

Opening balances, receipt tax aggregates, release aggregates, expected and
observed balances, and observed slots are decimal strings. A latest `mismatch`
or `error` is an operational stop signal; the API never converts it to a
healthy status. Every returned run also contains a server-owned boolean named
stale. It is computed with the database UTC clock using the fixed 90-second
ADR-0193 policy; the browser cannot supply or override that threshold. A stale
observation is an operational stop signal even when its stored status is
matched, and no latest run is not a matched state. These endpoints are
read-only. Browser clients cannot create targets, edit opening balances,
execute releases, clear alerts, or advance indexer state. Target provisioning
requires the independent finalized opening-balance ceremony defined by
ADR-0127.

## Governed vault-release claim lifecycle

The vault-release bounded context records a two-person treasury command; it is
not a wallet signer and it is not a substitute for finalized Solana evidence.
All endpoints derive the merchant from the authenticated principal and never
accept a caller-supplied merchant identifier.

`POST /v1/vault/release-claims` requires `tax:vault:propose` and the
`vault_release_proposer` role. It accepts Solana account addresses, decimal
string `u64` values, a non-zero 32-byte reference digest, and a Unix execution
window no longer than 90 days:

```json
{
  "idempotency_key": "release-01J8Q4J0M9M2G7YB8F5N3D6K9R",
  "mint": "<base58-mint>",
  "token_program": "<base58-token-program>",
  "tax_vault": "<base58-vault-token-account>",
  "destination": "<base58-destination-token-account>",
  "nonce": "42",
  "amount_atomic": "250000",
  "reference_digest": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32],
  "execute_after_unix": 1790000000,
  "expires_at_unix": 1790003600
}
```

Claim responses use the exact wire contract `nonce`, `amount_atomic`, and
optional `slot` as canonical decimal strings, including values above
JavaScript's safe-integer range. Clients must reject numeric representations,
malformed UUIDv7/account/digest fields, unsupported lifecycle states, and an
invalid execution window before rendering the claim. After a transition, the
client must reload the tenant-scoped claim; the transition envelope and local
form values are not a financial projection.

The first request creates a `proposed` claim, its append-only event, and its
outbox publication in one serializable transaction. An identical retry returns
the same claim; reuse of the idempotency key for different input returns
`409 VAULT_RELEASE_CLAIM_IDEMPOTENCY_CONFLICT`. `GET
/v1/vault/release-claims/{claim_id}` requires `tax:vault:read` and one of the
proposer, approver, governance, auditor, or operator roles.

`POST /v1/vault/release-claims/{claim_id}/transition` requires the exact pair
below. Evidence is a bounded JSON object and is persisted with the state
transition and outbox event.

| Target | Scope | Role | Additional evidence |
| --- | --- | --- | --- |
| `approved` | `tax:vault:approve` | `vault_release_approver` | No chain fields; proposer and approver must differ |
| `onchain_created` | `tax:vault:record` | `vault_release_operator` | Claim PDA, transaction signature, and slot required |
| `executed` | `tax:vault:execute` | `vault_release_approver` | Transaction signature and slot required; actor must be recorded approver |
| `cancelled` | `tax:vault:cancel` | `vault_release_governance` | No chain fields |
| `unknown` | `tax:vault:record` | `vault_release_operator` | Bounded error code required; reconciliation is mandatory |

The accepted state graph is `proposed → approved → onchain_created →
executed`, with governed cancellation and bounded `unknown` reconciliation
edges. PostgreSQL fences transitions, binds event rows to the same merchant as
the claim, and freezes financial/account fields once a claim is approved or
finalized. The API projection never authorizes a token transfer: the Solana
program validates the claim PDA, registry, token accounts, time window, vault
balance, approver, and PDA-signed checked CPI atomically. Finalized chain state
is the authority for whether funds moved.

## Revenue-stream plan selection and governance boundary

The partner-payout workspace is a server-owned reference-selection boundary.
It does not accept merchant-side account topology, raw settlement split or tax
policy digests, token-program choices, or a caller-supplied asset/recipient
pair.

`GET /v1/revenue-streaming/references` requires `tax:stream:propose` and the
`revenue_stream_proposer` role. It returns at most 64 active finalized-verified
payout destinations, 64 active effective settlement split references, and 64
published effective jurisdiction-policy references. The response contains
only selection metadata and opaque UUIDv7 references; inactive, expired,
cross-tenant, ambiguous, or integrity-failing records are omitted or fail the
request. It performs no Solana RPC.

`POST /v1/revenue-streaming/plans` accepts the following production request:

```json
{
  "idempotency_key": "payout-plan-request-01",
  "operator_subject": "payout-preparer-subject",
  "destination_route_id": "01992f1b-4a70-7e30-8ea2-b6e37a9d9d21",
  "split_plan_id": "01992f1b-4a70-7e30-8ea2-b6e37a9d9d22",
  "policy_id": "01992f1b-4a70-7e30-8ea2-b6e37a9d9d23",
  "start_slot": "100",
  "end_slot": null,
  "max_total_atomic": "1000000",
  "max_claim_atomic": "100000"
}
```

The API resolves every selected UUID under the authenticated merchant,
revalidates canonical split/policy evidence, checks that the destination and
split use the same asset and compatible token program, and derives recipient,
asset, route bindings, plan digest, and policy digest on the server. A raw
digest/topology request is rejected rather than interpreted as a legacy
fallback. The persisted plan and first outbox event are written atomically;
replays must use the same idempotency key and resolved inputs.

`GET /v1/revenue-streaming/plans` returns tenant-scoped plans. Their
`destination_route_id`, immutable digests, accrued/released amounts, and
UUIDv7 identities are evidence fields; they do not authorize token movement.
`POST /v1/revenue-streaming/plans/{plan_id}/transition` applies the bounded
four-eyes lifecycle, and `POST /v1/revenue-streaming/plans/{plan_id}/claims`
reserves a bounded release only. A prepared claim is not a transfer or legal
tax filing. Wallet signing, exact-message verification, finalized evidence,
and Linux validator conformance remain separate release gates.

## Settlement-adjustment proposal and governance boundary

The settlement-adjustment bounded context records immutable refund, reversal,
correction, chargeback, and remittance-adjustment deltas against an original
settlement. These endpoints create and govern financial evidence; they do not
change the original receipt, move tokens, or claim that a refund was paid.

`POST /v1/settlement/adjustments` requires the exact
`tax:adjustment:propose` scope and `settlement_adjustment_proposer` role. The
merchant, proposer subject, and adjustment UUIDv7 are derived from the
authenticated principal and server; no caller-supplied merchant or record ID
is accepted:

```json
{
  "idempotency_key": "refund-01J8Q4J0M9M2G7YB8F5N3D6K9R",
  "original_settlement_id": "01992f1b-4a70-7e30-8ea2-b6e37a9d9d13",
  "asset_id": [4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4],
  "kind": "Refund",
  "delta": {
    "gross_atomic": "-250",
    "tax_atomic": "-25",
    "net_atomic": "-225"
  },
  "reason_code": "customer_refund",
  "evidence_digest": [5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5],
  "source_calculation_id": "01992f1b-4a70-7e30-8ea2-b6e37a9d9d14",
  "replacement_calculation_id": null
}
```

`gross_atomic` must equal `tax_atomic + net_atomic`. Refund, reversal, and
chargeback deltas are non-positive and non-zero; correction and remittance
adjustment deltas may be positive or negative subject to the same conservation
rule. All financial values are canonical decimal strings and bounded to the
domain's unsigned-atomic-unit range. The first request returns `proposed`;
an identical retry returns the original record, while reuse of the key with a
different request returns `409 SETTLEMENT_ADJUSTMENT_IDEMPOTENCY_CONFLICT`.

`GET /v1/settlement/adjustments/{adjustment_id}` requires
`tax:adjustment:read` and one of the proposer, reviewer, approver, or auditor
roles. Tenant scope is derived from the bearer token; another merchant's ID is
returned as not found.

`POST /v1/settlement/adjustments/{adjustment_id}/transition` accepts only
`approved`, `rejected`, or `voided` in this slice. It requires the exact
scope/role pairs below and a bounded object-valued evidence document:

| Target | Scope | Role | Rule |
| --- | --- | --- | --- |
| `approved` | `tax:adjustment:approve` | `settlement_adjustment_approver` | Subject must differ from proposer |
| `rejected` | `tax:adjustment:reject` | `settlement_adjustment_reviewer` | Only a proposed adjustment may be rejected |
| `voided` | `tax:adjustment:reject` | `settlement_adjustment_approver` | Only an approved adjustment may be voided |

The accepted graph is `proposed → approved`, `proposed → rejected`, and
`approved → voided`. A `posted` transition and token movement are intentionally
absent until a separate on-chain posting, wallet/signature, finality, and
reconciliation ADR is implemented. Proposal, lifecycle event, and first
outbox record are committed transactionally; the outbox is never the source
of settlement truth.

### On-chain adjustment evidence compiler boundary

ADR-0153 adds an internal Rust `settlement-client` compiler contract for the
Anchor-compatible `record_adjustment` instruction. The authenticated API
boundary is exposed below. The compiler accepts the server-owned approved
adjustment envelope, distinct proposer/approver keys, the original receipt
address, a positive adjustment nonce, kind code, signed-magnitude deltas, and
non-zero adjustment/evidence/calculation digests. It emits a fixed ten-account
meta order, including the active merchant/mint-scoped adjustment registry, and
the deterministic PDA:

```text
["adjustment", original_receipt, adjustment_nonce]
```

The wire payload is versioned and uses one sign byte plus one little-endian
`u64` magnitude for each gross, tax, and net delta. The client and program
both validate sign semantics, exact conservation, overflow, digest presence,
authority separation, and packet budget. The checked-in
[`settlement-adjustment-evidence-v1.json`](../test-vectors/settlement-adjustment-evidence-v1.json)
vector locks the cross-language bytes and account order.

The program creates only an immutable adjustment evidence PDA and emits an
event. It requires the active registry, does not perform a token CPI, mutate
the original receipt, or imply that a refund was paid. Finalized
PDA/event reconciliation and a separate
authority-reviewed funding/beneficiary movement contract are required before
any `posted` or `paid` status can be added to this HTTP contract.

The original receipt account is not supplied by the browser or an operator.
ADR-0164 enriches the finalized `indexed_receipts` projection from the raw
`SettlementExecuted` event with the receipt PDA, payer, merchant, mint, nonce,
and policy digest. The server-side compiler endpoint resolves and validates that
identity through this projection and fails closed on incomplete
legacy rows. ADR-0165 additionally requires the server to resolve
`original_settlement_id` through the authenticated tenant's finalized
settlement intent, matching transaction signature, finalized slot, and asset
mint. Identical multi-stream projections may collapse; conflicting rows are
unavailable. No browser or operator may override this binding. The Docker
integration harness proves the complete authenticated path with a synthetic
finalized receipt projection; validator-backed registry lifecycle, wallet
submission, and finalized on-chain reconciliation remain separate release
gates.

### Wallet-bound adjustment authority and evidence compilation

`POST /v1/settlement/adjustment-authority/challenges` accepts only
`{"role":"proposer"}` or `{"role":"approver"}` with the exact
`tax:adjustment:authority:bind` scope and matching adjustment role. The API
returns a five-minute, tenant/subject/role/expiry-bound UTF-8 challenge. The
wallet must sign those exact bytes with Ed25519; a signature is encoded as
unpadded base64url.

`POST /v1/settlement/adjustment-authority/bindings` accepts the challenge ID,
role, Solana public key, and wallet signature. The service verifies the
signature before consuming the one-time challenge. Active keys are unique per
tenant, subject/role rotation revokes the prior proof, and the API never
receives or stores the private key.

`POST /v1/settlement/adjustments/{adjustment_id}/compile` requires
`tax:adjustment:compile`, the `settlement_adjustment_approver` role, the
recorded approver subject, an approved adjustment, complete source/replacement
calculation evidence, the ADR-0165 finalized receipt binding, both active
wallet proofs, and one active route matching the finalized merchant and mint.
The only request field is `{"confirm":true}`. The server allocates one
durable UUIDv7 evidence intent and monotonic nonce, then returns a
`prepared_unsubmitted` `record_adjustment` instruction. Before creating the
intent, the server derives and reads the merchant/mint adjustment-registry PDA
at finalized commitment, validates its owner, Anchor layout, active state,
authority keys, and provider agreement, and snapshots the registry address,
slot, commitment, and account digest into the intent. Retries reuse the same
nonce and reject changed registry or other immutable input. Receipt, merchant,
mint, token program, authority keys, deltas, registry address, and nonce are
never accepted from the browser.

The response includes `intent_id`, `request_digest`, `instruction_digest`,
`registry`, `registry_slot`, `registry_commitment`, and
`registry_account_digest` so operators can correlate the compiled artifact to
the exact finalized authority evidence.

The endpoint requires a configured settlement RPC coordinator even in local
simulation; simulation does not manufacture chain authority evidence. It never
signs, submits, marks an adjustment posted, or claims that tokens moved. Those
are separate finalized chain-evidence and movement contracts.

### Wallet-bound adjustment submission and finalized recording

The compiler response also includes `chain_intent_id`. The approver wallet
submits the exact returned instruction through
`POST /v1/settlement/adjustment-chain-intents/submit` with that chain intent,
the unchanged compiled instruction, and canonical base64 signed transaction
bytes. The API re-computes the instruction digest, verifies the signed message
contains exactly one `record_adjustment` instruction with the expected program,
data, account order, signer/writable flags, and bounded packet shape, then
fences submission in PostgreSQL before RPC I/O. It never trusts a
client-supplied digest or transaction status.

`GET /v1/settlement/adjustment-chain-intents/{intent_id}` exposes only the
tenant-scoped durable transport state. `POST
/v1/settlement/adjustment-chain-intents/{intent_id}/reconcile` observes the
signature at finalized commitment and first advances at most to `confirmed`.
It advances to `finalized` only after the finalized indexer projection and raw
event evidence exactly match the adjustment PDA, receipt, merchant, mint,
nonce, approver, signed deltas, adjustment digest, and on-chain evidence
digest. RPC acknowledgement, signature status, indexer lag, timeout, or
provider disagreement cannot imply recording, payment, filing, or legal
compliance.

The React adjustment workspace consumes the same contract. After an approved
adjustment is loaded, `Prepare wallet record` calls
`POST /v1/settlement/adjustments/{adjustment_id}/compile` with
`{"confirm":true}` and displays the finalized registry snapshot, exact account
count, instruction digest, and chain-intent ID. `Sign and submit evidence`
connects the injected wallet, requires its public key to equal the compiled
approver signer, sends one record-only instruction without browser-side tax or
amount arithmetic, and posts the unchanged signed bytes. Wallet rejection,
timeouts, unknown provider outcomes, and indexer lag remain explicit states;
the browser never synthesizes a signature or a `finalized` result.

Protocol `u64` values crossing this wallet boundary are canonical decimal
strings, including `adjustment_nonce` in the compiled instruction and
`registry_slot` in the registry proof. Rust retains the bounded `u64` domain
type; the browser must not parse these fields as JavaScript numbers. This
prevents precision loss for valid values above `2^53 - 1` and applies to the
future refund `claim_nonce` artifact as well.

### Governed Refund movement boundary

ADR-0154 adds the first explicitly bounded movement contract for a negative
`Refund` adjustment. It is an on-chain instruction family, not an HTTP
`posted` transition or a client-side success state. A claim is keyed by

```text
["refund-claim", adjustment_evidence_pda, claim_nonce]
```

and must be proposed by the registered adjustment proposer with a distinct
explicit merchant refund-funder signer. Execution requires the registered
adjustment approver and the same refund-funder signer. The tax leg moves from
the merchant/mint vault PDA authority, the net leg moves from the explicit
funder token account, and both checked transfers target one token account
owned by the original payer. Zero-valued legs are skipped, and the program
requires extension-free token accounts, matching mints/programs, pairwise
distinct source/destination accounts, exact adjustment conservation, claim
window bounds, sufficient balances, and single-use finalization.

Claim creation does not move tokens. Execution is atomic: either both
authorized legs and the claim finalization commit, or neither commits. The
Rust compiler emits fixed account orders for the 15-account claim and
14-account execution instructions, with bounded window, digest, nonce, signer,
alias, program, and packet checks. The checked-in
[`settlement-refund-claim-v1.json`](../test-vectors/settlement-refund-claim-v1.json)
vector locks the claim wire bytes, account flags, and deterministic PDAs.
The current implementation remains unavailable to a paid-state API until
Linux validator execution, finalized signature/slot and balance
reconciliation, bounded unknown-transaction handling, and operational/legal
approval are evidenced.

ADR-0155 adds a finalized-only `AdjustmentRefundExecuted` indexer projection
and migration-backed read model. It is bound to the raw event's signature,
slot, event index, evidence digest, claim PDA, adjustment PDA, original
receipt, merchant, mint, funder, payer destination, approver, and exact
gross/tax/net magnitudes. Duplicate identical evidence is idempotent;
conflicting evidence fails closed. This evidence can support reconciliation
workflows, but it must not be translated into `posted`, `chain_executed`, or
`paid` by an API or UI without the separate finalized token-account proof
required by ADR-0154.

ADR-0156 defines that proof: one bounded finalized multi-account snapshot must
show tax-vault decrease by the tax leg, refund-funder source decrease by the
net leg, and payer destination increase by gross at the same non-zero slot.
The pure domain contract rejects slot disagreement, underflow, overflow, and
any observed delta mismatch. The indexer now provides a fail-closed reader
with exact legacy-SPL owner/mint/layout and expected-authority checks, plus
bounded provider failover that never combines snapshots. No HTTP lifecycle
state is advanced from that diagnostic post-state reader. ADR-0157 requires
the production proof path to use finalized transaction pre/post token balances,
exact event/signature binding, and durable reconciliation evidence; claim
account topology remains independently validated.
See [ADR-0157](adr/0157-transaction-bound-refund-balance-proof.md) for the
transaction-bound proof contract.

ADR-0173 is the design gate for the claim/execute HTTP and browser workflow.
Existing adjustment approval, adjustment evidence recording, and indexed
refund evidence must not be presented as a paid or refunded outcome.

The first ADR-0173 control-plane slice now exposes the non-movement funding
configuration boundary:

- `POST /v1/settlement/refund-funding-configurations` requires
  `tax:refund:configure` and `refund_funding_proposer`. It creates an
  idempotent `proposed` UUIDv7 configuration after canonical Solana public-key,
  account-alias, explicit `refund_funder_subject`, and effective-window
  validation. The subject must later be bound to the configured funder key
  through the single-use wallet authority proof boundary; it is not a caller
  selectable signer during claim compilation.
- The existing wallet authority challenge/bind endpoints accept the
  `refund_funder` role with `tax:refund:authority:bind` and
  `refund_funding_funder`. A funding configuration cannot activate until that
  subject has an active proof binding whose public key equals `refund_funder`.
- The same challenge/bind boundary accepts the distinct `governance` role with
  `settlement_adjustment_governance`. Cancellation preparation requires this
  subject's active wallet binding and independently verifies the bound key in
  the finalized on-chain adjustment registry.
- `GET /v1/settlement/refund-funding-configurations` and
  `GET /v1/settlement/refund-funding-configurations/{configuration_id}` require
  the same scope and a governed funding/audit role. Tenant scope is derived
  from the authenticated principal.
- `POST /v1/settlement/refund-funding-configurations/{configuration_id}/transition`
  permits only four-eyes approval, suspension, or retirement. It cannot
  activate a configuration or attach verification evidence. Activation remains
  a server-owned finalized Solana verification operation.

ADR-0174 defines that operation: the verifier derives the vault-authority PDA,
reads the merchant, mint, tax-vault, refund-funder, and net-source accounts in
one finalized legacy-SPL snapshot, requires provider agreement, and persists
immutable slot/commitment/evidence before activation. Browser-supplied status
or evidence cannot satisfy this boundary.

These endpoints configure evidence only; they cannot compile a claim, submit
a wallet transaction, move tokens, or imply a refund outcome.

ADR-0175 and ADR-0176 now enable only the preparation boundary:

- `POST /v1/settlement/refund-claims` requires the exact
  `tax:refund:claim:prepare` scope and `settlement_adjustment_proposer` role.
  The request contains `adjustment_id`, an idempotency key, and `confirm: true`.
  The adjustment ID is only a tenant-scoped aggregate selector; receipt,
  adjustment PDA, amounts, funding configuration, token accounts, claim nonce,
  wallet bindings, and expiry are loaded or derived server-side.
- The response is a durable `prepared_unsubmitted` claim intent with a
  versioned unsigned artifact. It includes the server-derived gross/tax/net
  values, mint/token program, tax vault, source, payer destination, receipt
  signature/slot, claim PDA, instruction/artifact digests, and bounded claim
  window for wallet review. It does not submit, sign, or report a refund.
- A retry with the same tenant and idempotency key returns the exact persisted
  artifact when the canonical server-owned request digest matches. A changed
  adjustment, authority, topology, or financial input fails with a conflict;
  any nonce gap after a process crash is intentional and safe.

ADR-0238 adds the governed cancellation preparation and transport boundary:

- `POST /v1/settlement/refund-claim-cancellations` requires
  `tax:refund:claim:cancel:prepare` and the
  `settlement_adjustment_governance` role. The request contains only a
  tenant-scoped finalized `claim_intent_id`, an idempotency key, and explicit
  confirmation. The server loads the immutable claim artifact, resolves the
  tenant's governance wallet binding, and requires finalized adjustment-registry
  provider agreement for merchant, mint, proposer, approver, and governance.
- The response is a separate durable UUIDv7 cancellation intent containing a
  fixed five-account, amount-free unsigned artifact. It contains no gross,
  tax, net, token-account, or destination input. Replays return the exact
  artifact for the same tenant/idempotency binding.
- `GET /v1/settlement/refund-claim-cancellations/{intent_id}` requires
  `tax:refund:claim:cancel:read` and a governed adjustment role.
- `POST /v1/settlement/refund-claim-cancellations/submit` requires
  `tax:refund:claim:cancel:submit` and accepts only the intent ID plus
  canonical wallet-signed bytes. The server verifies the exact cancellation
  instruction, governance signer/writable fee-payer boundary, recent
  blockhash, immutable artifact digest, and no address lookup tables or extra
  instructions before RPC submission. Provider ambiguity is `unknown`, never
  cancelled success.
- `POST /v1/settlement/refund-claim-cancellations/{intent_id}/reconcile`
  requires `tax:refund:claim:cancel:reconcile` and a governed adjustment role.
  It recovers the exact submitted signature at finalized commitment, matches a
  typed `AdjustmentRefundClaimCancelled` event against the immutable claim and
  cancellation artifacts, then atomically records cancellation finality and
  converges the original claim projection to `cancelled`. Transport
  acknowledgement alone does not make a claim cancelled.

The indexer also runs a bounded automatic reconciliation cycle for submitted,
processed, confirmed, and unknown cancellation intents. It owns a short
PostgreSQL UUIDv7 lease, recovers only the persisted governance authority and
signature, and uses the same typed finalized-event matcher and token-fenced
atomic transition. The operator endpoint cannot steal an active lease; stale
leases are reclaimable after the bounded crash window. The backlog is exposed
as the fixed-cardinality `stp_indexer_refund_claim_cancellations_pending`
metric.

Signed submission and exact program/instruction/account/signer/artifact-digest
verification are implemented by ADR-0177. Finalized claim-event evidence is
implemented by ADR-0180; transaction-bound balance proof remains a separate
release gate. A timeout is `unknown`, not success; only finalized
transaction-bound event and balance proof can produce a successful movement
state.

ADR-0177 defines the next claim transport contract. Its recovery endpoint is
`GET /v1/settlement/refund-claims/{intent_id}`. Its submission endpoint is
`POST /v1/settlement/refund-claims/submit` with only `intent_id` and canonical
`signed_transaction_base64`; the server loads the persisted artifact and
rejects any caller-supplied instruction replacement. Submission verifies the
exact fixed-program refund instruction and recent blockhash before the RPC
call. Provider ambiguity advances the intent to `unknown`, and no response
claims a refund until later finalized event and balance proof.
The recovery response exposes the persisted signed-transaction digest,
signature, submitted slot, commitment, and bounded error code when present;
these are transport evidence only and are not a refund-success assertion.

When the durable lifecycle is `finalized`, the recovery response also includes
`finalized_evidence`. This is a server-derived typed projection of the exact
finalized `AdjustmentRefundClaimCreated` event: event identity, discriminator,
signature, slot, event index, claim/artifact bindings, execution window,
conserved gross/tax/net amounts, and the immutable evidence digest. It is
returned only after the server has matched the stored event against the
persisted signature and immutable artifact. All `u64` fields are canonical
decimal JSON strings. A finalized response without matching evidence is an
error; the browser never supplies or replaces these fields. The UI exposes
the proof as movement evidence only and does not claim filing, reporting,
legal compliance, or tax advice. See ADR-0213.

`POST /v1/settlement/refund-claims/{intent_id}/reconcile` requires the exact
`tax:refund:claim:reconcile` scope and `settlement_adjustment_proposer` role.
It accepts no caller-supplied signature, address, topology, or page budget. The
server searches the persisted proposer/signature through bounded finalized
recovery, validates the exact immutable `AdjustmentRefundClaimCreated` event
against the durable artifact and amounts, and only then advances the claim to
`finalized` with slot, commitment, event digest, and outbox evidence. Missing or
conflicting evidence remains a reconciliation failure; it never becomes a
successful claim.

The indexer service also runs the same proof-driven reconciliation as a
bounded, restart-safe worker. It selects only submitted claim intents with a
persisted signature, derives the proposer and event expectation from the
immutable artifact, and uses the same compare-and-set lifecycle/outbox path.
The worker accepts no API input and duplicate workers converge safely; a
provider failure or missing finalized event leaves the claim proof-pending.

ADR-0179 adds the proof-gated execute transport contract:

- `POST /v1/settlement/refund-executions` requires the exact
  `tax:refund:execute:prepare` scope and
  `settlement_adjustment_approver` role. The request contains only a
  finalized `claim_intent_id`, an idempotency key, and `confirm: true`.
- The server requires the claim intent to be finalized with signature/slot
  evidence, re-loads the finalized receipt and adjustment binding, verifies
  the active funding topology and distinct approver/refund-funder bindings,
  recompiles the fixed fourteen-account execute instruction, and persists the
  unsigned artifact before any wallet prompt.
- `GET /v1/settlement/refund-executions/{intent_id}` requires
  `tax:refund:execute:read` and returns only the authenticated approver's
  tenant-scoped durable artifact and transport evidence.
- `POST /v1/settlement/refund-executions/submit` requires
  `tax:refund:execute:submit` and accepts only `intent_id` plus canonical
  `signed_transaction_base64`. It verifies the exact execute instruction,
  both signer privileges, immutable artifact digest, and recent blockhash
  before RPC submission. Provider ambiguity becomes `unknown`.
- Execute `submitted`, `processed`, and `confirmed` responses are explicitly
  proof-pending. Only the finalized typed refund event and transaction-bound
  balance/conservation proof may produce `finalized` movement evidence.

## Compatibility rules

- Adding response fields is backward compatible.
- Changing signed or financial field meaning requires a new schema version and
  ADR.
- Decimal strings are base-10, canonicalized without leading `+`, and must
  fit the domain bounds.
- Clients must not infer settlement success from a calculation response.
- Production quotes must use `/v1/tax/quote`; `/v1/tax/calculate` is an
  explicitly simulated caller-supplied-rule route and is not a production
  registry substitute.
- `/v1/tax/calculate-document` remains explicitly simulated and caller-rule
  driven. Production document quotes must use `/v1/tax/document-quote`; neither
  document route currently authorizes aggregate settlement or aggregate
  attestation.
