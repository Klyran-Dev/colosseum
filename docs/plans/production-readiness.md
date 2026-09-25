# Production-readiness plan

## Correctness

- [ ] Canonical serialization has test vectors in Rust and TypeScript.
- [ ] Amount conservation is checked on-chain and in property tests.
- [ ] `u64` overflow, zero amount, cap, rate, and rounding boundaries are covered.
- [ ] Legacy SPL Token and Token-2022 account ownership is checked.
- [ ] Unsupported Token-2022 extensions are rejected or explicitly tested.
- [ ] Policy issuer, sequence, expiry, revocation, and merchant binding are enforced.
- [ ] Replay, duplicate nonce, stale blockhash, and duplicate submission are covered.
- [ ] All failed paths prove no partial token movement.
- [ ] Tax-engine golden vectors cover simple, progressive, tax-on-tax, cascading, inclusive/exclusive, min/max, threshold, exemption, credit, truncation, and rounding cases.
- [ ] Rule-graph cycle detection and dependency ordering are enforced.
- [ ] Published rule versions are immutable and cannot overlap within the same jurisdiction/tax regime without an explicit precedence decision.
- [ ] Calculation traces replay exactly from normalized facts, rule digest, engine version, and evidence references.
- [ ] PostgreSQL 18.x is deployed with the latest security patch, pinned image digest, backup/restore evidence, and migration rollback plan.
- [ ] Internal IDs use native PostgreSQL `uuid` with UUIDv7 where sortable distributed IDs are appropriate; secrets, hashes, Solana IDs, and strict sequence values use the correct alternatives.
- [ ] Database lock/statement timeouts, pool limits, partition/retention policies, and index plans are load-tested.

## Security

- [ ] Program audit complete with no unresolved critical/high findings.
- [ ] Upgrade authority, policy signer, and vault release keys are separated.
- [ ] HSM/KMS and multisig ceremonies are documented and rehearsed.
- [ ] Frontend/API do not receive seed phrases or user private keys.
- [ ] Dependency, container, SBOM, and secret scans pass in CI.
- [ ] Fuzzing and adversarial account-meta tests run on every program change.
- [ ] Rate limits, authentication, authorization, CORS, CSP, and NGINX headers are tested.
- [ ] Rule publishing requires separation of author, reviewer, and approver duties.
- [ ] Sensitive jurisdiction evidence is minimized, encrypted, and access audited.

## Performance

- [ ] p50/p95/p99 quote, compile, submit, processed, confirmed, and finalized latency is measured.
- [ ] Load test covers account-lock contention and recipient fan-out bounds.
- [ ] Compute-unit and transaction-size budgets have safety margins.
- [ ] v1 and v0+ALT transaction paths are tested on target clusters/RPCs.
- [ ] Multi-provider failover and degraded RPC behavior are tested.

## Operations

- [ ] Structured logs, traces, metrics, and alert routing are live.
- [ ] Indexer backfill and replay are automated.
- [ ] Vault balance reconciliation is scheduled and alerting.
- [ ] Runbooks cover policy compromise, upgrade rollback, vault release incident, RPC outage, and indexer gap.
- [ ] Mainnet configuration is separate from staging and requires approval.
- [ ] Releases are reproducible and record program binary and configuration hashes.
- [ ] SLOs, error budgets, RTO/RPO, support ownership, and incident severity policy are published.
- [ ] Backup restore, key recovery, RPC failover, indexer replay, and rule rollback drills pass.
- [ ] Vendor/RPC exit plan and dependency register are current.

## Legal and product

- [ ] Counsel reviewed tax, custody, money transmission, reporting, privacy, and jurisdictional claims.
- [ ] Merchant terms identify who is responsible for tax registration and filing.
- [ ] Policy issuer authority and evidence provenance are documented.
- [ ] Stablecoin issuer and redemption/off-ramp dependencies are documented.
- [ ] User-facing copy says “automated withholding and evidence,” not “guaranteed compliance.”
- [ ] Each published rule has legal/tax owner, source provenance, effective period, review date, and jurisdiction scope.
- [ ] UI clearly distinguishes calculated tax, attested policy, and legal responsibility.

## User experience

- [ ] Checkout explains tax before wallet signing and shows the policy/rule version.
- [ ] “Why this tax?” exposes a human-readable calculation trace without leaking private evidence.
- [ ] Rule authoring has draft, simulation, review, approval, publish, and rollback states.
- [ ] UI meets WCAG 2.2 AA, supports keyboard navigation, screen readers, responsive layouts, localization, currencies, time zones, and RTL.
- [ ] Loading, stale-policy, expired-quote, wallet rejection, RPC timeout, and reconciliation-required states are designed and tested.

## Public repository

- [ ] README accurately states status, scope, limitations, architecture, and reproducible setup status.
- [ ] `AGENTS.md`, `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`, and `CHANGELOG.md` are current.
- [ ] License choice is explicit before public source release.
- [ ] ADR index, coverage matrix, API/IDL documentation, and release notes are linked from the README.
- [ ] CI status, security scanning, dependency updates, and artifact provenance are visible without overstating readiness.
