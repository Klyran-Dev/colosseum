# Operations runbooks

These runbooks are the public, product-level response contract. They contain
no credentials, private evidence, or environment-specific hostnames. Each
production deployment must bind the roles and placeholders to its approved
on-call, identity, signer, RPC, and change-management systems before launch.

Every incident follows the same authority rule: Solana finalized state and
verified program evidence are authoritative; PostgreSQL, Redis, dashboards,
wallet notifications, and provider subscriptions are projections or signals.
When evidence is ambiguous, stop settlement or release work and preserve the
unknown state for reconciliation.

## Runbooks

- [Policy compromise](policy-compromise.md)
- [Program upgrade rollback](upgrade-rollback.md)
- [Vault release incident](vault-release-incident.md)
- [RPC outage or provider disagreement](rpc-outage.md)
- [Indexer gap and projection repair](indexer-gap.md)
- [Backup and restore](backup-restore.md)
- [Tax Engine mTLS certificate rotation](tax-engine-mtls-rotation.md)
- [Tax Engine service-token rotation](tax-engine-service-token-rotation.md)
- [Incident severity and error-budget response](incident-severity-and-error-budget.md)
- [Settlement-route verification](settlement-route-verification.md)

## Required incident record

Record the incident ID, severity, start time, affected environment, operator
subjects, change/reference IDs, program ID, route/policy digests, relevant
signatures and slots, exact commands/actions, and the final evidence digests.
Redact private keys, seed phrases, bearer tokens, PII, and raw jurisdiction
evidence. Never edit or delete historical evidence to make an incident appear
resolved.
