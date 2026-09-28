# External dependency register and exit plan

Status: public, provider-neutral baseline. This register is not an approval to
deploy a provider or custody key. Every production environment must add a
reviewed binding with owner, contract, region, data-processing terms, and
tested credentials/key ceremony.

| Dependency class | Authority/data boundary | Failure behavior | Replacement/exit path | Accountable role | Review |
| --- | --- | --- | --- | --- | --- |
| Solana/Agave cluster | Finalized ledger, program execution, token balances, receipts | Stop new settlement/release actions when commitment or cluster health is ambiguous; preserve unknown intents | Cluster migration runbook, program/config compatibility review, finalized replay and reconciliation | Settlement Platform | Each release and protocol upgrade |
| RPC providers | Transport and read/submission access; never financial authority | Bounded timeout, provider scoring, immutable-byte retry, provider disagreement becomes reconciliation-required | Provider-neutral `RpcProvider` port, second provider, exported raw evidence and signatures | SRE + Settlement | Monthly and after outage |
| PostgreSQL 18 | Durable projections, intents, evidence metadata, outbox fence | Fail closed for new quotes/settlements; never promote cache/event stream to authority | Logical/physical backup, PITR, schema expand/contract, tested restore into an independent PostgreSQL 18 cluster | Data Platform | Monthly restore, every migration |
| Redis | Disposable cache, lease/coordination hints | Bypass or reject according to bounded cache policy; never use as ledger | Remove cache dependency; warm from PostgreSQL/chain evidence | Platform | Quarterly |
| NATS JetStream | At-least-once asynchronous delivery only | Keep outbox durable; bounded retry/dead-letter; no settlement authority | PostgreSQL outbox dispatcher can be paused or replaced by another consumer; replay from outbox | Platform | Quarterly and before topology change |
| OIDC/JWKS provider | Authentication keys and principal claims | Reject unknown issuer/audience/key/expiry/revocation state; preserve audit evidence | Standard OIDC issuer replacement with overlapping key ring, dual verification window, tenant re-authentication plan | Identity/Security | Monthly key-rotation test |
| Policy HSM/KMS and multisig | Attestation, governance, upgrade, and release-key operations | Do not issue or accept new privileged evidence; pause affected authority | Versioned signer port, key export prohibited, ceremony-backed replacement, attestation revalidation | Security/Governance | Quarterly ceremony |
| SPL Token and Token-2022 programs | Token account/mint semantics supplied to bounded CPI | Reject unsupported extensions, owner/program mismatch, or ambiguous semantics before CPI | Versioned token-interface adapter and new route activation; never reinterpret historical evidence | Settlement Security | Each supported extension change |
| Wallet adapters | User signature and client-side key boundary | Show explicit rejection/timeout/unknown state; never fabricate submission | Versioned transaction artifact and another standards-compatible wallet | Product UI | Each browser release |
| NGINX and browser delivery | Static asset and bounded API edge | Serve last known immutable asset or fail closed; security headers remain mandatory | Static artifacts can move to another standards-compliant edge; API contracts remain independent | Product UI/SRE | Each release |
| OpenTelemetry/Prometheus/Grafana | Diagnostic telemetry, not business authority | Preserve structured logs and local metrics; do not block settlement on telemetry | OpenTelemetry protocol export and Prometheus-compatible rules; query retained evidence directly | SRE | Quarterly |
| GitHub Actions and artifact storage | CI execution and retained release evidence | A release candidate is unproven if required jobs/artifacts are missing | Reproduce from lockfiles, Dockerfiles, scripts, and public evidence contract on another CI runner | Release Engineering | Each release |
| Stablecoin issuer/off-ramp | Asset redemption/liquidity outside Solana settlement core | No implicit redemption or legal-liquidity claim; surface asset dependency state | Asset route registry, issuer-neutral asset adapter, merchant-controlled migration and reconciliation | Finance/Legal | Before each asset launch |

## Exit-control requirements

- No dependency may be the only copy of a settlement signature, policy digest,
  tax facts digest, or reconciliation evidence.
- Every adapter exposes a bounded port with explicit deadlines, retry budget,
  cancellation, and failure classification.
- A replacement must pass contract vectors, security review, data-retention
  review, load evidence, and the relevant restore/failover drill before it is
  eligible for production traffic.
- Provider-specific identifiers remain in bounded configuration or structured
  logs; they must not leak into unbounded metric cardinality or domain rules.
- The exit plan does not imply data portability rights or contract terms. Legal
  and procurement owners must verify those separately.
