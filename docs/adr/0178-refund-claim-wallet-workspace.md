# ADR-0178: Two-signer refund-claim wallet workspace

## Status

Accepted — implementation boundary complete; live wallet/provider and
finalized-proof evidence remain release gates.

## Context

The server-owned refund claim API now persists an immutable artifact and
accepts only exact signed bytes. A production UI must make the two required
signer roles explicit, keep the artifact reviewable, support recovery after a
browser reload without fabricating bytes, and distinguish wallet/provider
transport state from a completed refund.

## Decision

Add a lazy React workspace that:

- prepares a claim using only an approved adjustment ID, idempotency key, and
  explicit server confirmation;
- renders server-owned gross, tax, net, mint, destination, claim PDA,
  authorities, expiry, and transport evidence;
- validates the exact fifteen-account legacy-SPL claim shape in the browser as
  defense in depth, while retaining the API as the authority;
- collects the proposer signature and configured refund-funder signature as
  separate wallet actions over one unchanged transaction artifact;
- keeps partial signed bytes in memory and accepts an explicit paste of exact
  base64 bytes for recovery after reload; and
- blocks submission unless both signatures are present and states clearly that
  provider acknowledgement is not finalized refund proof.

The UI never accepts browser-supplied amounts, accounts, nonce, or signer
topology. It never stores private keys, signed bytes in durable browser storage,
or claims legal compliance, filing, or refund completion.

## Alternatives considered

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| One generic checkout panel | Less UI code | Hides two-signer authority and encourages wrong signing assumptions | Rejected |
| Server custody of the refund-funder key | One-click UX | Violates non-custody and key-separation mandates | Rejected |
| Browser builds the refund transaction from editable fields | Flexible | Reopens account, amount, nonce, and stale-evidence substitution | Rejected |
| Persist signed bytes in browser storage | Easier reload recovery | Sensitive transaction material survives beyond the explicit wallet session | Rejected |
| **Lazy server-artifact review with sequential role signing and explicit memory-only recovery** | Explicit consent, bounded validation, two-signer clarity, and no client financial authority | Requires two wallet interactions and manual recovery input after reload | **Selected** |

## Invariants and failure policy

- account order, signer flags, program ID, discriminator, and instruction size
  are checked before a wallet prompt;
- the proposer and refund-funder public keys must match their compiled roles;
- partial signatures are never sent as a complete submission;
- an unknown provider outcome stops further signing/submission and requires
  durable reconciliation; and
- only finalized transaction-bound event and balance proof can describe a
  completed refund.

## Verification

Parser tests cover the shared cross-language claim vector, account substitution,
canonical decimal values, nullable transport evidence, and accessibility checks
cover the workspace's keyboard and error surfaces. Component interaction tests
now exercise two deterministic injected-wallet identities, sequential role
signing, exact submission, and the proof-pending transport state for both claim
and execute workspaces. Real wallet-provider interaction, browser reload,
validator RPC, and finalized-proof evidence remain release gates.
