# ADR-0512: Metronic typography hierarchy

- Status: Accepted
- Date: 2026-10-11
- Decision owners: Product UI and frontend engineering

## Context

The licensed Metronic Demo 3 composition was present, but the application
type scale was inconsistent: workspace headings used display-sized text,
supporting labels dropped below a comfortable reading size, and the branded
authentication panel could allow its context kicker to sit beside the brand on
narrow screens. This increased scanning effort and made the shell feel less
like the compact Metronic product surface.

## Decision

Keep the existing Inter-compatible system stack and Material semantic type
roles, but tune the product hierarchy for operational density:

- use 12px as the smallest supporting-label size and 13px for medium labels
  and small body copy;
- use the 32px headline-large role for workspace page titles instead of the
  45px display-medium role;
- use 16px body copy for page descriptions and authentication explanations;
- keep heading line-height above 1.1 for readable multi-line wrapping;
- make the authentication context content a single-column grid so the brand,
  kicker, heading, explanation, and evidence list cannot collide at narrow
  widths;
- preserve tabular numerals only where values are operational data, not as a
  general visual treatment for prose.

## Alternatives considered

1. Keep the current Material defaults and adjust individual components —
   rejected because local overrides would continue to drift.
2. Import the full Metronic CSS bundle — rejected because it would introduce
   unrelated styles, duplicate tokens, and a second color/type authority.
3. Use a remote webfont — rejected because the SPA must remain deterministic,
   privacy-preserving, and usable without an external font request.
4. **Tune the repository-owned semantic roles and composition selectors** —
   selected because it preserves the existing accessibility and stylesheet
   contracts while correcting scale and wrapping behavior.

## Invariants

- No color literals or external font imports are added to the stylesheet.
- Text remains readable at the 320px WCAG reflow target.
- Authentication labels, errors, and actions remain semantic and keyboard
  accessible.
- The stylesheet source, gzip, and rule-count budgets remain green.
- Numeric protocol values retain their existing canonical formatting.

## Verification

The shell contract test asserts the tuned type roles, page-title role,
supporting-text role, and single-column authentication context composition.
The frontend suite, typecheck, Docker browser journeys, and visual browser
inspection remain required after the stylesheet rebuild.
