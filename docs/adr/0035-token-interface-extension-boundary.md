# ADR-0035: Use the token interface with an extension-free Token-2022 boundary

- Status: Accepted for the first Token-2022 compatibility slice
- Date: 2026-09-25
- Owners: Protocol / Security
- Scope: settlement mint, token-account, and transfer-program compatibility

## Context

The first settlement instruction uses the legacy SPL Token wrapper. The target
protocol must support both the legacy SPL Token program and Token-2022, but
Token-2022 extensions change transfer semantics and authority behavior. A
generic CPI that accepts every extension could silently violate tax/net amount
conservation or invoke unreviewed hooks.

The low-latency path must keep one bounded instruction shape and must reject
unsupported assets before any token CPI.

## Decision

Use Anchor's `token_interface` types for the mint, token accounts, and token
program. Accept either the legacy SPL Token program or Token-2022 only when:

- the mint and all token accounts are owned by the same selected token program;
- the mint uses the base account layout (82 bytes); and
- each token account uses the base account layout (165 bytes).

Those size checks intentionally reject Token-2022 extensions, including
transfer fees, transfer hooks, default-frozen accounts, delegates, confidential
transfer state, metadata/group extensions, and any future extension not yet
covered by a compatibility contract. The settlement code continues to use
`TransferChecked`, verifies mint decimals and account identities, and performs
the same conservation checks for both accepted programs.

Each future extension is a separate ADR and test matrix. It must define its
economic effect, authority model, account constraints, event/evidence meaning,
and exact conservation invariant before being admitted.

## Alternatives considered

See `docs/architecture/alternative-analysis.md` under “Token program
compatibility”. Separate instructions duplicate security-sensitive logic;
accepting all extensions fails closed behavior; legacy-only support does not
meet the target compatibility architecture.

## Consequences

Positive:

- Legacy SPL and extension-free Token-2022 use one explicitly bounded path.
- Token-2022 extensions cannot change transfer outcomes behind the protocol's
  back.
- The program remains low-latency and the audit surface stays concentrated.

Costs:

- Many real Token-2022 assets are rejected until their extension semantics are
  separately designed and audited.
- Localnet integration must exercise both program IDs and rejection paths.
- Account-length checks are a compatibility policy, not a substitute for future
  extension-specific validation.
