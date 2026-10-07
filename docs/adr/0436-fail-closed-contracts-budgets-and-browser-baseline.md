# ADR-0436 — Fail-closed contracts, stylesheet budget, and browser baseline

- Status: Accepted
- Date: 2026-10-07
- Decision owners: Principal UI Architecture, Security

## Context

A review of the web app's repository gates found three architecture gaps:

1. `scripts/verify_non_custodial_wallet_contract.sh` scanned browser and API
   source for user-key material with ripgrep behind `|| true`. Without `rg`
   installed (a normal CI or developer machine) "command not found" became "no
   findings", so a security contract printed "passed" without scanning. That
   violates the repository's fail-closed principle.
2. The one stylesheet every route downloads had no size budget, so it could grow
   unnoticed.
3. The stylesheet depends on modern CSS (`color-mix()`, container queries, `:is()`,
   `linear()` easing) but no supported-browser baseline was stated or enforced.

## Decision

1. **Fail closed.** The wallet contract scans with POSIX `grep` and fails on any
   scanner error, including an error reported only on stderr (BSD `grep` can exit 0
   or 1 while reporting an unreadable file). Because the scan runs in a command
   substitution (a subshell), it reports errors through its output and the caller
   fails. `scripts/verify_non_custodial_wallet_contract_test.sh` proves it: clean
   tree passes, injected keypair/mnemonic/secret-key material fails, test files are
   exempt, a scanner error fails, and no ripgrep dependency exists. Both scripts run
   from `src/nonCustodialContract.test.ts`, so `pnpm test` and CI exercise them.
2. **Budget the stylesheet.** `src/stylesheetBudget.test.ts` caps the source at
   155 KB, 26 KB gzipped, and 950 rule blocks (measured: 123 KB, 17.9 KB, 736), and
   forbids `@import` and remote `url()`. Raising a ceiling needs an ADR.
3. **State the browser baseline.** Chrome and Edge 113+, Firefox 113+, Safari
   17.2+, set as Vite `cssTarget` so the minifier preserves the modern features
   (verified present in the shipped CSS). `src/browserBaseline.test.ts` rejects
   features newer than the baseline. In an older browser a declaration using an
   unsupported feature is dropped (for example no spring easing), which degrades
   motion and tint, not function.

## Findings routed elsewhere

`pnpm audit` reports six pre-existing transitive vulnerabilities (two high:
`bigint-buffer` through the Solana libraries, with no patched version, and
`source-map-js` through the Vite toolchain). None come from the UI work, and the
Material colour library added in ADR-0422 is a dev-only dependency absent from
the shipped bundle. Resolving or time-boxing them is a separate dependency
decision and was handed off as its own task.

## Evidence

- Wallet contract: 7 regression cases pass; it passes on the real tree without
  `rg`; before the fix it printed "command not found" twice and still passed.
- Budget and baseline tests pass; the build is unchanged (about 105 kB, 13.7 kB
  gzipped).
- 423 unit tests and the 4 web contracts pass.

## Limits

- The browser baseline is enforced against the stylesheet and build target, not by
  running older browsers.
- The key-material scan is a pattern check, not proof that no key handling exists;
  it complements the non-custodial wallet port and review.
