# ADR-0280: Browser workspaces never render bearer credentials

## Status

Accepted — implemented and locally verified; staging identity-provider evidence
remains open.

## Context

The React shell keeps the access token in memory and passes it to protected API
clients. One invoice-quote workspace was also rendering that token in a
read-only textarea so the workspace could describe its authentication state.
This added no authorization value and exposed a bearer credential to screen
readers, screenshots, browser extensions, support captures, and accidental
copy/paste. A token is an execution credential, not product evidence.

The same UI must support local-only credential injection for Docker development
when explicitly configured, while staging and production use the OIDC session
boundary and never expose the token value in a domain workspace.

## Decision

Domain workspaces receive an opaque access-token string only as an in-memory
dependency for authenticated requests. They may render a bounded session
status, such as “Signed-in session ready” or “Sign-in required”, but must never
render, label, serialize, copy, log, or place the credential in an input value.

The shared authentication control remains the only local-development surface
that may accept a manually supplied credential, and its configuration is
local-only. OIDC deployments show sign-in, signed-in, expired, and signed-out
states without displaying the token.

Simulation and preview actions use explicit operator language such as “Run
controlled simulation”; they must not be presented as examples or as evidence
of a live settlement.

## Invariants and failure behavior

- A protected request may include the in-memory bearer token in its HTTP
  authorization header, but rendered DOM text and input values must not contain
  the token.
- An empty or expired session renders an honest sign-in-required state and
  keeps the protected action unavailable.
- Session status never authorizes an operation; the API and capability snapshot
  remain authoritative.
- Local credential entry is unavailable when the build is configured for OIDC
  or an unconfigured deployment.
- Tests must assert both the positive session-status copy and the absence of a
  known token value from rendered output.

## Alternatives considered

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Render the token in a read-only field | Makes the request dependency visible | Credential disclosure, accidental copy, screenshot leakage, and no authorization benefit | Rejected |
| Mask the token in a password field | Reduces casual disclosure | Still places a bearer credential in the DOM and teaches users to handle it as a form value | Rejected |
| Pass only a boolean session flag to the workspace | Strongest presentation boundary | Requires a separate authenticated request-client contract throughout the current slice | Deferred; the current token remains memory-only and is never rendered |
| **Pass the token in memory and render only bounded session status** | Small contract change, preserves typed API clients, removes disclosure, and supports honest failure states | Requires every workspace to preserve the boundary | **Selected** |

## Verification

- Document quote and operational-workspace tests prove a signed session renders
  status only and that the token is absent from the domain-workspace DOM. The
  shared authentication control is the sole credential-entry surface.
- Full React typecheck, accessibility tests, browser journeys, and the UI
  language contract remain required.
- Staging evidence must prove OIDC redirect, expiry, logout, revocation, and
  provider outage behavior without exposing credentials in support artifacts.
