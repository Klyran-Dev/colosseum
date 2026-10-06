# ADR-0339: Polite workspace navigation announcement

- Status: Accepted
- Date: 2026-10-05
- Deciders: Principal UI Architecture and Principal Low-Latency Blockchain Architecture
- Scope: React SPA shell accessibility boundary

## Context

The shell changes the visible workspace heading and moves keyboard focus after
navigation, but a screen-reader user also needs a concise confirmation that
the requested work area has changed. Repeating the full page description,
authorization state, or technical evidence in a live region would create
noise and could imply that a workspace is ready before its lazy content has
loaded.

## Decision

Render one visually hidden `role="status"` live region in the product shell.
Its text is the current plain-language workspace label followed by
“workspace”. The region uses polite, atomic updates and contains no tenant
identity, authorization decision, financial value, tax result, wallet state,
or technical identifier. Existing heading/focus behavior remains the primary
visual and keyboard navigation contract.

## Alternatives considered

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Rely on the heading and focus move only | No extra markup | Some screen readers do not announce the changed context consistently | Rejected |
| Assertive live region with full workspace state | Immediate notification | Interrupts user work and repeats sensitive/technical context | Rejected |
| Browser title mutation | Broad platform support | Global title changes are noisy and not scoped to the application landmark | Rejected |
| **Polite bounded workspace status** | Concise context, non-interruptive, no authority or financial data | Requires one stable shell status node | **Selected** |

## Invariants and failure behavior

- The announcement is presentation-only and cannot authorize, prepare, sign,
  submit, or imply settlement.
- Its content is derived only from the server-independent navigation label
  already selected by the shell.
- It is bounded to the current workspace label and never includes raw API
  errors, credentials, tenant IDs, amounts, signatures, or digests.
- If live-region support is unavailable, visible heading and keyboard focus
  behavior remain unchanged.

## Latency and resource budget

- One static DOM node and one text update per workspace transition.
- No network request, timer, subscription, dependency, or financial-path work.
- Zero impact on quote, tax, authorization, wallet, settlement, and indexer
  paths.

## Verification

- Add a failing-first React shell test for the bounded status text.
- Add a real-browser assertion that the status remains present during a direct
  workspace route and normal navigation.
- Re-run the complete web suite, browser axe checks, and public documentation
  contract.

## References

- [ADR-0142: SPA workspace focus management](0142-spa-workspace-focus-management.md)
- [ADR-0337: Single-open Material 3 navigation group](0337-single-open-material-navigation-group.md)
