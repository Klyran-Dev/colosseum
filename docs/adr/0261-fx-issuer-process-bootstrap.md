# ADR-0261: Vendor-neutral FX issuer process bootstrap

- Status: Implemented for local development; non-local activation remains gated
- Date: 2026-10-02
- Owners: API/Tax, FX Provider, Platform/SRE, Security/Governance
- Scope: API process composition of the server-owned FX issuance port

## Context

ADR-0260 closes the authenticated API composition boundary, and ADR-0255 and
ADR-0256 provide bounded vendor-neutral HTTP adapters for quote sources and
evidence signing. The API process still constructs `AppState` with
`fx_issuance: None`, so a configured local provider/signer topology cannot be
exercised through the real quote path.

The bootstrap must compose the adapters without selecting a vendor, accepting
provider or signer identity from a browser, or silently enabling non-local
traffic while DEC-0251-01/03/04/06/07/09 remain open.

## Decision

Add a dedicated API-process bootstrap module that reads a bounded, strict
configuration contract and constructs:

1. an immutable `FxProviderPolicyRegistry` from one inline document or one
   bounded file;
2. one `HttpFxQuoteSource` per configured provider source, with provider
   identity and endpoint owned by the process; and
3. one `HttpFxEvidenceSigner`, wrapped by the existing
   `FxEvidenceCoordinator` and `ConfiguredFxEvidenceIssuance` ports.

Provider bearer tokens are referenced by environment-variable name in the
source document rather than embedded in that document. Signer bearer material
is read from a dedicated secret environment variable. Key ID, public key,
policy digest, optional exact reference rate, endpoints, and timeout bounds are
server configuration; none are accepted in quote requests.

The complete FX configuration is absent by default. A partial configuration is
an error. A complete configuration is accepted only for `API_ENV=local` in
this ADR. Any non-local configuration fails before the listener binds with an
explicit release-gate error. This keeps the production default fail closed and
leaves provider selection, legal ownership, custody, residency, and regional
SLO evidence under the decision register.

The bootstrap validates that the selected policy digest exists, the configured
source topology is bounded and unique, every source is covered by policy
allowlist validation, signer identities are exact 32-byte lowercase hex values,
reference-rate fields are present as a pair and positive when supplied, and
all HTTP adapter limits remain within their existing bounds. Secrets never
enter `Debug`, logs, metrics, or error messages.

## Configuration contract

The following variables are optional as a group; if any member is present, all
required members must be present:

- `API_FX_POLICY_REGISTRY_JSON` or `API_FX_POLICY_REGISTRY_FILE` (exactly one)
- `API_FX_POLICY_DIGEST_HEX`
- `API_FX_PROVIDER_SOURCES_JSON`, containing bounded source records with
  `provider_id`, `endpoint`, and `bearer_token_env`
- `API_FX_SIGNER_ENDPOINT`
- `API_FX_SIGNER_BEARER_TOKEN`
- `API_FX_SIGNER_KEY_ID_HEX`
- `API_FX_SIGNER_PUBLIC_KEY_HEX`

`API_FX_REFERENCE_RATE_NUMERATOR` and
`API_FX_REFERENCE_RATE_DENOMINATOR` are optional but must appear together.
All JSON uses strict unknown-field rejection and bounded collection sizes.

## Alternatives considered

| Alternative | Result | Reason |
| --- | --- | --- |
| Keep `fx_issuance: None` permanently | Rejected | Leaves the real authenticated cross-currency path unexercisable even in local Docker. |
| Construct adapters inside quote handlers | Rejected | Adds configuration, allocation, secret, and policy-snapshot work to the latency-critical request path. |
| Add a vendor-specific SDK or provider choice | Rejected | DEC-0251-01/03/09 are open; the repository must not invent legal or custody authority. |
| Accept raw provider/signer configuration from the browser | Rejected | Violates server ownership and creates an authority substitution boundary. |
| **Dedicated process bootstrap with local-only activation** | **Selected** | Composes the existing bounded ports once at startup, keeps hot-path state warm, and preserves non-local fail-closed release gates. |

## Invariants and failure behavior

- No listener binds after malformed, partial, over-limit, or unsupported FX
  configuration.
- No provider or signer network call occurs during process startup.
- No quote handler parses secret material or constructs an HTTP client.
- Non-local FX configuration cannot activate through an environment toggle alone.
- Policy, provider identity, signer identity, and custody remain process-owned.
- Missing local configuration preserves the existing no-FX behavior; a local
  cross-currency request then returns the bounded `FX_ISSUANCE_UNAVAILABLE`
  response.
- Provider disagreement, signer failure, stale evidence, and policy rejection
  remain coordinator failures and never produce a persisted quote.

## Verification

Failing-first tests cover absent configuration, partial configuration, strict
source parsing, secret indirection, duplicate sources, digest and reference
rate validation, local construction, non-local rejection, and exact adapter
composition. The full repository gate remains required after integration.

This ADR does not claim concrete provider approval, legal coverage, HSM/KMS
non-exportability, staging readiness, or production FX enablement.
