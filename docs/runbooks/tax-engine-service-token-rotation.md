# Runbook: Tax Engine service-token rotation

This runbook covers bearer-credential rotation for the Tax Engine service. It
does not replace the workload-identity and Rustls mTLS rotation runbook; in
staging and production both controls remain active.

## Preconditions

- Use an approved secret manager or deployment controller that can write a new
  token to a temporary file, set owner and mode for the non-root service user,
  atomically rename it into place, and send `SIGHUP` to the Tax Engine process.
- Keep the old credential valid for the documented overlap window until all
  consumers have reloaded and the new credential is confirmed.
- Never put the token in a command argument, log message, metric label, trace,
  ticket, or shell history.

## Procedure

1. Generate a new cryptographically random token within the service limit of
   256 characters.
2. Deliver it to the configured `TAX_ENGINE_AUTH_TOKEN_FILE` using an atomic
   same-filesystem rename. Do not truncate or modify the active file in place.
3. Send `SIGHUP` once to the Tax Engine process. The service validates the
   replacement outside the async request executor and publishes it as one
   immutable snapshot. If TLS files are also configured, token and TLS
   validation succeed or neither snapshot changes.
4. From an approved client, call `/readyz` with the new credential and verify
   the expected `204` response. Confirm the old credential behavior matches the
   approved overlap policy.
5. Check bounded reload-success/reload-rejection telemetry, process health,
   authentication failure rate, and calculation latency. Do not query or print
   the credential itself.
6. After the overlap window, revoke the old credential at the secret issuer
   and repeat the readiness and failure-path checks.

## Failure handling

- A missing, unreadable, oversized, non-UTF-8, empty, or locally reserved
  replacement is rejected; the last known-good token remains active.
- A malformed TLS replacement similarly leaves the complete active runtime
  configuration unchanged.
- If the new credential is not accepted after the bounded reload window,
  stop further rotation attempts, preserve the last known-good file, inspect
  deployment permissions and signal delivery, and page the service owner.
- If compromise is suspected, follow the incident-response procedure, revoke
  the exposed credential, rotate the mTLS workload identity as required, and
  record the event without retaining secret material.

## Evidence

Record only non-secret metadata: change/incident identifier, target service
revision, secret version identifiers, rotation timestamps, signal delivery
status, readiness status codes, reload outcome, overlap/revocation timestamps,
and operator identity. The real-container contract is
`scripts/verify_tax_engine_runtime_contract.sh`; the architecture and failure
semantics are defined by [ADR-0237](../adr/0237-tax-engine-service-token-rotation.md).
