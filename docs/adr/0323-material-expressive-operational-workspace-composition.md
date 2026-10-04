# ADR-0323: Material 3 Expressive operational workspace composition

- Status: Accepted
- Date: 2026-10-04
- Deciders: Principal UI Architecture and Principal Low-Latency Blockchain Architecture
- Scope: React SPA shell composition, navigation disclosure, page-heading hierarchy

## Context

The shell has a governed Solana Aurora Material 3 role palette, but the
operational workspace still presents too many surfaces with equal visual
weight. A reviewer must scan navigation, runtime status, sign-in context,
protection context, and the current task before reaching the primary work.
That is undesirable for a high-consequence financial product: visual density
must not hide a failure state, and technical context must not compete with the
decision the operator is making.

The request is to improve the UI using Material 3 Expressive composition and
role-based color, without removing any production workflow or weakening
RBAC/PBAC/ABAC disclosure. This is a presentation boundary only. Server-owned
authorization, tax results, wallet artifacts, and chain evidence remain
unchanged.

## Decision

1. Keep Solana Aurora as the only palette. Components consume semantic M3
   roles; they may not add raw colors, gradients, or state colors.
2. Make the current workspace the visual anchor: one page heading, one
   concise access context, and workflow content below it.
3. Render navigation as accessible native disclosure groups. Daily work is
   open by default; lower-frequency groups are collapsed by default while all
   routes remain discoverable and keyboard reachable when opened.
4. Keep authorization and risk semantics available to assistive technology,
   tooltips, and the Access & approvals workspace, but do not repeat a status
   word, risk dot, and explanation beside every navigation item.
5. Keep runtime health, display preferences, and session controls behind
   explicit top-bar controls. A collapsed control must not claim that the
   underlying service, account, or authorization state is healthy.
6. Use expressive hierarchy through tonal surfaces, shape, typography,
   spacing, and restrained state emphasis. No decorative gradients, glass
   layers, or animation is introduced.

## Alternatives considered

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Keep every workspace expanded | Maximum immediate visibility | High scan cost; all workflows appear equally urgent | Rejected |
| Hide lower-frequency workspaces behind a search-only command menu | Very clean first view | Poor discoverability, keyboard friction, and route ambiguity | Rejected |
| Use colored navigation badges for authorization and risk | Fast visual signaling | Repeats state, consumes palette roles, and increases false urgency | Rejected |
| Replace the shell with a dashboard of metrics | Familiar SaaS pattern | Makes evidence and action flows secondary; encourages invented summary data | Rejected |
| **Native grouped disclosure plus one active-workspace hierarchy** | Preserves routes, reduces scan cost, accessible, deterministic, low runtime cost | Adds one interaction for lower-frequency groups | **Selected** |

## Invariants and failure policy

- The current workspace remains visible and marked with `aria-current="page"`.
- Every navigation route remains present in the DOM and reachable from its
  group; disclosure changes presentation only and never policy.
- Authorization decisions remain server-owned and fail closed. A collapsed
  group cannot be interpreted as denied, unavailable, or approved.
- Theme selection changes presentation only and cannot affect any API request,
  tax calculation, wallet transaction, or settlement state.
- The palette and accessibility contracts remain mandatory for all themes.
- Reduced-motion users receive the same structure and state meaning.

## Verification

- Add a failing UI contract for grouped disclosure defaults, stable route
  labels, and current-workspace marking before implementation.
- Run the React unit/UI suite, typecheck, static palette/color contracts,
  accessibility assertions, and real-browser release journey.
- Manually review dark, light, system, narrow, wide, RTL, keyboard-only, and
  reduced-motion presentation before release evidence is accepted.

## References

- [ADR-0298: Material 3 Expressive visual system](0298-material-3-expressive-visual-system.md)
- [ADR-0311: Solana Aurora Material 3 role palette](0311-solana-aurora-material-palette.md)
- [ADR-0317: Material 3 Expressive surface hierarchy and navigation density](0317-material-expressive-surface-hierarchy-and-navigation-density.md)
- [ADR-0306: Collapsed advanced evidence disclosures](0306-collapsed-advanced-evidence-disclosures.md)
