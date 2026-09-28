# ADR-0148: Reproducible real-browser UI journey contract

- Status: Accepted
- Date: 2026-09-28
- Owners: Principal Low-Latency Architecture, Web Platform, SRE
- Bounded contexts: Operator UI, Checkout UI, Edge Delivery, Developer Experience

## Context

The rendered `axe-core` contract in ADR-0111 runs quickly in `happy-dom`, but
it cannot prove browser focus behavior, code-split chunk recovery, responsive
layout, reduced-motion behavior, or the browser's computed direction and
overflow characteristics. Those gaps are material for a wallet-facing,
operator-facing product whose UI must not misrepresent tax or settlement state.

The test must exercise the built static SPA through the same NGINX edge used by
the local product topology. It must not create a second application runtime,
inject a financial success state, or make browser binaries part of the
production web image.

## Decision

Add a separate Docker Compose `browser` profile with a `web-e2e` service:

- use `@playwright/test` 1.63.0 as a development-only test dependency;
- use the official Playwright `v1.63.0-noble` multi-architecture image pinned
  by its immutable OCI index digest;
- build the test runner separately from the NGINX runtime image and run it
  against `http://edge:8080` over the Compose network;
- cover desktop and narrow mobile viewports, keyboard skip/focus behavior,
  display-settings RTL/reduced-motion behavior, every lazy workspace's real
  chunk load, and a bounded dynamic-import failure that must render the
  redacted ADR-0147 recovery state;
- keep API mutation and wallet-signing journeys outside this deterministic UI
  contract unless their server-owned fixtures and authority boundaries are
  explicitly added in a later ADR;
- retain trace, screenshot, and video output only on failure, under the
  ignored `test-results/web-e2e` directory; and
- expose the test through `make web-browser-check` and a dedicated CI job.

The edge remains the only application entry point. Browser route interception
is limited to failure injection for the dynamic-import test; ordinary page
loads use the built edge and its real health endpoint. No intercepted response
may claim a quote, attestation, wallet signature, submission, confirmation,
finality, or reconciliation result.

## Invariants and budgets

- The browser test never receives or stores a private key, seed phrase, real
  tax evidence, or production credential.
- A failed lazy chunk produces an accessible alert and reload action without
  exposing the exception or implying a financial outcome.
- Keyboard navigation reaches the skip link and the committed workspace
  landmark; navigation changes move focus to that landmark.
- At the narrow viewport, the document has no horizontal overflow and the
  selected RTL direction is reflected in the document and shell.
- The test runner has a 30-second per-test timeout and a 5-minute Compose CI
  job budget; it must not retry failed tests automatically.
- The browser image is isolated to the test profile and is not inherited by
  production images.

## Alternatives considered

| Alternative | Decision | Reason |
| --- | --- | --- |
| Keep only `happy-dom` plus axe-core | Rejected as the complete contract | Fast and deterministic, but cannot prove real browser layout, focus, or chunk behavior. Retained as the per-change baseline. |
| Install a browser on the developer/CI host | Rejected | Host state and browser drift weaken reproducibility and violate the clean-checkout Docker contract. |
| Add Playwright and browsers to the NGINX image | Rejected | Inflates the attack surface and couples test tooling to the immutable production edge. |
| Use an unpinned Playwright image or browser download | Rejected | Mutable browser binaries make failures and security evidence non-reproducible. |
| **Pinned Playwright runner in a Compose browser profile** | **Selected** | Real browser behavior, isolated dependencies, reproducible assets, clean-network edge testing, and explicit failure artifacts. |

## Verification and release impact

`make web-browser-check` must pass on a clean Docker environment. CI retains
the failure artifacts and publishes the test result as a separate browser
journey gate. This closes the deterministic real-browser journey contract but
does not, by itself, prove screen-reader quality, legal/tax correctness, live
RPC behavior, or general WCAG 2.2 AA conformance; manual assistive-technology
review and live-provider release gates remain required.
