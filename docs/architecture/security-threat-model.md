# Security and threat model

## Security objectives

- No settlement can create or destroy value outside the configured token program rules.
- Tax and fee deductions are deterministic, bounded, and attributable to a signed policy digest.
- No replay can cause a second successful settlement.
- No API, frontend, oracle, or indexer key can unilaterally move vaulted funds.
- Every successful settlement has a verifiable on-chain receipt and enough evidence for reconciliation.
- Emergency containment cannot silently rewrite history or confiscate funds outside the published release policy.

## Threats and mitigations

| Threat | Mitigation |
| --- | --- |
| Compromised policy signer | Threshold signer set, HSM/KMS, key rotation, expiry, sequence monotonicity, rate bounds, pause/revoke path, audit trail. |
| Malicious API changes tax amount | Program recomputes from the policy and canonical inputs; caller-provided amount is an assertion only. |
| Browser or API replay | Merchant-scoped client nonce and receipt PDA uniqueness. |
| Wrong token mint/program | Asset registry checks mint, decimals, token program owner, and allowed extensions. |
| Token-2022 transfer-hook or fee surprise | Explicit extension policy; reject unsupported hooks/fees in v1 or include them in a tested adapter. |
| Recipient account substitution | Split plan is on-chain and versioned; compiler verifies every account against the plan. |
| Rounding theft | Integer arithmetic, explicit rounding, amount-conservation invariant, property tests. |
| Vault rug pull | PDA authority, no service private key, release claims, allow-lists, timelock/multisig governance, separation of duties. |
| RPC censorship or equivocation | Multi-provider submission/read paths, signature-based status, finalized backfill, provider health scoring. |
| Indexer omission | Raw event retention, slot checkpoints, gap detection, chain-state reconciliation. |
| DDoS and quote abuse | NGINX limits, merchant/API quotas, cacheable public metadata, bounded payloads, no expensive chain calls before authentication. |
| Supply-chain compromise | Locked dependencies, SBOM, signed build artifacts, reproducible CI, audit of program upgrades. |
| Upgrade authority compromise | Multisig/timelock, staged deployment, verified program binaries, explicit upgrade/rollback policy. |
| PII leakage | Keep jurisdiction evidence off-chain or store only commitments; encrypt sensitive data; minimize logs. |

## Authority model

| Authority | Can | Cannot |
| --- | --- | --- |
| Payer | sign a payment and approved delegate spend | alter policy, vault rules, or split plan |
| Merchant admin | configure merchant, accepted assets, and split plans within governance rules | withdraw tax reserves directly |
| Policy issuer set | publish/revoke policy attestations | transfer customer funds or release vault balances |
| Vault release governance | approve a valid release claim | change historical receipts |
| Program upgrade multisig | deploy a reviewed binary through the upgrade process | bypass transaction invariants in an already deployed immutable version |
| API service | prepare and submit signed transactions from clients | sign for users or custody funds |

## Audit gates

Before mainnet:

- independent Solana program audit;
- property, fuzz, mutation, and adversarial account tests;
- Token-2022 extension matrix tests;
- key ceremony and recovery exercise;
- upgrade and pause tabletop exercise;
- RPC failover and indexer replay drill;
- legal review of policy issuance, escrow, custody, and reporting claims;
- dependency/SBOM and container image scan.

