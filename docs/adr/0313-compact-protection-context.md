# ADR-0313: Compact protection context in the operations shell

## Status

Accepted — implemented in the React/Vite shell; manual operator and
assistive-technology review remain release gates.

## Context

The shell repeated the same protection facts in a top-bar status disclosure,
the account menu, a page access card, a four-item security strip, and a sidebar
identity block. Repetition increased scan cost and made static assurance copy
look like a live authorization result. The product needs to communicate its
security posture without competing with the workspace task.

The shell must preserve the distinction between presentation and authority:
the page cannot grant access, approve a payment, or prove wallet custody. The
server remains authoritative for RBAC/PBAC/ABAC decisions and every protected
request repeats authorization.

## Decision

Remove the repeated full-width security strip. Keep the current access state in
the page-context card and place the four explanatory assurances behind one
closed, keyboard-operable **Protection details** disclosure:

- account access and the current server/session state;
- decision records are saved automatically;
- sensitive actions may require approval;
- wallets remain with their owners.

Runtime, network, service, and session diagnostics remain in the separate
system-status disclosure. Account identity and sign-out remain in the account
menu. This gives each fact one primary home while preserving accessible text,
keyboard access, and the existing authorization semantics.

The disclosure is presentation-only. It does not change capability snapshots,
tenant scope, server authorization, wallet artifacts, tax calculations,
settlement state, or failure handling.

## Alternatives considered

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Keep every repeated status surface | Maximum redundancy | Clutter, weak hierarchy, and ambiguous live-vs-static meaning | Rejected |
| Remove all protection explanation | Cleanest shell | Hides important responsibility and custody expectations | Rejected |
| Put all protection facts in the account menu | Fewer visible elements | Couples account identity with workspace safety and harms discoverability | Rejected |
| **Keep current access summary with one expandable protection disclosure** | One primary location, progressive disclosure, accessible and auditable | Requires browser/component regression coverage | **Selected** |

## Invariants and failure policy

- The closed state shows the current access result without claiming authority.
- Opening the disclosure exposes all protection assurances as text, not color
  alone, and works with keyboard input and screen readers.
- System diagnostics remain separate and retain their explicit failure states.
- A missing or stale server authorization result remains unavailable or
  indeterminate; the disclosure never upgrades it to allowed.
- No retired `.security-strip` selector or repeated shell strip may return
  without a new ADR and alternative analysis.

## Verification

- `apps/web/src/productShell-ui.test.tsx` proves one closed disclosure,
  complete assurance copy, and no repeated security strip.
- `apps/web/e2e/ui-journeys.spec.ts` proves keyboard opening, rendered text,
  absence of the retired strip, and browser accessibility checks.
- `scripts/verify_ui_language_contract.sh` guards the approved shell copy and
  rejects the retired selector.
- `make web-check`, `make web-browser-check`, and the full repository check are
  required before release evidence can claim this shell behavior.

## References

- [ADR-0298: Material 3 Expressive visual system](0298-material-3-expressive-visual-system.md)
- [ADR-0309: Contextual status disclosure](0309-contextual-status-disclosure.md)
- [ADR-0312: Outcome-first document payment language](0312-document-payment-outcome-first-language.md)
