# API contract baseline

The public JSON boundary uses decimal strings for all atomic amounts and
fixed-point rate numerators/denominators. JavaScript `number` cannot represent
all Solana `u64` values exactly, so numeric JSON for financial fields is
rejected instead of being rounded silently.

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

The calculation response returns `total_tax_atomic`, `total_net_atomic`,
component amounts, and rational trace fields as decimal strings. It also
returns `facts_digest` and `rule_digest`; these are the values an issuer must
bind into a calculation attestation.

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
  }
}
```

The service selects a currently effective `published` row from PostgreSQL,
recomputes the canonical rule digest, evaluates the pure engine, and commits
the calculation evidence and idempotency response in one transaction. A retry
with the same key and identical request returns the original calculation. A
different request using the same key returns `409 IDEMPOTENCY_KEY_REUSED`.
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
The response status is `calculated_unattested`; it is not an attestation,
wallet authorization, settlement, filing, or legal determination.

## Attestation verification

`POST /v1/tax/attestation/verify` verifies the canonical payload, Ed25519
signature, validity window, and exact calculation binding. Its successful
status is `verified_unsettled`; it does not authorize a transfer or claim that
the result is a tax filing or legal determination.

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
