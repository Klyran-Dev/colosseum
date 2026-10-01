# ADR-0214: Extension-free Token-2022 Split Plan conformance

- Status: Accepted
- Date: 2026-09-30
- Deciders: Principal Low-Latency Architecture, Solana Protocol, Security
- Bounded contexts: Settlement program, Token compatibility, Validator evidence

## Context

The settlement program deliberately uses one Anchor-compatible token-interface
path for legacy SPL Token and extension-free Token-2022 accounts. The existing
validator suite proves extension-free Token-2022 for a single-recipient
settlement, while ADR-0140 proves Split Plan fan-out only with legacy SPL.
Those two facts do not prove that the remaining-account validation and
multi-recipient CPI loop preserve the same compatibility boundary for
Token-2022.

The product must make a precise compatibility claim: extension-free
Token-2022 is supported for bounded settlement shapes; transfer fees, hooks,
delegates, confidential transfers, and other unsupported extensions are not
implicitly supported and must remain rejected before activation or movement.

## Decision

Extend the pinned external-validator conformance suite with a separate
extension-free Token-2022 two-recipient Split Plan case:

1. create a zero-decimal Token-2022 mint with no extensions;
2. create source, vault, and two recipient accounts owned by the Token-2022
   program;
3. initialize the merchant/mint issuer registry, policy attestation, and a
   60/40 immutable Split Plan;
4. execute the bounded `settle_split` instruction for gross 100, tax 25, and
   net 75; and
5. read finalized balances and require source 0, tax vault 25, recipient A
   45, and recipient B 30.

The case uses the same exact account ordering, receipt PDA, policy digest, and
conservation assertions as the legacy-SPL case. It does not add an alternate
instruction, silently normalize extension behavior, or accept a Token-2022
extension. The existing transfer-fee-extension rejection case remains a
separate negative control.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| Rely on the existing single-recipient Token-2022 case | Rejected | It does not exercise Split Plan remaining-account ordering or multi-CPI fan-out. |
| Add a second Token-2022-specific on-chain instruction | Rejected | Duplicates authority, conservation, and replay logic and expands the audit surface. |
| Treat every Token-2022 extension as compatible | Rejected | Transfer fees, hooks, delegates, and confidential semantics can invalidate amount and authority invariants. |
| **Reuse `token_interface` with a dedicated extension-free Split Plan validator case** | **Selected** | Preserves one bounded implementation while proving the exact deployed SBF and account topology. |

## Invariants and failure behavior

- The mint and every token account are owned by Token-2022, and all accounts
  bind to the same mint.
- `gross_atomic = tax_atomic + net_atomic`, and finalized balances equal the
  exact 0/25/45/30 expected vector.
- The immutable Split Plan digest, version, recipient order, policy digest,
  and receipt nonce remain bound to the instruction.
- Unsupported extensions fail closed before registry activation or token
  movement; no compatibility claim is inferred from a mocked account.
- Missing validator, unavailable RPC, failed finality, or any balance mismatch
  fails the conformance command and retains evidence.

## Verification and release impact

The TypeScript validator client contains the case and type-checks in the
pinned conformance image. A successful Linux/amd64 run is required before the
Token-2022 Split Plan item is marked complete. The local macOS Docker
`io_uring` failure, synthetic RPC fixtures, and unit tests do not satisfy this
release evidence gate.
