# Runbook: Tax Engine mTLS certificate rotation

## Scope and authority

Use this runbook for staging or production rotation of the Tax Engine server
certificate, private key, or client-CA bundle. The deployment secret manager
owns issuance, storage, permissions, atomic file replacement, and revocation.
The Tax Engine process only validates and serves the supplied material. A
successful reload changes transport identity; it does not authorize tax
calculation, settlement, policy publication, or vault release.

Local Compose uses plain private-network HTTP by design. Do not copy local
certificate paths, local tokens, or synthetic identities into a non-local
deployment.

## Preconditions

- Open an approved change and incident reference when rotation is emergency
  work.
- Confirm the target service, region, deployment environment, certificate
  subject/SAN, validity window, client-CA trust policy, and rollback material.
- Confirm the replacement certificate matches the replacement private key and
  the client CA contains the intended workload issuer.
- Confirm the secret manager writes each replacement file atomically, with
  restrictive permissions, and that the service account can read the files
  without exposing their contents to logs or operator terminals.
- For CA rotation, use an approved overlap plan: trust the new CA before
  retiring the old CA, verify all clients, then remove the old CA in a second
  change. Never replace a CA bundle blindly during an incident.

## Rotation procedure

1. Capture the change ID, service revision, active certificate fingerprint,
   active CA version, region, and current readiness/transport health. Record
   fingerprints and version identifiers only; never record private-key or
   bearer-token contents.
2. Atomically replace the certificate and private-key files. Replace the
   client-CA bundle as a single approved trust-policy change; do not edit files
   in place or write them through a shared temporary path.
3. Send `SIGHUP` to the intended Tax Engine process using the deployment
   supervisor's controlled signal mechanism. Do not use a shell wildcard or a
   signal that could target another service.
4. Confirm the service remains ready and that the structured log records a
   successful mTLS reload for the expected revision. The log must not contain
   certificate or key material.
5. From an approved client, establish a new mTLS connection and verify the
   expected server certificate, client-CA trust, ALPN, and authenticated
   readiness response. Existing connections may remain on the previous
   snapshot until they naturally close.
6. Confirm the calculation endpoint still requires its bearer token and that
   no quote, attestation, settlement, or evidence row was created by the
   transport probe.
7. Record the new fingerprint/version, reload outcome, connection result, and
   evidence references. Keep the previous material available only for the
   approved rollback window, protected by the secret manager.

## Fail-closed behavior and rollback

If the replacement certificate, key, CA, verifier, or ALPN policy is invalid,
the reload is rejected and the last known-good Rustls configuration remains
active. Do not retry in a tight loop and do not downgrade to plaintext.

If readiness or a new-client handshake fails after a successful reload:

1. Stop further rotation and preserve logs, process status, fingerprints,
   deployment revision, and the exact change reference.
2. Use the supervisor's controlled rollback to atomically restore the last
   known-good complete file set, then send one controlled `SIGHUP`.
3. Verify a new trusted-client handshake and authenticated readiness before
   resuming traffic.
4. If the old material is expired, revoked, or suspected compromised, do not
   roll it back. Stop the affected non-local Tax Engine route, preserve the
   unknown state, and escalate to security/SRE for a new trusted issuance.

Signal-stream failure, process termination, client-CA compromise, provider
identity disagreement, or inability to prove the active certificate is the
approved one is a release/security incident. Preserve the state and follow the
[incident severity runbook](incident-severity-and-error-budget.md).

## Evidence and post-change checks

Attach:

- change and incident references;
- service revision, environment, region, and deployment hash;
- old/new certificate and CA fingerprints or secret-manager version IDs;
- reload timestamp and structured outcome;
- trusted-client handshake and authenticated readiness result;
- rejection/rollback evidence if the first attempt failed;
- alert, dashboard, and on-call notification results.

Never attach private keys, seed phrases, bearer tokens, raw client
certificates, PII, or raw jurisdiction evidence. Production acceptance still
requires regional reload latency, connection-drain, CA revocation, secret
manager, and alert/on-call evidence in the readiness plan.
