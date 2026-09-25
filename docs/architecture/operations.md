# Operations, observability, and SRE

## Source-of-truth model

The Solana ledger, verified program logs, token balances, and settlement receipt accounts are authoritative. PostgreSQL contains rebuildable projections. Redis/cache entries are disposable. No business decision may depend on an unverified WebSocket notification.

## Telemetry

Emit OpenTelemetry traces and metrics with a correlation ID carried through quote, compile, wallet handoff, submit, signature lookup, index, and reconciliation.

Required metrics:

- `settlement_quote_latency_ms` and `settlement_compile_latency_ms`;
- `settlement_submit_latency_ms` by RPC provider and region;
- confirmation latency by commitment level;
- transaction success/failure by program error code;
- compute units requested/consumed and priority fee;
- stale policy and policy mismatch rejects;
- duplicate nonce rejects;
- indexer slot lag, event gap count, replay duration;
- vault balance delta versus receipt-derived balance;
- API rate-limit rejects and wallet-signature abandonment.

## Logging rules

- Never log private keys, seed phrases, raw wallet signatures when not needed, PII, or full policy evidence.
- Log public correlation identifiers, signature, slot, receipt PDA, policy digest, merchant ID, asset mint, and error code.
- Use structured JSON logs and a redaction test in CI.
- Preserve immutable raw chain evidence for the configured retention period.

## SLOs and alerts

Initial SLOs:

- 99.95% availability for read APIs;
- 99.9% successful submission acknowledgement excluding user rejection;
- 99.99% of finalized settlements represented in projections within 60 seconds;
- zero unexplained conservation mismatches;
- zero unauthorized vault-release events.

Page on:

- conservation mismatch or unexpected vault outflow;
- upgrade authority or policy signer change outside a change window;
- indexer gap beyond one slot checkpoint interval;
- RPC provider disagreement or elevated dropped transaction rate;
- finalization latency burn-rate breach;
- policy issuer key compromise or expired key set.

## Deployment model

- Build the React SPA once, publish content-addressed assets, and serve through NGINX.
- Run Rust API and indexer as separate deployables with independent scaling and permissions.
- Use blue/green or canary releases for off-chain services.
- Deploy Solana programs through devnet -> testnet/staging -> mainnet, with verified binaries and an approved upgrade authority.
- Every deployment records git commit, toolchain digest, program binary hash, IDL/schema version, and configuration version.

