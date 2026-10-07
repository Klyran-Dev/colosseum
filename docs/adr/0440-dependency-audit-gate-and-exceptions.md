# ADR-0440 — Dependency audit gate and time-bounded exceptions

- Status: Accepted. One recorded exception is **proposed and awaiting approval** by the
  accountable owner (see below).
- Date: 2026-10-07
- Decision owners: Security, Principal UI Architecture

## Context

`AGENTS.md` requires CI to fail on critical or high dependency findings unless an
approved, time-bounded exception exists. `pnpm audit` for `apps/web` reported six
findings (two high) and nothing enforced or recorded the policy. All six were
transitive: `bigint-buffer` through `@solana/spl-token`, a devDependency; `uuid` and
`stream-json` through `jayson`, the Node RPC server library; `source-map-js` through the
Vite/PostCSS toolchain. A build and bundle inspection found none of these packages in
the shipped browser bundle.

## Decision

1. **Patch what has a safe patch.** Pinned overrides in `apps/web/pnpm-workspace.yaml`:
   `source-map-js` 1.2.2 (clears the high build-tool finding) and `uuid` 11.1.1. Type
   check, 425 unit tests, the build, and the real-browser gate pass with them.
2. **Do not force what cannot be validated.** `stream-json` 3.x is a major jump in a
   Node-only server library not used by the browser; forcing it would be untestable
   here. Its three moderate findings stay reported, not excepted (the policy only
   mandates failing on high and critical).
3. **Gate the policy.** `scripts/verify_web_audit_contract.mjs` fails closed on an
   unreadable audit result, a high/critical finding without an exception, an expired,
   incomplete, or over-long (more than 180 days) exception, a stale exception (the
   advisory no longer applies), an exception that does not state it is absent from the
   shipped bundle, an excepted package found in `dist`, and an exception without a named
   `approvedBy`. `--allow-pending` reports an unapproved proposal as a warning.
   `src/auditContract.test.ts` covers each case with synthetic audit data.
4. **Record the remaining high finding as a proposal.** `bigint-buffer`
   (GHSA-3gc7-fjrx-p6mg, no patched release) is reachable only through the
   `@solana/spl-token` devDependency used by localnet tests. The exception in
   `docs/security/dependency-exceptions.json` expires 2027-01-05 and has an empty
   `approvedBy`: **approval is a decision for the accountable owner, not for the
   author of the change**, so the strict gate stays red until they set it.

## Evidence

- Audit findings 6 to 4 (one high, three moderate); no affected package in the shipped
  bundle; 425 unit tests, build, and 78 real-browser tests pass with the overrides.
- 9 gate tests pass; against the real audit the gate reports the proposal and passes with
  `--allow-pending`, and fails without it.

## Limits

- `pnpm audit` is a point-in-time registry check and needs network access; the gate
  fails closed when it cannot run.
- The gate is not yet wired into `make check` or CI, because the Makefile and CI files
  carry unreviewed local changes.
- Absence from `dist` is checked by package-name markers; it is evidence, not proof.
