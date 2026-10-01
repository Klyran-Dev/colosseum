# ADR-0218: Explicit release integrity hashes

- Status: Accepted
- Date: 2026-10-01
- Deciders: Principal Low-Latency Architecture, Security, Release Engineering
- Bounded contexts: Release Engineering, Solana Program Delivery, Deployment Configuration

## Context

ADR-0129 creates a release-candidate bundle and hashes every copied artifact,
but consumers had to infer which hash represented the Solana program and which
file represented the effective deployment configuration. A production-grade
release review needs named, machine-readable integrity claims for both. The
Compose configuration rendered by the release workflow is the resolved input
for the selected local/deployment topology; it must be reviewed as bytes, not
reconstructed from source after the build.

## Decision

`scripts/create_release_bundle.sh` requires the workflow-produced
`release-bundle/compose-config.yml`, copies it into the bundle as
`artifacts/config/compose-config.yml`, and includes it in the sorted checksum
set. The release manifest retains the existing complete artifact list and adds
an explicit `integrity` object:

```json
{
  "integrity": {
    "program_binary": {
      "path": "artifacts/solana/settlement.so",
      "sha256": "..."
    },
    "effective_configuration": {
      "path": "artifacts/config/compose-config.yml",
      "sha256": "..."
    }
  }
}
```

The values are calculated from the final copied bytes. The manifest is
descriptive evidence only: it does not sign, publish, deploy, authorize a
program upgrade, or replace multisig/upgrade-authority controls.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| Keep the generic artifact list only | Rejected | Reviewers must infer authority-sensitive paths and consumers need brittle path conventions. |
| Hash source Compose files instead of resolved configuration | Rejected | Profiles, interpolation, and image digests can change the effective topology. |
| Add a second detached hash file with no manifest binding | Rejected | Creates two sources of truth and weakens machine-readable release inspection. |
| Hash only the program binary | Rejected | A changed runtime topology can alter operational and security behavior. |
| **Hash final program and resolved configuration bytes in named manifest fields and the common checksum set** | **Selected** | Explicit, reproducible, machine-readable, and compatible with existing bundle verification. |

## Invariants and failure policy

- Missing resolved configuration fails bundle creation before any complete
  release artifact is reported.
- The program hash is calculated from the final copied `settlement.so` bytes.
- The configuration hash is calculated from the final copied Compose bytes.
- Both named paths appear in `checksums.sha256` and in the manifest artifact
  list.
- A checksum or manifest mismatch fails closed; no deployment is triggered.
- The bundle contains no credentials, private keys, or production secrets.
- A successful local contract test is not validator, audit, live-provider, or
  production-SLO evidence.

## Verification and release impact

- `scripts/release_bundle_integrity_contract.sh` creates a bounded synthetic
  conformance fixture and verifies the manifest, copied configuration, and
  checksum entries.
- The release-candidate workflow runs the contract after locked binaries,
  web assets, and the SBF artifact have been built.
- `make public-docs-check` requires the ADR and contract script.
- A successful retained Linux release-candidate run and independent release
  review remain required before production enablement.
