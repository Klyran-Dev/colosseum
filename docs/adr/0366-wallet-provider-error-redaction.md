# ADR-0366: Localized wallet-provider error redaction

- Status: Accepted
- Date: 2026-10-05
- Deciders: Principal UI Architecture, Security, and Wallet Integration
- Scope: Browser wallet connection and account-selection presentation

## Context

Injected wallets are external providers. Their rejected promises can contain
provider implementation details, browser-extension diagnostics, account
metadata, or uncontrolled wording. Rendering those exception strings in a
financial authorization flow creates an inconsistent operator experience and
can leak information that was never approved as user-facing evidence.

The wallet provider remains the authority for connection approval. This
decision must not convert a rejection into a connection, retry it, or change
the server-compiled transaction and signing boundaries.

## Decision

The shared wallet picker renders the localized `connectionRejected` message
for every provider exception except the typed, expected
`WalletAccountSelectionRequired` condition. That condition continues to show
the explicit account chooser and its bounded explanation. Provider exception
details remain out of the primary DOM; support telemetry and provider-specific
diagnostics require a separate governed contract.

## Alternatives considered

| Alternative | Operator clarity | Information safety | Wallet authority | Decision |
| --- | --- | --- | --- | --- |
| Render provider exception text directly | Inconsistent | Weak | Pass | Rejected |
| Retry or classify provider errors in the browser | Partial | Partial | Risk of changing intent | Rejected |
| **Map exceptions to localized rejection copy; preserve typed account selection** | **Pass** | **Pass** | **Pass** | **Selected** |
| Add a technical diagnostic drawer | Partial | Requires a support-evidence contract | Pass | Deferred |

## Invariants and failure policy

- A provider exception never establishes a connected wallet or authorizes a
  transaction.
- A provider exception is not retried automatically.
- Account-selection-required remains a typed, bounded recovery path and does
  not expose arbitrary provider exception text.
- The selected wallet, account, transaction bytes, and server authorization
  state are unchanged by presentation mapping.
- English, German, Arabic RTL, and Japanese use the versioned wallet catalog.

## Verification

- Wallet component tests assert provider exception redaction and preserve the
  multi-account selection flow.
- The real-browser wallet-rejection journey asserts localized safe copy, no
  raw provider error, no connected account, and axe cleanliness.
- The wallet boundary and full web/repository checks remain required gates.

