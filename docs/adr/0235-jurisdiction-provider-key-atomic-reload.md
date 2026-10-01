# ADR-0235: Atomic jurisdiction-provider key registry reload

- Status: Accepted
- Date: 2026-10-01
- Deciders: Principal Low-Latency Architecture, Platform Security, SRE
- Bounded contexts: Jurisdiction Evidence, API Runtime, Deployment Operations

## Context

The API verifies provider-signed jurisdiction evidence before a production
quote or document quote can be persisted. The provider registry is currently
validated at startup, while ADR-0220 defines structured registry input for
provider-key rotation. A process restart should not be the only way to remove
an emergency-compromised provider key or publish an overlapping replacement.

The registry is part of the quote authorization boundary. Reloading a partial,
malformed, or untrusted file must never change which evidence is accepted, and
the quote request path must not perform filesystem I/O, network I/O, or acquire
a reload lock.

## Decision

Add an optional `API_JURISDICTION_PROVIDER_KEYS_FILE` deployment input. When
configured, the API reads a bounded JSON registry at startup and listens for
Unix `SIGHUP`. A deployment controller must atomically replace the file and
send `SIGHUP` to the intended process. The reload path:

1. reads at most the bounded registry size in a blocking task outside the
   asynchronous request executor;
2. parses and validates every provider key with the existing cryptographic
   parser, including key count, identifier, encoding, and Ed25519 validity;
3. atomically swaps one immutable `Arc<HashMap<key_id, Ed25519 key>>` only
   after complete validation; and
4. retains the last known-good registry after every read, parse, size, or
   key-validation failure.

The request path loads one immutable registry snapshot and verifies all
evidence against that snapshot. It performs no reload I/O. Structured
`API_JURISDICTION_PROVIDER_KEYS_JSON` and the compatibility key pair remain
supported for startup-only/local compatibility. File and inline forms are
mutually exclusive. A file-backed registry is required to be present and
valid at startup; an invalid later replacement is observable but does not
degrade the currently active registry.

The identity of the provider, overlap/revocation timing, secret-manager
atomic delivery, file ownership/permissions, and legal authority remain
deployment responsibilities. Removing a key affects newly verified evidence;
already persisted evidence remains bound to its original provider key and
facts digest.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| Restart-only rotation | Rejected | Expands emergency response time and couples evidence authority changes to deployment scheduling. |
| Fetch provider keys from a network on every quote | Rejected | Adds unbounded hot-path latency and makes quote authorization dependent on an external service. |
| Periodic background polling | Rejected for this first contract | Requires provider authentication, stale-cache, backoff, split-brain, and alert policy before it is safe. |
| Mutable map behind `RwLock` | Rejected | Introduces request-path lock contention and makes verification latency sensitive to reload timing. |
| **Atomic immutable snapshot with operator-triggered file reload** | **Selected** | Preserves deterministic hot-path cost, bounded validation, last-known-good safety, and explicit deployment ownership. |

## Invariants and failure behavior

- A request observes exactly one fully validated provider-key snapshot.
- No request can observe a partially parsed or partially replaced registry.
- Missing, unreadable, oversized, malformed, duplicate, or invalid-key
  replacements retain the last known-good snapshot.
- Startup fails closed when a configured registry file cannot be read or
  validated; simulation mode may omit provider verification as already defined
  by the deployment environment contract.
- File-backed, structured-inline, and compatibility-pair forms are mutually
  exclusive.
- Reload logs and metrics never contain key bytes, signatures, raw evidence,
  merchant identifiers, or request payloads.
- Provider key count and file size are bounded, and reload filesystem work is
  never executed on the request executor.
- Persisted tax evidence remains immutable and does not change retroactively
  when the active provider registry rotates.

## Verification and release impact

- Unit tests prove valid replacement, removed-key rejection, malformed
  replacement retention, duplicate-key rejection, and file-size bounds.
- A real API Docker contract verifies a valid provider evidence envelope before
  rotation, rejects it after its key is removed, accepts a replacement key,
  and retains the replacement after a malformed reload.
- Deployment preflight and API startup tests prove file/inline exclusivity.
- Controlled staging still requires provider-managed overlap/revocation,
  HSM/KMS custody, atomic secret delivery, permissions, reload timing,
  regional evidence, and an approved tax-data provider ceremony.
