# ADR-0140: Validator-backed Split Plan settlement conformance

- Status: Accepted
- Date: 2026-09-27
- Deciders: Principal Low-Latency Architecture, Solana Protocol, Security
- Scope: Pinned local validator conformance and release evidence

## Context

The bounded Split Plan context already has a pure Rust allocator, a shared
browser vector, an immutable Split Plan PDA, and a compiler artifact. Those
checks do not prove that the deployed SBF program performs the recipient fan-
out atomically, preserves gross conservation, binds the immutable plan, or
rejects a caller-controlled account order. The production-readiness checklist
must remain open until a real validator executes this boundary.

## Decision

Extend the existing validator-backed TypeScript conformance suite with a fresh
legacy-SPL mint and a two-recipient immutable Split Plan. The test will:

1. initialize the protocol, issuer registry, policy attestation, and Split
   Plan with 60/40 basis points;
2. settle gross 100 atomic units as tax 25 and net 75;
3. assert finalized balances of source 0, tax vault 25, and recipients 45/30;
4. submit the same nonce again and assert the deterministic receipt PDA blocks
   a second movement; and
5. submit a new nonce with recipient accounts in the wrong order and assert the
   transaction fails before any balance changes.

All amounts, plan bytes, account metas, and PDAs are constructed in the test;
the browser or API is not treated as an authority. Existing tests continue to
cover single-recipient legacy SPL, extension-free Token-2022, and unsupported
Token-2022 extensions. The suite remains a mandatory CI/release gate and is
not downgraded to a mock when the validator is unavailable.

## Alternatives considered

### Unit-test the allocator only

Rejected. Pure arithmetic cannot prove Anchor account constraints, CPI token
movement, remaining-account order, receipt initialization, or transaction
rollback.

### Use `solana-program-test` only

Rejected as release evidence. It is useful for fast program tests but does not
exercise the pinned Agave process, deployed SBF artifact, external JSON-RPC,
or the exact client serialization boundary.

### Add a second synthetic RPC fixture

Rejected. A fixture can model responses but cannot prove token program CPI,
account ownership, or atomic rollback.

### Selected: extend the pinned external validator suite

Selected because it is the narrowest deterministic test that exercises the
actual program, token accounts, transaction serialization, and finalized
ledger state while reusing the existing evidence bundle and CI runtime.

## Invariants and failure behavior

- `gross_atomic = tax_atomic + net_atomic` and finalized token balances conserve
  the input amount.
- The Split Plan PDA digest and version are bound to the settlement instruction.
- Remaining recipient accounts must match immutable plan order and mint.
- A duplicate receipt nonce cannot transfer funds a second time.
- Wrong recipient order fails before CPI transfers, leaving all balances
  unchanged.
- Missing validator, SBF artifact, RPC readiness, or any failed assertion fails
  the conformance command and is retained as failure evidence.

## Evidence and remaining gates

The test output, validator logs, Compose configuration, program hash, and
runtime metadata are retained by the existing conformance harness. This closes
only the local validator Split Plan evidence once CI executes successfully; a
program audit, target-cluster compute/priority-fee measurements, staging
governance, and production deployment approval remain separate gates.
