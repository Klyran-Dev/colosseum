# ADR-0255: Bounded HTTP FX provider adapter

- Status: Implemented as a vendor-neutral transport boundary; concrete provider pending
- Date: 2026-10-02
- Decision owners: FX Provider bounded context, Platform Security, Tax/Legal

## Context

ADR-0252 defines bounded concurrent acquisition and ADR-0254 makes its result
ordering deterministic, but the repository has not yet defined the transport
boundary used by a real FX source. A transport adapter must not silently choose
a vendor, legal owner, supported jurisdiction, custody model, or production
SLO. It must also prevent an HTTP integration from becoming an SSRF, secret
leak, unbounded-body, retry-storm, or non-deterministic evidence boundary.

## Decision

Create a separate vendor-neutral `fx-provider-http` Rust crate implementing
`fx_provider::AsyncFxQuoteSource`. The adapter is a bounded transport source;
policy admission, provider identity, legal acceptance, signing, and settlement
remain separate contexts.

The adapter will:

- accept a configured provider identity and immutable endpoint; the response
  cannot replace the provider identity;
- require HTTPS for staging and production, and permit HTTP only for an
  explicitly loopback local fixture with an explicit port;
- reject endpoint credentials, query strings, fragments, missing hosts, and
  overlong endpoint or bearer-token configuration;
- keep bearer credentials private and exclude them from errors, logs, traces,
  metrics, request evidence, and `Debug` output;
- send a fixed canonical JSON request containing currency pair, merchant
  digest, and facts digest; request and response bodies are bounded;
- accept only successful JSON responses with strict unknown-field rejection,
  decimal-string integer fields, exact lowercase hexadecimal digests, and
  explicit legal-acceptance semantics;
- derive source evidence from the bounded raw response bytes and attach the
  configured provider identity, rather than trusting a server-supplied field;
- apply one bounded client timeout and no hidden retries. ADR-0252 owns the
  aggregate deadline and bounded concurrency; a future provider policy may
  define explicit retry/circuit behavior outside this adapter;
- map transport, status, content-type, body-limit, and decoding failures to a
  fail-closed domain error without exposing vendor response data.

The adapter does not enable a provider, persist a quote, sign evidence, call a
tax service, or activate a settlement route. Provider selection, regional
routing, legal acceptance, HSM/KMS custody, privacy retention, and production
SLO evidence remain open release decisions in the Tax Engine decision register.

## Bounds and invariants

| Resource or invariant | Bound |
| --- | --- |
| Endpoint text | 2 KiB |
| Bearer token | 4 KiB |
| Per-request timeout | positive, at most 5 seconds; default 500 ms |
| Response body | positive, at most 32 KiB; default 32 KiB |
| Request body | at most 4 KiB |
| Retry behavior | none inside the adapter |
| Provider identity | configured server-side value only |
| Evidence input | exact bounded raw response bytes |

No binary floating point is accepted for FX rates. Amounts, rates, and
quantities use decimal strings and are parsed into exact integer types before
the candidate enters policy validation.

## Alternatives considered

- Put HTTP directly in `fx-provider`: rejected because policy/orchestration and
  transport security would become inseparable and harder to audit.
- Implement a vendor-specific adapter first: rejected because the provider,
  legal, regional, and custody decisions are intentionally not frozen.
- Use an unbounded generic HTTP client: rejected because response size,
  timeout, proxy, endpoint, credential, and retry behavior would be implicit.
- Queue provider requests: rejected for the synchronous quote path because it
  adds expiry and delivery ambiguity; asynchronous refresh can be added later
  as a separate bounded workflow.
- **Separate bounded vendor-neutral HTTP source:** selected because it closes
  the transport contract while preserving provider-neutral release gates.

## Verification

Before any real provider is enabled, tests must prove endpoint policy,
credential redaction, strict request/response schemas, exact decimal parsing,
identity binding, raw-response evidence hashing, status/content-type rejection,
body limits, timeout behavior, cancellation, and no hidden retry. Production
approval additionally requires provider, jurisdiction/asset, legal-owner,
HSM/KMS, privacy, regional SLO, failover, and independent security evidence.
