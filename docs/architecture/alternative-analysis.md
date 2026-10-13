# Architecture alternative analysis

This document records the comparison baseline required before implementation. It is intentionally explicit: “latest” or “fast” is not an evaluation criterion without a workload, support horizon, security model, and operational cost.

## Evaluation criteria

- deterministic financial correctness;
- Solana and wallet compatibility;
- p50/p95/p99 latency and tail behavior;
- throughput and contention under concurrent merchants;
- auditability and replayability;
- security and key isolation;
- developer correctness and testability;
- operational complexity and failure recovery;
- accessibility and product-quality UX;
- support lifecycle and supply-chain reproducibility.

### Tax-rule authoring density (ADR-0448)

| Option | Result |
| --- | --- |
| Permanently expanded component forms | Rejected: expert controls overwhelm the review surface and increase component-selection errors. |
| Separate simplified editor | Rejected: it would hide or split canonical tax semantics. |
| Separate wizard step per component | Rejected: cross-component dependencies become harder to inspect. |
| **Collapsible component sections with localized summaries** | **Selected: preserves the complete deterministic model while reducing cognitive load and keeping review in one surface.** |

### Document-line authoring density (ADR-0449)

| Option | Result |
| --- | --- |
| Permanently expanded line forms | Rejected: repeated controls overwhelm multi-line review. |
| Opaque JSON line editor | Rejected: weakens field validation, localization, and accessibility. |
| Separate wizard step per line | Rejected: obscures document-level relationships and totals. |
| **Collapsible line sections with localized summaries** | **Selected: preserves field-level correctness while keeping multi-line documents scannable.** |

### Signed-out authentication affordance (ADR-0450)

| Option | Result |
| --- | --- |
| Keep email sign-in and a read-only account menu | Rejected: duplicates the next action and adds a competing popover. |
| Make the account icon start sign-in | Rejected: hides the email-OTP method and weakens discoverability. |
| **Keep email sign-in as the sole signed-out action; show account menu only for sessions** | **Selected: clear next step, fewer mobile controls, and a verified account menu when identity exists.** |

### Single global signed-out sign-in entry point (ADR-0459)

| Option | Discoverability | Visual hierarchy | Narrow-layout clarity | Decision |
| --- | --- | --- | --- | --- |
| Keep a header action and a workspace-gate action | Good | Weak: duplicate transition | Weak: action competition | Rejected |
| Remove the header action | Weak | Partial | Partial | Rejected |
| **Keep the header action; make the gate explanatory only** | **Good** | **Strong** | **Strong** | **Selected** |

Chosen: retain one global signed-out transition while keeping the protected
workspace gate focused on status, reason, and next-step guidance.

### Auth-service browser origin policy (ADR-0460)

| Option | Local usability | Trust boundary | Configuration determinism | Decision |
| --- | --- | --- | --- | --- |
| One fixed loopback origin | Partial | Strong | Strong | Rejected |
| Wildcard origin | Good | Weak | Strong | Rejected |
| Reflect any request origin | Good | Weak | Weak | Rejected |
| **Explicit exact-origin allow-list** | **Good** | **Strong** | **Strong** | **Selected** |

Chosen: support the documented local loopback forms while requiring every
deployment to declare the origins that may call the authentication service.

### Stable authorization context visibility (ADR-0461)

| Option | Stable-task focus | Degraded-state clarity | Information discoverability | Decision |
| --- | --- | --- | --- | --- |
| Show the access card on every workspace | Weak | Strong | Strong | Rejected |
| Remove access context everywhere | Strong | Weak | Weak | Rejected |
| Keep a compact card on every workspace | Partial | Strong | Strong | Rejected |
| **Show the card only for public or uncertain states** | **Strong** | **Strong** | **Strong via account menu** | **Selected** |

Chosen: preserve access explanations when they carry decision-making value and
remove only the stable authenticated repetition.

### Workflow density follows panel width (ADR-0462)

| Criterion | Viewport-only breakpoints | Always one form column | **Panel-width container query** |
| --- | --- | --- | --- |
| Narrow calculator readability | Partial | **Good** | **Good** |
| Wide desktop scanability | Good | Poor | **Good** |
| Collapsed-rail / variable shell behavior | Poor | Good | **Good** |
| Empty result-space control | None | None | **Good** |
| **Selection** | Rejected | Rejected | **Selected** |

Chosen: size field density from the calculator panel's available width and
start-align an empty result surface so the task remains readable at laptop
widths without sacrificing wide-desktop efficiency.

### Authenticated session control ownership (ADR-0463)

| Criterion | Duplicate direct action | Account menu only | **Passive status plus labelled account-menu action** |
| --- | --- | --- | --- |
| Session-action discoverability | Good | Partial | **Good** |
| Account/access context | Partial | **Strong** | **Strong** |
| Topbar density | Poor | Good | **Good** |
| Keyboard/screen-reader clarity | Partial | Good | **Strong** |
| **Selection** | Rejected | Rejected | **Selected** |

Chosen: keep the status visible as a passive signal and make the account menu
the single owner of identity and session actions.

### Wallet-step panel-width reflow (ADR-0464)

| Criterion | Shorter copy | Always vertical | **Panel-width two-row reflow** |
| --- | --- | --- | --- |
| No-wallet readability | Partial | **Good** | **Good** |
| Wide-desktop density | Good | Poor | **Good** |
| Wallet-control usability | Partial | Good | **Strong** |
| Localization resilience | Poor | Good | **Strong** |
| **Selection** | Rejected | Rejected | **Selected** |

Chosen: reserve the narrow panel's width for readable prerequisite copy and a
usable wallet control, while keeping the compact step on wide panels.

### Mode-specific tax result language (ADR-0451)

| Option | Result |
| --- | --- |
| Reuse quote language in the simulator | Rejected: falsely implies approved-rule authority and advertises an unavailable quote action. |
| Duplicate the result component for each mode | Rejected: creates financial presentation drift and doubles accessibility maintenance. |
| **Shared renderer with typed localized mode copy** | **Selected: preserves structural consistency while accurately communicating authority and next actions.** |

### Quiet tonal Material 3 Expressive shell (ADR-0408)

| Alternative | Hierarchy clarity | Palette auditability | Accessibility/operations | Decision |
| --- | --- | --- | --- | --- |
| Preserve stacked historical shell layers | Weak; every state competes for attention | Weak; precedence is difficult to reason about | Partial; diagnostics are always visually loud | Rejected |
| Assign a unique accent per bounded context | Weak; color becomes a second navigation taxonomy | Weak; context colors drift into state colors | Partial; more hues increase cognitive load | Rejected |
| Use a flat neutral shell | Pass for calmness | Pass | Weak; selection, risk, and primary actions lose emphasis | Rejected |
| **Use M3 semantic roles with quiet tonal layering and progressive disclosure** | **Pass** | **Pass** | **Pass** | **Selected; see ADR-0408** |

### Explicit fee-payer binding (ADR-0359)

| Alternative | Authority clarity | Regression resistance | Hot-path cost | Decision |
| --- | --- | --- | --- | --- |
| Rely on indirect account-count and privilege rejection | Partial | Weak | Low | Rejected as the sole contract |
| Permit sponsored fee payers in the existing artifact | Ambiguous | Weak | Low | Rejected |
| Reconstruct or re-sign at the API | Fails non-custody | Partial | Higher | Rejected |
| **Compare the first compiled account with the signed message fee payer** | **Pass** | **Pass** | **Bounded in-memory check** | **Selected; see ADR-0359** |

### Document-simulation localization (ADR-0360)

| Alternative | Operator clarity | Protocol/value safety | Determinism | Decision |
| --- | --- | --- | --- | --- |
| Leave the subsection in English | Fails multilingual workflow consistency | Pass | Pass | Rejected |
| Runtime machine translation | Partial | Partial | Fail | Rejected |
| Translate protocol and server-owned values | Partial | Fail; breaks audit/replay identity | Fail | Rejected |
| **Typed document-simulation presentation catalog** | **Pass** | **Pass** | **Pass** | **Selected; see ADR-0360** |

## Decisions

### Tenant role-control-plane summary (ADR-0506)

| Alternative | Scanability | Authority boundary | Operational cost | Decision |
| --- | --- | --- | --- | --- |
| Lead with detailed people and invitation tables | Weak | Pass | Pass | Rejected |
| Client-only optimistic dashboard | Strong | Fail | Pass | Rejected |
| Separate aggregate endpoint | Strong | Pass | Partial; adds another bounded contract | Rejected |
| **Summary derived from the existing validated projections** | **Strong** | **Pass** | **Pass** | **Selected** |

The selected summary improves the first-read experience of the Metronic
administrator surface without creating a new source of truth or a browser
authorization model. Unknown data remains visibly unknown while projections
load or fail.

### Responsive product navigation

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Icon-only mobile strip | Compact and familiar to expert console users | Hides task meaning and weakens keyboard/screen-reader discovery | Rejected |
| Fixed bottom navigation | Familiar mobile placement | Cannot represent the full operational task catalog and blurs high-risk actions | Rejected |
| Full-screen route menu | Maximum room for copy | Disruptive and removes current-work context | Rejected |
| **Labeled disclosure in the existing shell** | Complete task discovery, native semantics, preserved context, one navigation model | Adds responsive disclosure state | **Selected in ADR-0265** |

### Quote-to-payment workspace handoff

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Clear all state on navigation | Simple lifecycle | Breaks the primary quote-to-payment journey | Rejected |
| Retain all workspace results | Smooth switching | Allows stale cross-context data and unbounded browser state | Rejected |
| Put the quote in the URL | Reloadable | Exposes financial context and adds a second transport contract | Rejected |
| **Retain only the bounded quote/payment cluster** | Preserves context while keeping authority and state scope explicit | Requires transition tests | **Selected in ADR-0266** |

### Product-shell account and deployment context

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Fixed tenant/operator/environment/network labels | Simple and visually complete | Can falsely describe the account, deployment, or Solana cluster | Rejected |
| Browser storage, URL, or caller-supplied context | Easy to customize | Untrusted display data can be mistaken for authority and can leak between accounts | Rejected |
| Infer deployment from RPC URL or hostname | No extra build setting | Proxies, failover, and local fixtures make inference unreliable | Rejected |
| Runtime context endpoint | Server-owned and changeable | Adds a request and availability dependency to the static shell | Deferred until a signed deployment-manifest contract exists |
| **Explicit immutable build context plus server-owned account snapshot** | Fast, deterministic, honest on configuration failure, and compatible with NGINX | Release builds must pass and record context values | **Selected; ADR-0267** |

### Product-shell status density

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Keep deployment, network, service, and sign-in warnings persistent | Immediate visibility | Repetition makes the primary operator task harder to scan and visually overweights failures | Rejected |
| Hide degraded state behind a generic error page | Minimal chrome | Conceals the exact support and recovery context | Rejected |
| **One system-status disclosure with separate session disclosure** | Preserves every diagnostic, improves hierarchy, native accessibility, and no hot-path cost | Requires an intentional inspection action | **Selected; ADR-0309** |

### Material shell landmark and role boundary (ADR-0398)

| Alternative | Accessibility and hierarchy | Palette reproducibility | Decision |
| --- | --- | --- | --- |
| Retain the stale `workspace-content` skip target | Fails when the target is absent and weakens keyboard bypass | Pass | Rejected |
| Remove the skip link | Removes a required bypass around persistent navigation | Pass | Rejected |
| Keep multiple component-local color layers | Hard to audit across themes and states | Weak | Rejected |
| **Point the skip link at the stable main landmark and apply one semantic role boundary** | **Deterministic keyboard entry, explicit surface hierarchy, and auditable state pairing** | **Pass** | **Selected; ADR-0398** |

### Versioned navigation access language (ADR-0399)

| Alternative | Locale consistency | Authority/evidence safety | Decision |
| --- | --- | --- | --- |
| Keep English access labels in every shell | Fails multilingual UX | Pass | Rejected |
| Translate server-owned decision reasons in the browser | Partial | Fails exact authorization evidence semantics | Rejected |
| Remove access status from navigation descriptions | Consistent but hides the reason a protected workspace is unavailable | Weak | Rejected |
| **Catalog only presentation labels and preserve server-owned reasons exactly** | **Pass across supported locales** | **Pass; no authority or evidence mutation** | **Selected; ADR-0399** |

### Browser authentication and session handling

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Paste a bearer token into each workspace | Compatible with synthetic fixtures | Unsafe product ergonomics, duplicated credentials, and no real sign-in flow | Rejected for non-local builds |
| OIDC implicit flow | Minimal redirect implementation | Front-channel tokens and weaker replay protection | Rejected |
| **OIDC Authorization Code + PKCE public client** | Standards-based, no client secret, memory-only access token, static-SPA compatible | Requires provider redirect/CORS configuration | **Selected; ADR-0268** |
| Server-side BFF cookie session | HttpOnly session and central revocation | Adds a stateful callback service and operational surface | Deferred |
| Wallet signature as workforce identity | Native wallet support | Does not establish tenant, role, employment, or revocation authority | Rejected |

### Separate email-OTP identity issuance

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Store passwords and OTPs inside the payment API | Few services; direct UI integration | Expands the settlement API trust boundary, increases credential/PII blast radius, couples identity outages to payment code, and complicates independent rotation | Rejected |
| External OIDC provider only | Mature signup, recovery, email delivery, MFA, abuse controls, and identity governance | Requires provider selection, redirect/CORS setup, claim mapping, and external dependency evidence | Retained as the production target and fallback |
| **Separate Rust email-OTP issuer with OIDC-compatible EdDSA tokens** | Isolates identity authority, reuses the existing API verifier and SPA session boundary, supports local deterministic tests, and keeps signing/email ports replaceable | Requires durable challenge state, provider account/DNS operations, key ceremony, privacy policy, and tenant provisioning decisions | **Selected; local and provider-backed slices are implemented, deployment enablement remains gated** |
| Browser-only email OTP with a custom bearer token | Fastest apparent demo | Weakens standard discovery/PKCE interoperability and encourages client-owned authority | Rejected |
| Wallet signature as account signup | No email dependency; native Solana UX | Proves key control, not email ownership, organizational identity, roles, tenant membership, or recovery | Rejected for account authentication |

### Production transactional email delivery (ADR-0496)

| Option | Provider compatibility | Secret/transport surface | Delivery idempotency | Decision |
| --- | --- | --- | --- | --- |
| SMTP from the auth service | Broad | Larger TLS, connection, and retry surface | Service-defined | Rejected for the first adapter |
| **HTTPS Resend API adapter** | Resend account and verified sender | Bounded HTTPS JSON request with secret-manager API key | Provider idempotency key bound to challenge/invitation ID | **Selected** |
| Provider SDK | One vendor | SDK/runtime abstraction and dependency surface | SDK/provider-defined | Rejected in favor of direct documented HTTP |
| Browser/provider-direct delivery | Broad | Client-controlled credentials and authority | Unsafe | Rejected |

Chosen: keep the local inbox deterministic for Docker while making staging and
production use a server-owned, bounded HTTPS provider adapter. Provider account
ownership, DNS authentication, retention, bounce handling, and on-call remain
deployment decisions.

### Optional Google Workspace SMTP delivery (ADR-0519)

| Option | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Google Workspace Gmail SMTP with an app password | Works with a Workspace mailbox; easy to bootstrap for a controlled staging deployment | App-password secret, mailbox/account policy dependency, lower sending limits, no direct provider message-id contract, and credential rotation burden | Supported as an explicit adapter, not the default |
| Google Workspace SMTP relay with source-IP authentication | Avoids storing a mailbox password; designed for application/device relay | Requires stable egress IP and Workspace administrator allowlisting; Docker/local NAT is not a production identity boundary | Supported as an explicit no-credential relay mode |
| **Resend HTTPS adapter** | Bounded HTTP contract, provider idempotency key, clean secret-manager integration, and existing production evidence | Requires separate provider account and domain verification | **Default production adapter; retained** |

Google SMTP support is intentionally opt-in through `AUTH_EMAIL_MODE=google-smtp`.
TLS, bounded timeouts, explicit authentication mode, and runtime-only secrets are
required. A Google Workspace mailbox does not by itself authorize application
SMTP, and local Docker is not allowed to pretend its dynamic egress address is
an approved relay identity.

### Hackathon public deployment topology (ADR-0520)

| Option | Cost/latency | Operational boundary | Failure isolation | Decision |
| --- | --- | --- | --- | --- |
| GCE VM with PostgreSQL in the pinned Compose stack, fronted by Cloudflare Tunnel | Low setup cost and one local network hop; bounded by one VM | One operator-managed VM and database volume | Weak: VM and database share a failure domain | **Selected for the controlled hackathon deployment** |
| GCE application VM plus private Cloud SQL PostgreSQL | Higher monthly cost and a managed database hop | Google-managed backups, patching, monitoring, and optional HA | Stronger database isolation | Reserved for production hardening |
| Public GCE IP with direct NGINX exposure | Simple, but exposes origin and application ports | GCE firewall and NGINX become the primary perimeter | Weak against origin discovery/misconfiguration | Rejected |
| Cloud Run/Pages split | Managed edge and autoscaling | Requires a different service topology and release pipeline | Stronger service isolation | Deferred until the application is adapted and measured |

The selected topology wins for the hackathon because it preserves the tested
Docker Compose boundaries, keeps PostgreSQL and service ports loopback-only, and
uses an outbound-only Cloudflare Tunnel as the public ingress. It is explicitly
not a high-availability or mainnet-production decision; Cloud SQL migration,
private connectivity, backups/PITR, and failure evidence remain follow-up gates.

### OIDC edge connectivity and CSP

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Global `connect-src https:` | Works with any identity provider | Allows arbitrary HTTPS browser egress | Rejected |
| Keep `connect-src 'self'` | Strong static policy | Blocks a configured external identity provider | Rejected |
| Runtime browser-controlled CSP | Avoids image rebuilds | Configuration is untrusted and can drift from release evidence | Rejected |
| Same-origin BFF | Same-origin browser policy and HttpOnly session | Adds a stateful callback/session service | Deferred |
| **Build-time exact issuer-origin allow-list with same-origin metadata endpoints** | Strict, reproducible, compatible with static NGINX, and fail-closed | Provider changes require a release | **Selected; ADR-0269** |

### Production Rust panic safety

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Review checklist only | No build changes | Regressions can be introduced between reviews | Rejected |
| Text grep over all Rust sources | Easy to run | False positives in tests/benches and no Rust configuration awareness | Rejected |
| Workspace-wide deny lint with no test distinction | Strong default | Rejects legitimate fixture assertions and obscures the production boundary | Rejected |
| **Per-crate `cfg_attr(not(test), deny(clippy::expect_used, clippy::panic, clippy::unwrap_used))` plus workspace Clippy** | Enforced at compile time, no runtime cost, test fixtures remain expressive | New crate roots must copy the contract | **Selected in ADR-0212** |

### Signed-message adversarial verification

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Example-only account-meta tests | Simple and fast | Misses positional and mutation combinations | Rejected |
| Coverage-guided `cargo-fuzz` as the mandatory gate | Broad exploration | Requires a separate nightly/toolchain and reproducibility contract | Deferred |
| **Bounded stable-Rust `proptest` over real signed transactions** | Reproducible, shrinkable, CI-friendly, and tests the actual trust boundary | Not coverage-guided; scheduled fuzzing remains useful | **Selected in ADR-0110** |

### React accessibility verification

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| JSX/source convention checks | Fast and simple | Cannot inspect the rendered accessibility tree | Rejected |
| Browser Playwright audit as the only per-PR gate | Real browser and interaction coverage | Heavier startup and browser lifecycle | Deferred as a complementary journey gate |
| **Testing Library + happy-dom + axe-core rendered baseline** | Deterministic, fast, visible in Vitest CI, and validates actual DOM semantics | Does not replace real-browser/assistive-technology review | **Selected in ADR-0111** |

### Locale, time-zone, and RTL display semantics

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Component-local browser-default `Intl` formatting | Minimal code | Inconsistent financial/timestamp output and hidden time-zone assumptions | Rejected |
| Large translation framework as the first localization change | Mature catalog workflows | Adds runtime/dependency surface before exact money and evidence display semantics are established | Deferred |
| JavaScript `Number` currency conversion | Convenient currency APIs | Loses precision for large atomic values | Rejected |
| **Centralized `BigInt`/`Intl` formatter module with explicit display context** | Exact, dependency-free, testable locale/time-zone/RTL contract across lazy workspaces | Translation catalogs and real-browser review remain follow-up gates | **Selected; ADR-0146** |

### Lazy workspace failure containment

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Rely on `Suspense` fallback alone | Minimal code | Does not catch rejected imports or render-time exceptions | Rejected |
| Root-level boundary for the entire SPA | Catches everything | Hides navigation and turns one workspace defect into a full-shell outage | Rejected |
| Render raw exception details | Useful for debugging | Can disclose implementation or sensitive evidence details | Rejected |
| **One bounded redacted boundary around the selected workspace** | Preserves global shell, exposes a deterministic recovery action, and avoids retry storms | Real-browser chunk/cache evidence remains separate | **Selected; ADR-0147** |

### Real-browser UI journey evidence

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| `happy-dom` plus axe-core only | Fast and deterministic | Cannot prove browser focus, layout, responsive overflow, or code-split failure behavior | Retained as the fast baseline, rejected as the complete contract |
| Real-browser axe audit through the pinned Playwright runner | Proves rendered browser accessibility semantics at the NGINX edge | Adds browser runtime cost and still cannot certify assistive-technology quality | **Selected as a complementary automated gate in ADR-0148** |
| Host-installed browser in CI/developer machines | Low repository setup cost | Browser and OS drift weaken reproducibility and clean-checkout parity | Rejected |
| Browser binaries inside the NGINX image | One image to operate | Expands production attack surface and couples test tooling to the serving edge | Rejected |
| Unpinned browser image/download | Simple updates | Mutable binaries undermine evidence, debugging, and supply-chain review | Rejected |
| **Pinned official Playwright runner in a Docker Compose browser profile** | Real browser behavior, isolated assets, reproducible edge-network testing, and failure artifacts | Adds a heavier opt-in CI/local job; does not replace manual assistive-technology review | **Selected; ADR-0148** |

### Database pool saturation telemetry

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Query PostgreSQL for pool state | Familiar SQL observability | Consumes a scarce connection and fails during pool exhaustion | Rejected |
| Dynamic per-tenant/request labels | Detailed attribution | Unbounded cardinality and identifier leakage | Rejected |
| **Read-only deadpool status snapshot with fixed Prometheus gauges** | No DB round trip, bounded, cheap during overload | Eventually consistent snapshot | **Selected in ADR-0112** |

### Browser idempotency identity

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Require HTTPS for every local browser execution | Matches production secure-context behavior | Hides a clean-checkout HTTP failure and adds local certificate/bootstrap complexity | Rejected as the sole fix |
| Timestamp or `Math.random()` fallback | Minimal implementation | Predictable collisions and unsafe concurrent retry identity | Rejected |
| Client-generated UUIDv7 | Time ordered | Confuses client request keys with server-owned entity IDs and does not establish ordering authority | Rejected |
| **Native `randomUUID()` with CSPRNG `getRandomValues()` v4 fallback** | Secure in secure and local contexts, dependency-free, and authority-preserving | Request keys are intentionally not sortable entity IDs | **Selected; ADR-0149** |

### Evidence-volume query plans and retention

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Catalog index existence checks | Fast and simple | Does not prove planner selection at volume | Rejected |
| Force `enable_seqscan=off` | Makes index use deterministic | Manufactures evidence and hides selectivity defects | Rejected |
| Delete old rows in local drill | Exercises cleanup | Invents retention authority and can destroy replay evidence | Rejected |
| **Normal-planner EXPLAIN over bounded synthetic evidence volume plus explicit retention policy** | Auditable query evidence without unsafe deletion; clear production follow-on | Local volume is not production capacity; partition/archive work remains | **Selected in ADR-0113** |

### PostgreSQL point-in-time recovery evidence

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Logical dump/restore only | Simple, portable, already reproducible | No arbitrary WAL target; changes after dump can be lost | Rejected as the sole recovery gate |
| Streaming replica only | Low recovery point gap and operationally familiar | Can replicate corruption/operator error; not an independent historical restore | Rejected as the sole recovery gate |
| Managed-provider PITR only | Strong production integrations and off-site durability | Provider-specific, not reproducible in a clean checkout | Required production evaluation, not the repository-only gate |
| **Physical base backup plus archived WAL with exact-LSN replay** | Proves PostgreSQL-native PITR boundary in Docker; deterministic and auditable | Local files are not encrypted, durable, or production-scale | **Selected in ADR-0114** |

### Settlement RPC provider routing

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Fixed provider order | Deterministic and simple | Repeatedly hits degraded endpoints; poor tail latency | Rejected |
| Per-request random provider | Spreads traffic without shared state | Unpredictable evidence and no failure quarantine | Rejected |
| Durable PostgreSQL/Redis health state | Shared across replicas | Adds hot-path network dependency and stale coordination | Rejected |
| Separate indexer and settlement health algorithms | Local ownership | Divergent cooldowns and operational semantics | Rejected |
| **Shared bounded process-local adaptive health** | Deterministic cooldown, bounded failover, low latency, disposable state | Requires per-process metrics and regional SLOs | **Selected in ADR-0115** |

### Settlement transaction budget

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Absolute 1,232-byte packet check only | Simple protocol boundary | No growth margin; compiler can approach the hard limit | Rejected |
| JSON/base64 envelope sizing | Easy API measurement | Does not represent validator wire serialization | Rejected |
| Live RPC sizing during compilation | Cluster-aware in theory | Adds hot-path latency and provider dependency | Rejected |
| Arbitrary extra static keys | Wallet flexibility | Wastes packet bytes and expands review surface | Rejected |
| **Deterministic legacy wire estimate plus exact static-key admission** | Pure, bounded, conservative, network-free | Compute/v0/ALT evidence remains separate | **Selected in ADR-0116** |

### Compute-unit and priority-fee policy

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| No compute-budget instructions | Smallest transaction shape | Leaves compute exhaustion and fee bidding to wallet/RPC defaults; not reproducible | Rejected |
| Client-selected compute limit and priority fee | Wallet flexibility | Client can alter execution budget and fee behavior after quote/compile; weakens replay and evidence binding | Rejected |
| RPC simulation on every signing request | Cluster-aware estimate | Adds a network round trip and provider variance to the hot path; simulation is not a settlement authority | Rejected |
| **Server-owned versioned policy serialized as exact compute-budget instructions** | Deterministic admission, explicit fee ceilings, replayable evidence, and bounded wallet behavior | Requires validator-backed calibration and policy rollout | **Selected in ADR-0162** |

### Settlement provider-health observability

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Logs only | Rich forensic context | Poor alertability; failure storms and sensitive cardinality | Rejected as sole signal |
| Endpoint URL metric labels | Easy provider identification | Sensitive topology and unstable cardinality | Rejected |
| Durable health state | Shared across replicas | Hot-path coordination and stale state | Rejected |
| **Fixed provider-index gauges from process-local snapshots** | Bounded, redacted, low overhead, alertable | Requires deployment-level provider-index mapping | **Selected in ADR-0117** |

### Tenant-scoped evidence-access explorer (ADR-0373)

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Reuse replay operations | No new route or navigation item | Mixes recovery actions with audit history and authorization semantics | Rejected |
| Generic JSON audit browser | Broad apparent coverage | Unbounded disclosure, weak tenant boundary, unstable payload/cardinality contract | Rejected |
| Separate warehouse/event projection | Scales long retention and analytics | Adds asynchronous freshness and another operational authority | Rejected for this slice |
| **Tenant-scoped bounded PostgreSQL read model over the existing ledger** | **Indexed, deterministic, low-latency, read-only, and privacy-bounded** | **Latest-N view is not a historical export** | **Selected** |

### Collapsible Material 3 navigation rail (ADR-0374)

| Option | Discoverability | Workspace space | Accessibility | Decision |
| --- | --- | --- | --- | --- |
| Permanent expanded sidebar | High | Low | Pass | Rejected: persistent density overwhelms evidence workflows |
| Icon-only rail | Low | High | Risk | Rejected: labels become tooltip-dependent |
| Responsive breakpoint-only collapse | Medium | High | Risk | Rejected: viewport is not a user preference |
| **User-controlled compact rail with semantic labels** | **High** | **High** | **Pass** | **Selected; see ADR-0374** |

### Bounded linear tax-base operations (ADR-0375)

| Option | Global semantic coverage | Determinism/replay | Bounded latency and review | Decision |
| --- | --- | --- | --- | --- |
| Continue additive-only sources | Low | High | High | Rejected: cannot represent common weighted/deduction bases |
| Repeated source identifiers as weights | Medium | Low | Medium | Rejected: ambiguous authoring and serialization |
| General expression AST | High | Medium | Low | Rejected: unbounded operator and legal-review surface |
| Floating-point or implicit rounding | Medium | Low | Medium | Rejected: non-replayable financial semantics |
| **Versioned linear terms with exact rationals and integral-atomic admission** | **High for linear regimes** | **High** | **High** | **Selected; see ADR-0375** |

### Settlement submission concurrency fence

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Database/advisory lock held across RPC | Serializes requests | Holds database resources across external I/O and is difficult to recover | Rejected |
| In-process mutex/single-flight | Low local overhead | Does not fence replicas or survive restart | Rejected |
| Duplicate `sendTransaction` with Solana deduplication | Simple caller path | Wastes provider capacity and does not bind the response to the verified message | Rejected |
| **Durable `signed -> submitting` compare-and-set with signature binding** | Cross-replica ownership, crash visibility, no database lock across RPC, safe reconciliation | Requires an explicit lifecycle state and operational reconciliation | **Selected in ADR-0118** |
| **Explicit accepted-but-ambiguous local RPC fixture mode** | Deterministic unknown-state and signature-reconciliation evidence without production fault-injection surface | Synthetic provider behavior; does not replace live crash/failover evidence | **Selected in ADR-0119** |
| **Provider hold + project-scoped API SIGKILL + signature reconciliation** | Exercises the real durable claim/restart boundary without timing races or direct row mutation | Local single-replica evidence; requires a controlled fixture | **Selected in ADR-0120** |
| **Pinned health future selected with the bounded service loop** | Failure propagation, cancellation, one bounded task, no detached lifecycle | Couples service loop and health-server lifecycle explicitly | **Selected in ADR-0121** |
| Bare `RuleSet` JSON | Small payload, no envelope | Cannot identify document compatibility or fail closed on future representations | Rejected |
| Schema field inside `RuleSet` | Self-describing domain object | Changes rule identity/digest and couples transport schema to financial semantics | Rejected |
| **Versioned `RuleSetDocument` envelope with bounded legacy replay adapter** | Explicit new contract, stable rule digest, expand/contract migration | Temporary compatibility branch for historical rows | **Selected in ADR-0122** |


### Solana program implementation

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Rust + Anchor-compatible interfaces | Account validation, IDL, ecosystem, test tooling, review familiarity | Macro abstraction and version compatibility | **Selected**; use explicit account constraints and profile hot paths |
| Native Rust without Anchor | Maximum control and smaller abstraction surface | More boilerplate, easier account-validation omissions, client contract burden | Use selectively for specialized low-level pieces only after profiling/ADR |
| Solidity transpiler/SVM compatibility | EVM skill reuse | Non-native semantics, weaker Solana account ergonomics, audit complexity | Rejected for Solana core; Solidity remains optional EVM adapter |
| Separate EVM chain | Existing Solidity ecosystem | Cross-chain atomicity/custody/bridge risk, violates Solana-first scope | Rejected for core settlement |

### Tax calculation architecture

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Full rule graph on-chain | Maximum on-chain determinism | Compute/account/upgrade limits, expensive rule evolution, poor global-tax fit | Rejected for Tax Engine engine |
| External tax SaaS only | Broad possible coverage | Vendor lock-in, opaque calculation, network dependency, weak replay | Rejected as authority; may be an evidence adapter |
| JavaScript-only engine | Fast UI integration | Numeric/runtime drift, harder shared authoritative service semantics | Rejected as authority |
| **Pure Rust engine + signed attestation** | Deterministic, typed, replayable, low latency, shared native/WASM core | Requires rule governance and signer security | **Selected** |
| **Pure Rust domain engine plus a warm Rust service adapter** | Independent batch/simulation scaling while retaining in-process hot-path execution and one semantic authority | Requires bounded cache, service identity, and deployment topology | **Selected; see ADR-0225** |

### Service runtime

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| **Rust + Tokio/Axum-style services** | Low overhead, explicit concurrency, shared domain language, strong numeric safety | Higher hiring/learning curve | **Selected** |
| Node.js | Strong web ecosystem and fast iteration | Event-loop blocking risks, duplicated Rust domain semantics, numeric pitfalls | Use only for tooling if justified |
| Go | Strong concurrency and operations | Duplicated domain model and numeric semantics | Rejected for core services |
| Java/Kotlin | Mature enterprise tooling | Higher memory/latency floor and duplicated domain model | Rejected for latency-critical core |

### Frontend delivery

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| **React + TypeScript + Vite SPA** | Wallet-centric client, static delivery, independent scaling, explicit user constraint | SEO/server rendering is separate work | **Selected** |
| Next.js | Full-stack and SSR | Explicitly prohibited; unnecessary server runtime | Rejected |
| Remix/server-rendered app | Good data loading model | Adds server runtime and deployment coupling | Rejected for core UI |
| Micro-frontends | Team autonomy | Runtime/version complexity and inconsistent UX | Rejected until organizational scale proves need |

### Persistence and async work

| Area | Selected | Alternatives rejected | Reason |
| --- | --- | --- | --- |
| Durable projections/evidence | PostgreSQL 18.x with native `uuid`/`uuidv7()` | PostgreSQL 17 or older, SQLite, document-only stores | Current supported major, native time-ordered UUID generation, transactions, replay queries, relational audit model |
| PostgreSQL recovery evidence | Docker-isolated `pg_dump`/`pg_restore` drill plus production PITR | Filesystem snapshot only, application row copier | Portable logical restore validates the schema/evidence contract; production still requires encrypted off-site backups and PITR |
| Tenant settlement account configuration | PostgreSQL tenant-scoped immutable route registry plus deterministic payer ATA derivation | Caller-supplied account tuples, RPC discovery in compile path, environment-only route | Removes account substitution while keeping the compile path bounded and avoiding RPC/ATA creation in the settlement hot path |
| Route activation verification | Dedicated bounded verifier with finalized `getMultipleAccounts`, provider failover, and append-only evidence | Verify on checkout, trust database rows, single-provider confirmed reads, on-chain registry reads | Keeps RPC and account parsing off the hot path while proving account topology at finalized commitment before activation |
| Revenue-plan reference input | Server-owned bounded UUIDv7 catalog for verified destinations, active split plans, and published policies | Browser-supplied digests/keys, client-side digest selection, generic search service | Keeps the API authoritative for tenant scope, lifecycle, effective windows, canonical digests, and payout topology while giving operators a business-readable selection flow |
| Disposable cache/leases | Redis | Memcached, cache-only in-process | Shared low-latency cache with explicit non-authority |
| Async delivery | Outbox + NATS JetStream when required | Kafka by default, direct fan-out | Keep broker off hot path; reduce operational weight while preserving replayable async work |
| Chain truth | Solana finalized state + receipt evidence | API database, WebSocket notification alone | Correctness and auditability |

### Merchant authentication and authorization

| Tenant onboarding | Auth-service invitation plus signup-time transactional binding | Caller-supplied tenant ID, single-tenant signup, hosted provider authority | Keeps tenant ownership server-side, preserves self-signup isolation, and binds identity/role/audit state atomically |

| Role administration | Auth-service routes with EdDSA verification and PostgreSQL transaction | Browser role selector, settlement API ownership, hosted IdP migration | Keeps identity authority in the auth bounded context, prevents client privilege escalation, and commits role state plus audit evidence atomically |

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| **OIDC-compatible EdDSA JWT validated against a configured JWKS key set** | Standard browser/service integration, offline verification on the quote hot path, key rotation by `kid`, no introspection round-trip | Requires issuer/audience governance and a safe key-distribution/reload process | **Selected for the first protected API slice**; fail closed when issuer, audience, keys, signature, or merchant scope are invalid |
| Opaque bearer tokens with introspection | Immediate revocation and central policy | Network dependency and tail latency in the money path; availability coupling | Rejected for synchronous quote authorization; may be used at an edge gateway with bounded caching |
| Mutual TLS only | Strong workload identity and no bearer replay | Poor browser UX; does not express end-user merchant scope | Rejected as the sole product authentication mechanism; retain for service-to-service deployment profiles |
| Caller-supplied merchant identifiers | Simple and low latency | No identity proof; cross-merchant data and idempotency risk | Rejected for any persisted quote or settlement operation |

### Tax-rule ownership and governance isolation

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Global rule catalog with API-only tenant predicates | Shared releases and simple catalog administration | One missed predicate permits cross-tenant policy disclosure or substitution; the database cannot enforce the boundary | Rejected |
| Nullable owner with a global fallback | Low migration friction | Ambiguous authority and accidental global access; fail-closed behavior is not enforceable | Rejected |
| Separate database/schema per tenant | Strong physical isolation | Operationally expensive and poor fit for bounded shared services | Rejected for the first boundary |
| **Non-null merchant-owned rule releases with composite indexed predicates and constraints** | Explicit authority, bounded lookup, database-enforced calculation association, duplicate release keys per tenant | Requires reviewed ownership backfill and tenant-aware provisioning | **Selected in ADR-0079** |

### SBF platform-tools acquisition

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Default cargo-build-sbf downloader only | Minimal upstream-aligned path | API/asset restrictions can block a pinned build even when the release asset is reachable | Rejected as the only path |
| Floating latest compiler tools | Convenient | Non-reproducible bytes and unreviewed compiler changes | Rejected |
| Check the compiler archive into Git | Offline operation | Excessive repository size and maintenance burden | Rejected |
| **Pinned browser asset, SHA-256 verification, ignored reusable cache** | Reproducible and auditable; supports local archive/mirror recovery without weakening trust | Initial large download and scheduled digest review | **Selected in ADR-0080** |

### On-chain policy-attestation verification

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| **Governed, cached PolicyAttestation PDA checked in O(1) account reads** | Small settlement instruction, bounded compute, exact amount binding, easy revocation and expiry checks | Registration is a separate governed transaction; freshness depends on bounded expiry | **Selected for the first production settlement boundary** |
| Fresh Ed25519 verification instruction on every settlement | Strongest per-transaction freshness and no cached policy account | More instruction/account bytes, compute and transaction-size pressure, signer-key registry complexity | Retain as a later high-assurance path when measured budgets and issuer registry are complete |
| API-only signature verification | Lowest on-chain cost | Solana cannot independently reject API substitution or stale/replayed policy data | Rejected as settlement authority |
| Recompute the global tax graph on-chain | Maximum local computation | Unbounded rule evolution, compute/account limits, poor global-tax fit | Rejected; the program verifies bounded attested outputs |

### On-chain issuer governance

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| **Merchant/mint-scoped issuer-registry PDA with separate governance authority** | O(1) registration authorization, explicit issuer rotation, bounded settlement setup, auditable state | Governance rotation is a separate transaction and must be protected by multisig/timelock outside the program | **Selected for the first governed issuer slice** |
| Arbitrary signer supplied with each attestation | Minimal account surface and implementation effort | Any signer could authorize a policy; no durable authority boundary or rotation audit | Rejected; violates issuer and governance separation |
| Global issuer allow-list PDA | Centralized lookup and simpler administration | Cross-merchant contention, broad blast radius, unclear jurisdiction/asset scope | Rejected for the hot path; may be an upper governance layer |
| Fresh signature-only verification with no registry | Strong cryptographic proof per transaction | No durable issuer activation/revocation or scoped key lifecycle; larger instruction path | Rejected as the sole authority; retain as a later complementary verification mode |

### Token program compatibility

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| **Anchor token interface with a strict extension-free account contract** | One audited transfer path for legacy SPL and Token-2022, bounded account model, low client branching | Token-2022 extensions require later compatibility contracts; unsupported extensions are unavailable | **Selected for the first compatibility slice** |
| Separate legacy-SPL and Token-2022 instructions | Explicit per-program behavior | Duplicated authority/conservation logic and larger audit surface; clients can select the wrong path | Rejected for the first slice |
| Accept all Token-2022 extensions through generic CPI | Broad nominal asset coverage | Transfer fees/hooks/delegates/confidential semantics can invalidate amount conservation and authority assumptions | Rejected; fail closed |
| Legacy SPL only | Smallest current implementation | Cannot make a truthful Token-2022 compatibility claim | Rejected as the target architecture; retained only as the initial historical slice |

### Document quote latency evidence

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Reuse aggregate quote measurements | No new harness work | Hides document payload, rule-scope, storage, and outbox differences | Rejected |
| Copy the aggregate benchmark into a second script | Isolated document workload | Duplicates timeout, credential, percentile, cleanup, and invariant logic | Rejected |
| Mixed aggregate/document workload | Capacity mix is closer to a fleet | Per-context regressions and durable attribution become ambiguous | Rejected as the primary gate |
| **Explicit document mode in the existing bounded Docker harness** | DRY, reproducible, and context-specific with shared safety gates | Adds one bounded CI workload | **Selected in ADR-0209** |
| External load tool or host-installed binary | Familiar ecosystem | Violates the Docker bootstrap contract and introduces toolchain drift | Rejected |

### Docker full-topology bootstrap

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Default `docker compose up` | Short command | Omits required profiles and has no explicit full-topology readiness contract | Rejected |
| Enable every Compose profile | Starts all declared services | Also launches one-shot conformance and browser-test jobs | Rejected |
| Host supervisor with custom polling | Flexible per-service control | Duplicates health semantics and depends on host tooling | Rejected |
| **Explicit long-running services with Compose `--wait`** | Pinned Docker boundary, health-derived readiness, no host toolchain | Starts more local containers and requires validator capabilities | **Selected in ADR-0210** |

### Docker host-port collision policy

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Fixed host ports | Simple copy/paste URLs | Collides with unrelated stacks even when Compose projects are isolated | Rejected |
| Random host-port allocation | Avoids most collisions | Breaks reproducible URLs and makes browser/wallet/test configuration unstable | Rejected |
| Host networking | Fewer published mappings | Removes isolation and broadens local security exposure | Rejected |
| **Explicit environment-backed host-port overrides** | Stable defaults, parallel topologies, unchanged internal contracts | Operators must choose and record a complete alternate map | **Selected in ADR-0211** |

### Settlement conformance evidence

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Pure helper/unit tests | Fast and deterministic | Cannot prove CPI, runtime account ownership, deployed serialization, or token deltas | Rejected as the only gate |
| In-process `solana-program-test` only | Fast native transaction tests | Does not exercise the pinned Dockerized Agave validator and external RPC boundary | Rejected as release evidence; may complement the suite later |
| **Pinned external validator plus TypeScript transaction client** | Exercises the deployed SBF artifact, exact client envelope, legacy SPL, Token-2022, PDA initialization, and RPC confirmation | Slower and requires a pinned SBF build with external tool downloads | **Selected** for localnet release evidence |
| Mocked token programs or fabricated account data | Easy to run | Can produce false compatibility claims | Rejected |

The conformance client is run in the dedicated pinned Docker image defined by
ADR-0168. Host-installed Node.js and pnpm were rejected because they violate
the clean-checkout Docker contract and can diverge from the locked CI runtime.
The Playwright browser image was also rejected because chain conformance does
not need a browser and should keep its runtime surface minimal.

### Refund-proof operational observability

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Logs only | No code or storage query | Not restart-safe, difficult to alert, and poor for backlog detection | Rejected |
| Per-claim/signature metric labels | Easy drill-down | Unbounded cardinality, privacy leakage, and memory/alerting hazards | Rejected |
| Redis-backed worker counters | Shared across replicas | Adds a non-authoritative dependency and can reset or diverge from durable evidence | Rejected |
| **PostgreSQL pending snapshot plus fixed operation metrics** | Durable backlog truth, bounded labels, low overhead, compatible with existing scrape contract | One bounded aggregate query and process-local request counters | **Selected; see ADR-0159** |

### Chain indexing and reconciliation

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| **Finalized RPC subscription signal plus bounded slot-range backfill** | Works with standard Solana RPC, repairs missed notifications, explicit commitment semantics, straightforward local testing | Requires checkpoint persistence, provider failover, and bounded backfill queues | **Selected for the first indexer slice** |

Provider selection uses a bounded process-local health score layered over the
finalized-page port. Static order and round-robin were rejected for degraded
provider tail latency; a remote health registry was rejected for adding a hot
path dependency. The selected pool has at most three providers, per-attempt
deadlines, saturating atomic scores, and capped cooldowns. It only reorders
attempts and never weakens Solana commitment or coordinator validation.
| WebSocket notifications as the source of truth | Low notification latency and simple happy path | Notifications can be dropped, duplicated, reordered, or arrive before the required commitment | Rejected as authority; use only as an acceleration signal |
| Geyser/plugin-only ingestion | High throughput and rich slot data | Operational and provider coupling; heavier local/deployment footprint | Rejected for the first portable boundary; retain as a scale adapter |
| Database/API status as settlement truth | Convenient product reads | Cannot prove finalized token movement or repair a missed chain event | Rejected; finalized Solana state is authoritative |

The selected application boundary validates provider pages before persistence:
serializable storage receives only the exact planned range, a complete page,
and a finalized watermark. The current adapter uses bounded HTTPS JSON-RPC and
at most three deadline-bounded providers; it preserves finalized commitment
across failover. In-memory cursors lose progress on crash, broker-only offsets
do not atomically commit projections, and overwrite upserts would hide
conflicting provider evidence.

The selected persistence adapter uses serializable PostgreSQL transactions with
checkpoint fencing and exact receipt replay. In-memory cursors lose progress on
crash, broker-only offsets do not atomically commit projections, and overwrite
upserts would hide conflicting provider evidence.

### API authorization claims

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| **JWT `scope`/`roles` claims enforced at each service boundary** | Offline low-latency authorization, explicit least privilege, auditable policy, no extra request hop | Claim issuance and revocation policy remain identity-provider responsibilities | **Selected** |
| Authentication-only merchant claim | Smallest implementation | Any authenticated principal could invoke every merchant operation | Rejected; identity is not authorization |
| Remote authorization introspection on every request | Immediate central policy changes | Adds network tail latency and availability coupling to the quote path | Rejected for the synchronous path; may be used for bounded control-plane workflows |
| API gateway-only authorization | Centralized edge policy | Internal routes can be called directly unless every service repeats policy | Rejected as sole enforcement; gateway policy may be defense in depth |

### Product UI data boundary

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| **Authenticated production quote workspace with isolated preview mode** | Real API contract is exercised, evidence states are honest, no client-side money authority, useful operator workflow | Wallet signing and settlement require a later audited adapter | **Selected** |
| Client-side tax calculation as checkout authority | Instant feedback and offline behavior | Numeric/rule drift, no published-rule evidence, easy to mistake preview for authority | Rejected as production checkout authority; retain only as visibly labeled preview |
| Static dashboard with fabricated live states | Easy demonstration | Misleading money/settlement status and no recoverable failure semantics | Rejected |
| Server-rendered/full-stack UI | Integrated data loading | Violates React/Vite SPA and NGINX-only delivery constraint | Rejected |

### Durable asynchronous publication

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| **Transactional PostgreSQL outbox** | Calculation/evidence and event commit atomically; replayable; no broker dependency in the quote path | Requires a bounded dispatcher and cleanup/retention policy | **Selected for the first durable event boundary** |
| Publish to a broker before database commit | Fast fan-out and broad integration options | Lost or duplicated events around partial commit; broker becomes part of the money path | Rejected |
| Direct webhook/event fan-out in the request | Simple consumer integration | Unbounded latency, retry storms, and external availability coupling | Rejected |

### Outbox delivery transport

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| **NATS JetStream with PostgreSQL lease claims** | Low latency, durable acknowledgements, subject-based fan-out, message-id deduplication, bounded operational footprint | At-least-once delivery still requires idempotent consumers and broker operations | **Selected**; PostgreSQL remains the durable handoff and JetStream is downstream transport |
| Kafka | Mature partitioning and ecosystem | Higher local/operational cost than the first workload requires; more latency and resource overhead | Rejected for the first transport boundary; revisit at measured scale |
| Redis Streams | Familiar and fast | Cache/coordination semantics can be confused with evidence authority; weaker separation of concerns | Rejected as the durable event transport |
| Direct PostgreSQL polling by every consumer | Minimal dependencies | Consumer contention, duplicated query logic, and no shared delivery contract | Rejected |

### Local development and packaging

The workspace uses Rust 1.98.1 for off-chain services, while the selected
Agave 4.3.0 / platform-tools v1.57 SBF compiler requires the on-chain package
to declare a Rust 1.95 compatibility floor. This split is recorded in
[ADR-0067](../adr/0067-sbf-compiler-compatibility-boundary.md); it is not an
invitation to bypass Cargo's version checks.

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| **Docker Compose with pinned stable images/digests** | Reproducible local topology, low onboarding cost, production-like boundaries | Desktop resource use, Compose is not production orchestration | **Selected for local development** |
| Kubernetes locally | Production-like orchestration | High complexity and slow feedback | Rejected for default local workflow |
| Host-installed toolchains | Fast for experts | Drift, onboarding failure, inconsistent versions | Rejected as default |
| Floating `latest` image tags | Easy updates | Non-reproducible and unsafe | Rejected; resolve latest stable then pin digest |

### Local cache and observability topology

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| **Profiled Redis + Prometheus + Grafana + OpenTelemetry Collector** | Explicit failure domains, bounded metrics, reviewable dashboards, Docker-only reproducibility | Uses local CPU/memory and requires image maintenance | **Selected for local development** |
| Host-installed monitoring tools | Familiar to operators | Violates clean-checkout and toolchain reproducibility requirements | Rejected |
| One all-in-one monitoring image | Fewer containers | Blurred ownership, larger blast radius, opaque upgrades | Rejected |
| Remote telemetry in the request path | Centralized production data | Adds latency and availability coupling to money paths | Rejected |

### Edge admission control

| Alternative | Strengths | Risks/trade-offs | Decision |
|---|---|---|---|
| **NGINX bounded per-source-IP rate and connection limits** | Rejects overload before API/database work, deterministic local behavior, no hot-path dependency | NAT concentration and production tuning require measured limits | **Selected as defense in depth; ADR-0074** |
| API-only limits | Tenant-aware and easy to evolve | Still consumes edge sockets and request parsing capacity | Rejected as the sole control |
| Redis-backed distributed edge quotas | Shared limits across replicas | Adds network latency and a failure dependency to every request | Deferred to authenticated tenant quotas |
| Cloud WAF/CDN-only controls | Managed global perimeter | Vendor coupling and no protection for direct/self-hosted edge access | Rejected as the only control |

### Tax-rule governance workflow

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| **Authenticated API transition plus serializable PostgreSQL event transaction** | Explicit state machine, four-eyes separation, durable evidence, bounded synchronous control path | Requires identity-provider role lifecycle and operator ceremonies | **Selected; ADR-0076** |
| Database-only status mutation | Simple and durable | Cannot prove who authored, reviewed, or approved a release | Rejected |
| Workflow broker as authority | Rich orchestration and notifications | Adds asynchronous divergence and delivery dependency to publication correctness | Rejected as authority; may be used for notifications |
| Single administrator promotion role | Operationally simple | Violates separation of duties and increases blast radius | Rejected |

### Tax-rule governance read model and UI

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| **Authenticated repeatable-read PostgreSQL snapshot plus React operator workspace** | Current status and immutable event history are shown from one source, bounded control-plane latency, no projection lag | Requires explicit timeout/conflict/error UX and read-role provisioning | **Selected; ADR-0077** |

### Service-level latency evidence

| Option | Benefits | Costs / risks | Decision |
| --- | --- | --- | --- |
| Domain-only Criterion benchmark | Very fast feedback and stable hot-path signal | Omits HTTP, authentication, PostgreSQL contention, serialization, and outbox work | Rejected as the only latency evidence |
| Unpinned external load binary | Rich load-generation features | Toolchain drift and weaker clean-checkout reproducibility | Rejected |
| Unbounded request flood | Easy to write | Measures overload, creates retry storms, and cannot establish a declared budget | Rejected |
| **Pinned Docker stack with bounded `curl` workers** | Reuses required tooling, isolates state, records every response, and is easy to audit | Less feature-rich than a dedicated load platform; production regional evidence remains separate | **Selected; ADR-0078** |
| Reuse the quote workload for persisted-rule simulation | Avoids another script | Mixes read-only simulation with quote persistence, idempotency, outbox, and jurisdiction-provider behavior | Rejected; ADR-0145 |
| **Pinned two-replica simulation workload with facts-only response invariants** | Measures the governance read path, warm compiled-rule cache, pool pressure, and server-owned binding without financial writes | Local Docker latency is not regional production evidence | **Selected; ADR-0145** |
| Client-only lifecycle state | Fast local interaction | Stale or fabricated state can mislead operators and cannot prove evidence | Rejected |
| Separate event-store projection as immediate authority | Scales independent reads | Projection lag and failure can misrepresent publication state | Rejected as authority; may be added for analytics |
| Browser-decoded JWT role gating | Convenient affordances | Client claims are not an authorization boundary | Rejected |

### Emergency settlement control

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| **Singleton on-chain pause PDA checked before settlement CPI** | Directly protects wallet transactions, deterministic, low overhead, auditable | Requires governance signer custody and an explicit initialization transaction | **Selected** |
| API or NGINX kill switch | Fast to operate off-chain | Does not stop direct wallet transactions or already-built transactions | Rejected |
| Per-policy or per-merchant pause only | Fine-grained control | Cannot stop a shared program/token integration incident immediately | Rejected as the sole control |
| Program upgrade to disable settlement | Powerful | Slower, broader privilege, and harder to audit as an incident action | Rejected as the first response |

### Pause evidence and alerting

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Logs-only alerting | Minimal implementation | Not restart-safe, difficult to replay, and weak under provider failover | Rejected |
| API/NGINX kill switch | Fast off-chain admission control | Cannot stop direct wallet transactions and can misrepresent chain state | Rejected as authority |
| Database-owned pause flag | Simple operator read | Makes a projection a second settlement authority | Rejected |
| **Finalized `ProtocolPauseChanged` projection with fixed-cardinality metrics** | Replayable, restart-safe, auditable, bounded alert labels, subordinate to chain truth | Requires event decoding, migration, and independent staging verification | **Selected; ADR-0131** |

### Settlement instruction compilation

| Alternative | Benefit | Risk/cost | Decision |
|---|---|---|---|
| **Pure Rust compiler with derived PDAs and fixed account order** | Deterministic bytes, bounded validation, wallet review, no custody or RPC dependency | Transaction wrapping and blockhash policy remain separate | **Selected** |
| Browser-constructed arbitrary account metas | Fewer API fields | Account substitution and duplicated serialization logic | Rejected |
| API-held payer signing key | Operationally simple submission | Custody, key compromise, and loss of user authorization | Rejected |
| RPC-coupled compiler | Can attach a fresh blockhash immediately | Network latency and provider availability enter deterministic domain code | Rejected |

### Cross-language settlement serialization

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| **Shared versioned JSON golden vectors consumed by Rust and TypeScript** | One reviewable contract for program ID, bytes, PDAs, account order, and flags; catches serializer drift | Requires deliberate vector versioning and synthetic fixture maintenance | **Selected** |
| Rust-only compiler tests | Smallest test surface | Does not prove browser wallet construction preserves the artifact | Rejected |
| TypeScript-only snapshots | Convenient frontend feedback | Makes the browser an accidental protocol authority | Rejected |
| Shared Rust/WASM compiler in the browser | Reduces duplicate derivation logic | Larger bundle and runtime coupling; requires measured browser performance | Deferred pending evidence |

### Threshold and exemption semantics

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| **Explicit threshold basis/mode plus signed cumulative snapshot input** | Deterministic, replayable, supports excess-only and trigger semantics, keeps the hot path pure | Requires upstream evidence ownership and legal review of each rule | **Selected** |
| Read cumulative state from PostgreSQL during calculation | Convenient integration | Race-prone, nondeterministic, and adds a database dependency to the domain path | Rejected |
| Caller-provided exemption boolean | Simple request shape | Unverifiable and not suitable for audit or attestation | Rejected |
| Model thresholds as credits | Reuses an existing field | Changes tax output rather than the taxable base and cannot express activation semantics | Rejected |

### Tax rounding scope and residual allocation

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Implicit component rounding | Backward-compatible by omission | Cannot express aggregate settlement rounding and hides the legal calculation scope | Rejected |
| Mixed component and settlement scopes | Flexible authoring | Dependent components have ambiguous rounded-versus-exact inputs | Rejected |
| Total rounding with residual discard | Minimal arithmetic | Breaks conservation and creates unexplained atomic-unit loss | Rejected |
| **Explicit component/settlement scope with bounded largest-remainder allocation** | Deterministic, replayable, conserving, and compatible with tax-on-tax component graphs | Settlement scope is restricted to independent components | **Selected in ADR-0202** |
| Add line/invoice facts in the rounding change | Broader coverage | Expands the fact, API, storage, UI, and golden-vector contracts beyond one bounded decision | Deferred to a dedicated line-facts ADR |

### Tax decimal-scale representation

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Binary floating-point decimal places | Familiar | Unsafe financial boundaries and cross-runtime drift | Rejected |
| Adapter-specific decimal strings | Human-readable | Parser and currency-precision divergence | Rejected |
| **Positive `quantum_atomic` integer** | Exact, asset-precision aware, language-neutral, and bounded | Rule authors must declare the asset's atomic increment | **Selected in ADR-0202** |

Quantum compatibility is also explicit: floor and cap boundaries that are not
exact multiples of the declared quantum are rejected, preserving the meaning
of min/max semantics after final rounding.

### Line and invoice tax facts

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Reuse aggregate `TaxFacts` for invoices | Minimal schema | Loses line identity and makes rounding/replay ambiguous | Rejected |
| Browser-owned line aggregation | Fast preview | Caller-controlled evidence and cross-language numeric drift | Rejected |
| **Versioned bounded document facts with explicit line/invoice scope** | Canonical identity, bounded work, conserving allocation, replayable evidence | Adds a separate domain/result contract | **Selected in ADR-0203** |

### Production document quote persistence

| Option | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Reuse aggregate `tax_calculations` | Small migration | Shape confusion and unsafe settlement/attestation reuse | Rejected |
| Browser/object-storage persistence | Flexible and responsive | Caller-controlled authority and missing transactional evidence | Rejected |
| **Dedicated PostgreSQL document-calculation adapter** | Explicit type firewall, tenant scope, idempotency, encrypted replay, outbox | Additional schema and API surface | **Selected in ADR-0204** |
| Unbounded generic line rule graph | Broad expressiveness | Unbounded latency/memory and difficult audit/attestation limits | Rejected |

### Document-calculation attestation

| Option | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Reuse aggregate `CalculationAttestation` | No new signer contract | Cannot distinguish document and aggregate authority; unsafe settlement reuse | Rejected |
| Add document fields to aggregate schema v1 | One wire type | Breaks existing signed payloads and verifier compatibility | Rejected |
| Signed ad-hoc JSON envelope | Flexible | Ambiguous canonicalization and domain separation | Rejected |
| **Dedicated policy contract, persistence table, API route, and outbox event** | Explicit scope firewall and independent replay | Additional audited contract surface | **Selected in ADR-0205** |

### Calculation-attestation cross-language compatibility

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Rust-only canonical-payload tests | Minimal implementation | TypeScript serialization drift can change the authority-bound digest unnoticed | Rejected |
| JSON serialization comparison | Easy to inspect | JSON ordering, UTF-16 text lengths, and numeric coercion are not the signed byte contract | Rejected |
| Browser-generated or browser-signed attestations | Low apparent interaction latency | Makes an untrusted client an issuer authority and permits evidence substitution | Rejected |
| **Versioned binary golden vector consumed by Rust and TypeScript** | Exact byte/digest parity, deterministic review, no network dependency | Requires explicit fixture maintenance and mutation tests | **Selected; see ADR-0224** |

### Document-to-payment binding

| Option | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Reuse aggregate `settle` with document totals | Smallest code change | Loses document provenance on chain and permits aggregate/document coercion | Rejected |
| Put document identity only inside the policy digest | No program instruction change | Chain evidence cannot independently distinguish source and payment scope | Rejected |
| Prorate arbitrary partial payments from stored totals | Flexible checkout behavior | Incorrect for progressive, thresholded, cascading, capped, credited, or rounded rules | Rejected |
| Put document lines/rule graph on Solana | On-chain visibility | Unbounded compute/account/packet cost, privacy leakage, and poor rule evolution | Rejected |
| **Dedicated domain plus `settle_document` and document receipt** | Type firewall, deterministic whole-line semantics, on-chain provenance | Requires a separately audited instruction and event | **Selected in ADR-0206** |

## Re-evaluation triggers

Re-open these decisions when:

- measured p99 breaches the documented budget after profiling;
- the rule graph exceeds bounded compute/memory limits;
- transaction account/compute limits require a new batching model;
- legal/tax governance requires a new evidence or privacy model;
- service volume or team topology justifies a different event-stream architecture;
- a selected dependency loses security support or Solana compatibility.

## Identifier policy

UUIDv7 is selected for internal sortable IDs because it provides distributed uniqueness with time-ordered locality. It is not used as a security secret or as a replacement for domain ordering. Strict ordering uses an explicit sequence/slot/offset, and cryptographic evidence uses a content digest. Solana public keys and transaction signatures remain native strings/bytes with their native validation.

## Compiled tax-rule cache

The quote adapter uses a bounded process-local LRU keyed by the canonical
32-byte rule digest. The database still decides whether a rule is published
and effective; the cache only stores the immutable compiled graph after digest
verification. An unbounded map, a rule-ID-only key, and Redis on the synchronous
quote path were rejected for memory, integrity, and tail-latency reasons.

### Browser security policy

| Alternative | Benefit | Risk/cost | Decision |
| --- | --- | --- | --- |
| **Strict same-origin CSP and security headers at NGINX, with Docker integration assertions** | Protects static and API responses at the first boundary and keeps policy deterministic | New wallet/telemetry origins require explicit review | **Selected; see ADR-0081** |
| React document meta tags only | Simple frontend ownership | Does not cover error/API responses and can be removed by a future document change | Rejected |
| Per-service application middleware | Flexible | Header drift and incomplete coverage across static assets and services | Rejected |
| Wildcard CORS or permissive CSP | Broad integration compatibility | Increases data-exfiltration and injection blast radius | Rejected |

### Request correlation authority

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| **API-owned UUIDv7 middleware** | Covers direct, edge, auth-failure, and handler-failure paths; sortable support IDs; one implementation boundary | Small per-request UUID allocation; downstream services must propagate explicitly | **Selected; see ADR-0082** |
| NGINX `$request_id` as the system ID | Available at the first edge | Non-UUIDv7, absent on direct API traffic, and not authoritative to Rust services | Rejected |
| Caller-supplied `x-request-id` | Useful for client correlation | Spoofable and can introduce unbounded or invalid log values | Rejected |
| UUIDv4 framework helper | Mature middleware integration | Violates the repository’s UUIDv7 correlation policy | Rejected |

### Rust trace export

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| **Shared `tracing` + bounded OTLP gRPC exporter** | Uses existing spans, preserves JSON logs, one redaction/configuration boundary, collector portability | Adds pinned OpenTelemetry dependencies and a lifecycle boundary | **Selected; see ADR-0201** |
| Prometheus-only metrics and JSON logs | Lowest dependency cost | No distributed span context or collector trace evidence | Rejected for Tax Engine operations |
| Synchronous exporter in request paths | Immediate delivery | Couples business latency and availability to telemetry | Rejected |
| Per-service custom exporters | Local control | Configuration and privacy drift across services | Rejected |

### UUIDv7 schema evidence

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| **Catalog-level SQL contract executed in clean-stack and restore drills** | Verifies final PostgreSQL defaults, native types, and version checks; catches migration drift | Requires maintaining an explicit generated-ID inventory | **Selected; see ADR-0083** |
| Static migration-file inspection | Easy to run | Does not prove the final catalog or restored database | Rejected |
| Application-only UUID tests | Covers one generator path | Direct SQL and database defaults can drift independently | Rejected |
| UUIDv7 for every UUID-shaped value | Superficially uniform | Misclassifies secrets, hashes, external IDs, and Solana-native identifiers | Rejected |

### Alert evaluation and validation

| Option | Advantages | Costs/risks | Decision |
| --- | --- | --- | --- |
| Dashboard-only visibility | Low setup cost | Human detection, no paging contract, poor failure detection | Rejected |
| Service-local alert delivery | Close to the failure | Duplicated policy, hot-path coupling, notification side effects in services | Rejected |
| Managed-provider-only rules | Managed operations | Not reproducible or portable from the public repository | Rejected |
| **Versioned Prometheus rules validated by pinned `promtool`** | Reviewable, portable, bounded labels, CI-verifiable, low overhead | Production receiver wiring remains environment-specific | **Selected; see ADR-0084** |

### Validator conformance evidence

| Option | Advantages | Costs/risks | Decision |
| --- | --- | --- | --- |
| CI console output only | No extra files | Weak forensic context and no stable artifact hash bundle | Rejected |
| Upload the entire workspace/ledger | Maximum context | Excessive size, generated state, and secret exposure risk | Rejected |
| Boolean pass/fail status only | Minimal | Cannot diagnose validator/runtime failures or audit exact inputs | Rejected |
| **Bounded public-safe evidence bundle retained on success and failure** | Exact metadata, program hash, logs, test output, and reproducible review | Requires retention policy and careful redaction | **Selected; see ADR-0085** |
| Trust the conformance process exit code | Minimal CI configuration | Does not prove Linux capability, pinned runtime, artifact identity, or complete case coverage | Rejected |
| Privileged Docker workaround for constrained kernels | May start Agave in more environments | Expands privilege and changes the evidence environment | Rejected |
| **Bounded postcondition verifier on Linux CI/release** | Preserves local fail-closed diagnostics and makes runtime/artifact/test evidence reviewable | Adds an explicit release gate | **Selected; see ADR-0199** |

### Replay-gap operator control

| Option | Advantages | Costs/risks | Decision |
| --- | --- | --- | --- |
| Manual SQL | Immediate access | Unauthenticated mutation, no bounded audit trail, easy to confuse repair with acknowledgement | Rejected |
| Merchant-scoped API | Reuses existing tenant auth | Replay streams are system-level; merchant claims are not a safe operator boundary | Rejected |
| Acknowledge by marking completed | Simple state model | Falsely represents finalized projection repair | Rejected |
| **System operator role/scope plus append-only action ledger** | Explicit authority, atomic requeue, auditable and replayable | Requires operator identity governance and runbook | **Selected; see ADR-0086** |

### Replay-gap operator UI

| Option | Advantages | Costs/risks | Decision |
| --- | --- | --- | --- |
| SQL-only operations runbook | No frontend work | Poor evidence context and bypass risk | Rejected |
| Generic JSON/admin console | Fast to expose endpoints | Unbounded input and unsafe rendering of unvalidated state | Rejected |
| Cached always-on dashboard | Convenient monitoring | Stale state can imply repair or finality | Rejected |
| **Lazy-loaded typed React workspace using the authenticated API** | Bounded input, explicit failures, accessible review flow, no local authority | Requires a dedicated parser and action-state model | **Selected; see ADR-0087** |

### Indexer projection rebuild

| Option | Advantages | Costs/risks | Decision |
| --- | --- | --- | --- |
| Rebuild from normalized projections or a database backup | Simple operational path | Cannot independently detect projection corruption | Rejected |
| Re-query live RPC during recovery | Fresh provider data | Provider availability, finality timing, and provider-specific encoding become recovery dependencies | Rejected |
| SQL-only decoding of Anchor bytes | Avoids a service binary | Duplicates protocol semantics and weakens typed validation | Rejected |
| One unbounded transaction/export | Easy implementation | Lock, timeout, and memory blast radius at scale | Rejected |
| **Rust canonical decoder with keyset-paginated bounded batches** | Reuses event invariants, supports clean rebuilds, and limits resource usage | Requires an operational drill and projection-specific contracts | **Selected; see ADR-0088** |

### Split-settlement evidence projection

| Alternative | Advantages | Risks | Decision |
| --- | --- | --- | --- |
| Treat `SplitSettlementExecuted` as a generic receipt | Minimal schema change | Loses split-plan version/digest and weakens replay and reconciliation evidence | Rejected |
| Infer split recipients from transaction account metas | No additional event decoder | Account order is not a durable executed-state contract; cannot prove recipient amounts | Rejected |
| Retain only raw split event bytes | Strong forensic preservation | Makes bounded operational queries and support workflows need repeated decoding | Rejected |
| **Typed split projection plus immutable raw evidence** | Preserves event identity, bounded queries, idempotent replay, and recovery | Requires a migration and a second typed merge path | **Selected; ADR-0093** |

### PostgreSQL contention evidence

| Option | Advantages | Costs/risks | Decision |
| --- | --- | --- | --- |
| Unit-test timeout constants only | Fast and deterministic | Does not exercise PostgreSQL lock cancellation, SQLSTATEs, migrations, or indexes | Rejected |
| Full quote load only | Covers HTTP, idempotency, and connection use | Does not create a deterministic lock or statement-timeout scenario | Rejected |
| Unpinned `pgbench` workload | Broad throughput signal | Not a stable contract for exact schema and lock invariants | Rejected |
| **Pinned Docker PostgreSQL with targeted SQL contention and catalog assertions** | Reproducible, bounded, and auditable | Production saturation and regional evidence remain separate | **Selected; see ADR-0089** |

### Idempotency contention evidence

| Option | Advantages | Costs/risks | Decision |
| --- | --- | --- | --- |
| Independent idempotency keys only | Simple and parallel | Misses the shared-row serialization/replay boundary | Rejected |
| Unit-test the idempotency helper | Fast | Does not exercise PostgreSQL locking, retries, or the outbox under HTTP concurrency | Rejected |
| Unbounded stress test | Broad pressure | Noisy CI and weak deterministic correctness signal | Rejected |
| **Bounded same-key Docker workload with database convergence assertions** | Real API/database path, finite, and auditable | Separate production-scale contention evidence is still needed | **Selected; see ADR-0090** |

### Revenue split execution

| Option | Advantages | Costs/risks | Decision |
| --- | --- | --- | --- |
| Keep one destination and distribute off-chain | Smallest instruction | Loses atomic conservation and introduces custody/credit risk | Rejected |
| Arbitrary recipient array supplied by the caller | Flexible | Unbounded fan-out and recipient substitution risk | Rejected |
| Asynchronous escrow per recipient | Isolated recipient failure | More state, fees, latency, and recovery paths | Rejected |
| Replace the existing v1 instruction | One contract | Breaks signed-message and deployed-client compatibility | Rejected |
| **Immutable bounded Split Plan PDA with additive split instruction** | Atomic, deterministic, auditable, bounded, and backward compatible | Requires a new plan lifecycle and conformance evidence | **Selected; see ADR-0091** |

### Authenticated Split Plan resolution

| Option | Advantages | Costs/risks | Decision |
| --- | --- | --- | --- |
| Caller-provided recipient list | Minimal API work | Destination substitution and no governed evidence | Rejected |
| Redis-only plan storage | Low-latency reads | Disposable and not authoritative or auditable | Rejected |
| On-chain-only plan lookup | Strong chain authority | RPC in quote/compile hot path and poor tenant query ergonomics | Rejected |
| Route table owns plan fields | One lookup | Couples account topology and plan versioning | Rejected |
| **Tenant-scoped PostgreSQL registry plus on-chain PDA verification** | Auditable, cacheable, quote-bound, and chain-authoritative | Requires immutable lifecycle and activation evidence | **Selected; see ADR-0092** |

### Governed Split Plan checkout review

| Alternative | Advantages | Costs/risks | Decision |
| --- | --- | --- | --- |
| Accept browser recipient JSON | Flexible client UX | Destination substitution and no governed evidence | Rejected |
| Fetch a plan by opaque ID only | Small API surface | No discoverable review, encouraging guessed IDs | Rejected |
| Redis-only plan read model | Low-latency reads | Disposable cache cannot validate canonical evidence | Rejected |
| **Authenticated bounded PostgreSQL list plus ID-bound compiler** | Tenant-scoped, canonical, reviewable, and reuses chain-bound compiler | Adds a bounded read endpoint and explicit UI state machine | **Selected; ADR-0094** |

### Browser API deadline and cancellation policy

| Alternative | Advantages | Costs/risks | Decision |
| --- | --- | --- | --- |
| No browser deadline | No timeout configuration | Indefinite pending UI, stale authorization context, repeated user actions | Rejected |
| Ad-hoc `AbortSignal.timeout` at each call site | Small local changes | Duplicated policy and inconsistent cancellation/testing | Rejected |
| Browser retries for all failures | May mask transient reads | Can duplicate settlement side effects and hide provider ambiguity | Rejected |
| **Shared bounded fetch boundary with explicit read/submit budgets** | One tested cancellation contract with endpoint-specific risk budgets | Requires all call sites to use the boundary | **Selected; ADR-0095** |

### Rule-draft authoring authority

| Alternative | Advantages | Costs/risks | Decision |
| --- | --- | --- | --- |
| Database fixtures/operator SQL | Fast for bootstrap | Bypasses tenant auth, idempotency, audit, and product controls | Rejected |
| Client-only draft state | No backend work | Cannot support review, replay, durable evidence, or multi-operator work | Rejected |
| Direct publication on create | Small workflow | Bypasses four-eyes review and effective-window governance | Rejected |
| External SaaS-owned rule document | Delegates rule authoring | Opaque semantics and dependency; cannot own protocol digest/evidence | Rejected |
| **Authenticated Rust API with transactional PostgreSQL/outbox creation** | Canonical validation, tenant isolation, idempotency, evidence, and real UI workflow | Requires migration and bounded JSON authoring form | **Selected; ADR-0096** |

### Settlement submission and lifecycle

The accepted design uses a bounded server-side coordinator after wallet
signing. It reuses the hardened JSON-RPC transport, preserves immutable signed
bytes across at most three provider attempts, and separates `submitted` from
chain commitment. PostgreSQL UUIDv7 intents and append-only events fence
idempotency and make a provider timeout `unknown` rather than silently creating
a duplicate. Custodial signing, browser-only submission as the sole path, and
unbounded retry/rebuild strategies are rejected; see ADR-0056 and ADR-0057.
ADR-0058 adds the mandatory cryptographic message-binding gate: the exact
compiled instruction, program, account domain, signer/writable policy, and
wire transaction signatures are independently verified before durable state or
RPC submission. Address-table lookups and extra instructions remain rejected
until separately governed.

### Checkout wallet handoff

The first real checkout handoff uses an explicit browser wallet port backed by
the latest stable `@solana/web3.js` 1.x maintenance release. It builds one
legacy transaction from the server compiler artifact, uses a confirmed recent
blockhash, requests the wallet signature, and submits unchanged base64 bytes to
the server coordinator. `@solana/kit` 8.x is the strategic SDK direction but is
deferred for the first compatibility slice because injected wallets still
commonly expose the web3.js `Transaction` boundary; wallet-standard support
must be proven against the supported provider matrix before switching. Wallet
adapter registries, server-side signing, and browser-only RPC submission were
rejected; see ADR-0059.

### Wallet Standard compatibility boundary

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Legacy `window.solana` only | Minimal bundle and implementation | Cannot enumerate providers or prove advertised capabilities | Rejected |
| **Wallet Standard discovery with explicit selection and legacy fallback** | Capability-aware multi-wallet support while preserving the existing web3.js and server signed-byte boundary | Adds bounded selection/lifecycle state and exact dependencies | **Selected; ADR-0200** |
| Implicitly use the first account returned by a provider | Lowest interaction cost | Can authorize the wrong signer and cannot be safely recovered after compilation | Rejected |
| **Accessible explicit account selector after `standard:connect`** | Preserves signer intent and makes multi-account ambiguity visible before signing | Adds one bounded selection step for multi-account wallets | **Selected; ADR-0200** |
| Full Solana Wallet Adapter registry | Broad adapter coverage | Larger bundle and lifecycle abstraction than this product boundary requires | Rejected for this slice |
| Direct browser `sendTransaction` | Fewer API calls | Bypasses durable intent/reconciliation authority | Rejected |

### Jurisdiction resolution

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Caller-provided jurisdiction string | Lowest latency and simplest API | Caller substitution, no evidence provenance, no replayable precedence | Rejected |
| Per-provider precedence logic | Flexible provider integrations | Duplicated semantics and inconsistent conflict handling | Rejected |
| External tax SaaS authority | Apparent global coverage | Opaque rule/evidence binding, outage and vendor lock-in risk | Rejected |
| On-chain jurisdiction resolution | Settlement-visible result | Privacy, changing law, unbounded data, and account/compute constraints | Rejected |
| **Pure Rust versioned policy resolver over authenticated evidence** | Deterministic, provider-agnostic, replayable, low-latency after compilation, fail-closed | Requires legal/tax-owned policies and evidence adapters | **Selected; see ADR-0101** |

### Binding jurisdiction evidence to calculation facts

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Keep only the jurisdiction string | No schema migration | No provenance or replay proof | Rejected |
| Store a caller-provided digest | Small payload | A digest alone does not prove resolver execution | Rejected |
| Store raw address/KYC evidence in every calculation | Maximum local context | PII, retention, payload-size, and privacy risks | Rejected |
| Re-resolve during replay | Fresh provider data | Historical results drift and replay gains network dependencies | Rejected |
| **Server-owned bounded resolution envelope in facts/results** | Digest-bound, replayable, privacy-minimized, backward compatible | Requires application resolver integration and explicit legacy handling | **Selected; see ADR-0102** |

### Production quote jurisdiction authority

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Caller-supplied candidates or evidence digests | Simple and fast | Forged evidence and no provider provenance | Rejected |
| Synchronous external tax/geocoding call in quote path | Broad apparent coverage | Latency, outage, replay drift, opaque precedence | Rejected |
| Redis/configuration-only policy | Fast reads | Non-authoritative and difficult to audit or rotate safely | Rejected |
| Global policy registry independent of the selected rule release | Reusable policies | Effective-window and tenant-selection skew | Rejected |
| **Signed provider evidence plus immutable tenant policy bound to the published rule release** | Cryptographic provenance, bounded hot path, replayable, tenant-safe | Requires key governance and policy lifecycle | **Selected; see ADR-0103** |

### Jurisdiction policy governance

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Direct SQL or fixtures | Fast bootstrap | Bypasses identity, four-eyes review, idempotency, and audit | Rejected |
| Couple policy changes to rule JSON only | One approval surface | Cannot correct precedence independently; weak policy provenance | Rejected |
| External workflow/SaaS authority | Rich workflow features | Adds availability dependency and replay drift | Rejected |
| **Authenticated Rust API plus PostgreSQL four-eyes lifecycle** | Tenant-safe, bounded, replayable, auditable publication | Requires role provisioning and governance UI | **Selected; see ADR-0104** |

### Vault release control

| Option | Advantages | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| API-held release signer | Simple request path | Key custody in the API, weak separation, and difficult incident containment | Rejected |
| Single on-chain release authority | Small account model | One compromised or mistaken signer can move the full scoped vault balance | Rejected |
| Off-chain claim ledger followed by an unconstrained transfer | Rich workflow metadata | Database is not authoritative for token movement; destination substitution and replay remain possible | Rejected |
| **Merchant/mint registry with separate proposer/approver keys and immutable release-claim PDAs** | Four-eyes control, exact amount/destination binding, PDA-signed checked CPI, atomic replay protection | Adds registry and claim lifecycle state plus key-ceremony operational work | **Selected; see ADR-0105** |

### Settlement intent tenant boundary

| Alternative | Advantages | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Global intent lookup followed by an API merchant check | Minimal method signature | Persistence boundary can leak cross-tenant existence and future callers may omit the check | Rejected |
| PostgreSQL session tenant variable/RLS only | Centralized policy | Pooled-connection context installation and reset are easy to misconfigure; harder offline verification | Rejected as the sole control |
| Opaque merchant-prefixed intent identifiers | Hides some identifiers | Changes the public contract and conflates identity with authorization | Rejected |
| **Explicit merchant predicate on every intent read and state transition** | Auditable, fail-closed, compatible with UUIDv7 IDs, and independently testable | Adds one required domain parameter and intentionally returns 404 for cross-tenant IDs | **Selected; ADR-0107** |

### Sensitive tax-evidence persistence

| Area | Selected | Alternatives considered | Selection reason |
| --- | --- | --- | --- |
| Replay evidence at rest | Application AEAD over normalized facts with injected KMS/HSM key provider; raw jurisdiction envelopes discarded | Plaintext JSONB, PostgreSQL `pgcrypto`, disk encryption only, raw envelope retention | Minimizes retained evidence, binds ciphertext to tenant/digest, supports rotation and audited access without widening the settlement hot path |

### Evidence-key lifecycle

| Area | Selected | Alternatives considered | Selection reason |
| --- | --- | --- | --- |
| Key resolution | Injected provider with a bounded warm current/previous-key ring | Per-request KMS/HSM calls, one immutable environment key, database-held key history | Keeps quote/replay latency bounded while supporting rotation, revocation, and an auditable KMS/HSM adapter; see ADR-0109 |

### Indexer replay telemetry

| Option | Advantages | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Per-stream metric labels | Easy dashboard filtering | Unbounded cardinality from configuration data | Rejected |
| Independent database exporter | Separates telemetry from the service | Adds lag and another operational boundary | Rejected |
| Log-only backlog reporting | No metric implementation | Cannot provide stable alert expressions | Rejected |
| **Fixed-cardinality gauges from the indexer** | Scoped, bounded, alertable, and operationally close to replay state | Requires one bounded database snapshot query | **Selected; see ADR-0123** |

### Targeted finalized signature recovery

| Alternative | Benefits | Risks / costs | Decision |
| --- | --- | --- | --- |
| Forward-only finalized `getBlock` scan | Strong checkpoint semantics and complete block evidence | Expensive for narrow investigations and ambiguous signatures | Retained as authoritative projection path |
| Provider-specific operator queries | Fast to try manually | Unbounded, inconsistent commitment/response semantics, not reproducible | Rejected |
| Unbounded `getSignaturesForAddress` loop | Finds all history | Unbounded provider work and cancellation/retry risk | Rejected |
| **Bounded typed finalized signature pages with separate evidence fetch** | Fixed work budget, canonical cursor, provider-independent contract, supports targeted recovery | Needs a separate authorized worker to fetch and verify full finalized evidence | **Selected in ADR-0124** |

### Targeted signature-to-evidence recovery

| Alternative | Benefits | Risks / costs | Decision |
| --- | --- | --- | --- |
| Rebuild from retained raw finalized evidence | No provider dependency; strongest replay determinism | Cannot recover evidence that was never retained | Retained as the preferred rebuild path |
| Poll `getTransaction` without address discovery | Fewer calls for a known signature | Does not prove address scope and encourages unbounded/manual provider use | Rejected |
| Persist signature-list entries as projection evidence | Very low latency | A signature list contains no message, event bytes, or amounts | Rejected |
| **Bounded address signature discovery followed by finalized `getTransaction` and projection-only persistence** | Binds the target to finalized evidence, preserves checkpoint authority, supports idempotent repair | Requires an authorized control-plane wrapper and live provider evidence | **Selected in ADR-0125** |

### Targeted-recovery audit reconciliation telemetry

| Alternative | Benefits | Risks / costs | Decision |
| --- | --- | --- | --- |
| Query the audit table on every API metrics scrape | Direct visibility from the writer | Adds a database round trip to the API metrics endpoint and couples telemetry to request serving | Rejected |
| Per-request/address/signature metric labels | Easy target lookup | Unbounded sensitive cardinality and memory/alerting hazards | Rejected |
| Log-only stale requested rows | Minimal implementation | No stable alert expression or bounded dashboard signal | Rejected |
| **Fixed-cardinality stale-request aggregate refreshed by the indexer service** | Bounded, alertable, no hot-path mutation, reuses operational refresh loop | Requires one indexed aggregate query and reconciliation runbook | **Selected; see ADR-0126** |

### Finalized vault-balance reconciliation

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Cumulative receipt tax only | Simple projection query | Ignores governed vault outflows | Rejected |
| PostgreSQL-only expected/observed comparison | Cheap and deterministic | PostgreSQL is not the Solana funds authority | Rejected |
| On-chain cumulative vault counter | Strong on-chain evidence | Adds writable hot state and contention to settlement | Rejected |
| `getTokenAccountBalance` without account proof | Small RPC response | Does not prove owner, mint, token program, or extension compatibility | Rejected |
| **Explicit opening balance plus finalized receipt/release delta, checked against a finalized verified token account** | Auditable initial condition, exact u64 semantics, preserves Solana authority, bounded off-hot-path work | Requires opening-balance ceremony and a separate Token-2022 extension ADR | **Selected in ADR-0127** |

### Vault-reconciliation operator read model

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Browser target-creation and opening-balance form | Convenient onboarding | Browser cannot prove finalized chain evidence; creates an unsafe authority boundary | Rejected |
| Global system-wide dashboard at the merchant API boundary | Centralized visibility | Cross-tenant disclosure and broader operator authority | Rejected |
| Logs only | Minimal implementation | No bounded authenticated evidence or usable recovery state | Rejected |
| **Tenant-scoped authenticated read model with server-owned immutable values** | Protects tenant boundary, supports explicit UI failure states, no settlement mutation, bounded indexed queries | System-wide finance view remains a separate governed surface | **Selected in ADR-0128** |

### Release artifact integrity

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Manual release notes and ad-hoc binaries | Low setup cost | Not reproducible; weak source-to-artifact binding | Rejected |
| Tag-triggered deployment or image publication | Fast path to operations | Adds unapproved deployment authority and can bypass license/readiness gates | Rejected |
| Floating actions, images, or dependency updates | Low maintenance effort | Non-reproducible and unsafe supply-chain boundary | Rejected |
| **Tag/manual evidence bundle with checksums, SBOM, and validator evidence** | Binds tested source to exact artifacts without deployment authority | Requires a successful retained workflow run and independent review | **Selected in ADR-0129** |

### Deployment environment safety

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Blank environment with permissive defaults | Convenient local startup | Ambiguous security boundary and unsafe mode combinations | Rejected |
| Separate binaries per environment | Strong visual separation | Duplicated startup logic and release drift | Rejected |
| Deployment-tool-only validation | Centralized policy | Direct process starts can bypass it | Rejected |
| **Explicit runtime environment contract with checked-in local defaults** | Fail-closed startup, deterministic tests, one immutable binary | Production provisioning and launch approval remain separate controls | **Selected in ADR-0130** |

### Worker deployment environment safety

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Check only whether insecure RPC is enabled | Small change | Secure-RPC workers can still start with an ambiguous environment identity | Rejected |
| Infer environment from RPC endpoint names | No additional setting | Endpoint names are not authority, and proxies/failover make inference unreliable | Rejected |
| Separate worker binaries per environment | Visible separation | Duplicated startup logic and release drift | Rejected |
| **Required `*_ENV` contract in each worker with explicit Compose local defaults** | Fail-closed identity, deterministic tests, one immutable binary | Deployment provisioning still needs independent approval and secrets | **Selected; see ADR-0132** |

### Multi-replica quote idempotency evidence

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Single API process under high concurrency | Simple and fast to run | Cannot expose process-local state or replica routing errors | Rejected as the only evidence |
| In-process or Redis lock | May reduce duplicate work | Adds a non-authoritative lock boundary and can diverge from PostgreSQL | Rejected |
| Sticky load-balancer sessions | Easy routing model | Masks failover and retry-to-another-replica behavior | Rejected |
| **Two independent API containers fenced by PostgreSQL** | Reproducible, bounded, and validates the actual durable authority | Local evidence does not prove regional production capacity | **Selected; see ADR-0133** |

### Multi-replica settlement submission-fence evidence

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Submit both requests to one API process | Fast and simple | Does not exercise replica routing or process-local state separation | Rejected as the only evidence |
| Sticky routing or process-local single-flight | May reduce duplicate work | Masks failover and cannot be the durable money-moving authority | Rejected |
| Rely on RPC/Solana duplicate-signature handling | No application change | Allows duplicate provider work and weakens operational guarantees | Rejected |
| **Two API processes with PostgreSQL compare-and-set fencing** | Directly validates the durable submission authority with bounded Docker evidence | Does not prove regional provider failover or production capacity | **Selected; see ADR-0134** |

### SLO, incident, and dependency controls

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Infrastructure availability metrics only | Easy to collect and compare | Hides quote correctness, finality, reconciliation, and financial-integrity failures | Rejected |
| Vendor status page as the operating contract | Fast to adopt | Provider lock-in, incomplete user-journey semantics, and weak public reproducibility | Rejected |
| Team-local undocumented targets | Flexible during early development | No consistent ownership, release gate, RTO/RPO, or incident severity | Rejected |
| **Versioned user-journey SLO policy plus dependency register and public incident runbook** | Binds latency, correctness, recovery, ownership, and exit paths while keeping provider adapters replaceable | Requires deployment-specific owner/provider binding and retained evidence | **Selected; see ADR-0135** |

### Scheduled vault reconciliation evidence

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| PostgreSQL row seeded directly by the drill | Fast read-model and authorization assertion | Does not exercise the scheduler, RPC serialization, account owner/mint/layout validation, or expectation calculation | Rejected as the only evidence |
| Unit-only mock of the finalized balance port | Deterministic and fast | Cannot detect Compose wiring, process startup, JSON-RPC, or persistence regressions | Retained as a complement, not release evidence |
| Full validator for every reconciliation test | Strong ledger realism | Slow and environment-dependent; validator remains a separate Linux conformance gate | Rejected as the only routine drill |
| **Bounded synthetic `getMultipleAccounts` fixture plus the real scheduled indexer process** | Exercises the deployed service boundary deterministically and supports matched/mismatch evidence | Synthetic account semantics do not replace live validator or provider-failover tests | **Selected; see ADR-0136** |

### Multi-provider RPC failover evidence

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Unit tests only | Fast and deterministic | Cannot prove endpoint wiring, Docker networking, or service transport behavior | Rejected as the only evidence |
| Unbounded retries against one provider | Simple implementation | Violates deadline, backpressure, and overload controls | Rejected |
| Random provider selection per request | Spreads traffic | Nondeterministic evidence and weak adaptive health semantics | Rejected |
| Public/testnet outage injection | More realistic network | Uncontrolled, non-repeatable, and unsuitable for CI traffic | Rejected |
| **Two isolated fixtures with deterministic first-provider method failure** | Process-level, bounded, repeatable failover evidence | Synthetic failure does not prove live provider SLOs or regional capacity | **Selected; see ADR-0137** |

### Database migration execution

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| PostgreSQL init-directory ordering only | Simple clean bootstrap | No durable applied ledger, lock, or historical checksum verification | Rejected for staging/production |
| Re-run all idempotent SQL files | Minimal tooling | Idempotence does not prove compatibility or detect source drift | Rejected |
| Provider-specific migration service | Mature operational integrations | Adds vendor dependency and weakens clean-checkout reproducibility | Rejected as repository authority |
| **Rust runner with advisory lock, ordered ledger, and SHA-256 checksums** | Deterministic, typed, fail-closed, low-latency deployment boundary | Requires staged Compose/deployment integration | **Selected; see ADR-0138** |

### Migration database-connection lifecycle

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Detached `tokio::spawn` connection driver | Simple and common in small binaries | Connection failures can arrive after a false-success migration result; task lifetime is detached | Rejected |
| `tokio::try_join!` migration and connection futures | Propagates errors | A healthy PostgreSQL driver normally does not finish before the client closes, so successful migration waits indefinitely | Rejected |
| Post-migration connection health check | Easy to add | Cannot prove that a failure did not occur during migration | Rejected |
| **Owned `tokio::select!` migration/connection supervisor** | Bounded lifetime, prompt failure propagation, cancellation, and no detached task | Requires explicit clean-termination semantics | **Selected; see ADR-0221** |

### Service shutdown-signal handling

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Convert `ctrl_c()` errors to `None` | Keeps the loop simple | Masks failed shutdown control and can leave a process running indefinitely | Rejected |
| Retry signal registration forever | May recover from transient setup errors | Unbounded retry and no deterministic lifecycle result | Rejected |
| Detached signal-listener task | Separates signal code | Loses ownership and complicates cancellation/error propagation | Rejected |
| **Handle `ctrl_c()` success/error in the existing structured service loop** | Fail-closed, bounded, minimal, and preserves orderly shutdown | Requires explicit error branch in each entrypoint | **Selected; see ADR-0222** |

### Non-authoritative metric-observation failures

Metric export is operational evidence, not settlement authority. Unexpected
rejection by the fixed operation registry must therefore be visible without
allowing telemetry to fail a quote, settlement, reconciliation, or worker
operation.

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Discard `observe` errors | Zero request-path overhead | Silently loses evidence and hides registry drift | Rejected |
| Propagate errors through financial paths | Makes failures visible to callers | Incorrectly makes non-authoritative telemetry a money-path dependency | Rejected |
| Queue metric errors asynchronously | Decouples the caller | Adds unbounded/lossy queue and lifecycle complexity for a bounded validation error | Rejected |
| **Synchronous bounded structured log via `observe_or_log`** | Visible, bounded, no retry or external dependency, preserves financial-path availability | Requires a fixed log contract and log-volume monitoring | **Selected; see ADR-0223** |

All PostgreSQL-backed operational drills use the same migration authority.
Each clean-stack drill waits for PostgreSQL health and runs the pinned
one-shot migration service before touching application tables; see
[ADR-0160](../adr/0160-clean-stack-drill-migration-gate.md).

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Duplicate schema setup in each drill | Direct and locally fast | Schema drift and false evidence when migrations break | Rejected |
| Reuse a pre-migrated shared volume | Fast repeat runs | Order-dependent and not clean-checkout reproducible | Rejected |
| **Run the authoritative migrator after PostgreSQL health** | Clean-stack reproducibility, checksum/ordering coverage, minimal topology | Adds bounded setup time | **Selected; ADR-0160** |

### v0 transaction and Address Lookup Table admission

The verifier must resolve dynamic accounts from chain state before accepting a
wallet-signed v0 settlement; see [ADR-0161](../adr/0161-governed-v0-alt-settlement-path.md).

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Reject v0+ALT indefinitely | Smallest attack surface | Leaves a required production transaction path unavailable | Rejected |
| Trust client-provided table contents or digest | No RPC read and low latency | Client-controlled authority boundary and account substitution risk | Rejected |
| Accept any provider-returned table without bounds | Broad compatibility | Unbounded reads, provider disagreement, and extra-account risk | Rejected |
| **Server-owned bounded finalized resolution with exact expanded-domain verification** | Preserves v0 capability while retaining account, privilege, size, and failover controls | Adds bounded RPC reads before submission | **Selected; ADR-0161** |

## Non-local deployment configuration

| Decision | Selected | Alternatives considered | Selection reason |
| --- | --- | --- | --- |
| Staging/production configuration boundary | Shared repository preflight plus service-owned runtime validation (ADR-0139) | Convention-only environment variables, deployment-tool-only validation, separate binaries per environment | Catches cross-service local-default leakage before rollout while retaining independent fail-closed checks in every service and one immutable release artifact |

The preflight requires non-local TLS-verified databases, HTTPS RPC endpoints,
identity and evidence keys, jurisdiction-provider inputs, and no synthetic seed
directory. It does not replace secret validity checks, HSM/KMS ceremonies,
deployment approvals, provider health, or live staging evidence.

### Tax Engine deployment identity preflight

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Operator convention only | No additional manifest fields | Local token or omitted service label can reach non-local rollout | Rejected |
| Tax Engine-only checker | Narrow ownership | Duplicates shared deployment policy and can drift | Rejected |
| **Shared preflight plus Tax Engine runtime validation** | Cross-service parity, early local-default rejection, independent process safety | Secret validity, mTLS, and rotation remain deployment-owned | **Selected in ADR-0226** |

### Tax Engine service transport

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Plain HTTP plus private network policy | Lowest handshake overhead | No cryptographic workload identity or confidentiality at the service boundary | Rejected |
| NGINX/sidecar-only mTLS termination | Centralized certificate operations | Extra hop and bypassable proxy authority on the internal calculation path | Rejected as the default |
| Bearer token over externally terminated TLS | Simple application integration | Does not protect a bypassed/internal plaintext hop from token replay | Rejected as the sole control |
| **Direct Rustls mTLS in the Tax Engine process** | Explicit client CA verification, no non-local plaintext fallback, bounded path | Certificate provisioning and rotation remain deployment work | **Selected in ADR-0227** |

Rustls provider selection is explicit: automatic feature-based selection is
rejected because the workspace contains both AWS-LC and ring feature paths;
AWS-LC is installed process-wide before the Tax Engine constructs TLS configs.
Changing that provider requires a compatibility and compliance ADR.

### Tax Engine mTLS certificate rotation

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Restart the process for every rotation | Simple lifecycle | Connection churn and a larger emergency blast radius | Rejected as the only mechanism |
| Poll or watch certificate files | Automatic detection | Partial-write races, platform-specific semantics, hidden operator action | Rejected |
| Sidecar-only reload and proxy termination | Centralized certificate operations | Extra hop and a second internal transport authority | Rejected as the default |
| **Explicit SIGHUP with validated atomic Rustls snapshot replacement** | Bounded, auditable, no extra hop, retains last known-good config | Requires Unix signal and atomic secret-delivery ceremony | **Selected in ADR-0228** |

The reload path validates certificate, key, client CA, verifier, and ALPN in a
blocking task before swapping the live snapshot. Invalid replacements are
observable but cannot displace a valid configuration.

### Tax Engine mTLS client identity

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Bearer token only | Simple application integration | Shared-secret replay and workload confusion | Rejected |
| CA-chain validation only | Minimal handshake configuration | Trusts every workload issued by the CA | Rejected |
| Subject/CN allow-list | Easy to inspect in certificates | CN is not a reliable workload identity and subject conventions drift | Rejected |
| **Exact URI-SAN allow-list in the Rustls verifier** | Handshake-bound identity, no request-path I/O, bounded policy | Identity-set changes require a deployment reload/restart | **Selected in ADR-0236** |
| External identity introspection per request | Centralized policy | Adds latency, availability, and a new authority to the hot path | Rejected |

The selected verifier delegates chain, expiry, and client-auth validation to
the standard WebPki verifier and then requires exactly one `spiffe://` URI SAN
to match an immutable, bounded deployment allow-list. Raw certificate
identities remain excluded from logs and metric labels.

### Tax Engine service-token rotation

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Process restart for every rotation | Simple lifecycle | Connection churn and larger emergency blast radius | Rejected as the only mechanism |
| Secret-manager lookup per request | Centralized source | Adds hot-path latency and availability coupling | Rejected |
| Mutable mutex-protected token | Straightforward implementation | Request-path contention and less explicit snapshot semantics | Rejected |
| **Bounded file source with atomic SIGHUP replacement** | Lock-free request path, auditable rotation, last-known-good retention | Requires atomic deployment file delivery and Unix signal control | **Selected in ADR-0237** |

### Governed refund-claim cancellation workflow

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| PostgreSQL-only cancellation flag | Fast and simple | Leaves the on-chain claim executable; database becomes a false authority | Rejected |
| Browser-constructed cancellation instruction | Fewer server compiler types | Allows PDA, nonce, registry, and authority substitution | Rejected |
| Reuse refund claim/execute compiler | Less code | Carries the wrong account shape and risks financial/topology confusion | Rejected |
| API-held governance signer | Operationally convenient | Custody, larger key blast radius, and breaks wallet authorization | Rejected |
| **Server-owned immutable cancellation artifact plus governance wallet signature and finalized proof** | Exact authority boundary, non-custodial, replay-safe, auditable | Adds a cancellation intent and reconciliation lifecycle | **Selected in ADR-0238** |

The selected design has a fixed five-account legacy instruction and no RPC
lookup in the compiler or wallet handoff path. Cancellation contains no
amounts or token accounts; the server derives the registry and claim PDA from
the authenticated claim identity. Submission is only an intent state change,
and finalized on-chain event evidence remains the only cancellation authority
for the off-chain projection.

### Tax Engine observability identity

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Accept both `tax-engine` and `tax_engine` | Transitional compatibility | Splits fixed-cardinality dashboards, alerts, and evidence queries | Rejected |
| Accept arbitrary service names | Flexible reuse | Allows unreviewed telemetry identities and cardinality drift | Rejected |
| **Use one explicit `tax_engine` identity** | Matches existing metrics and preserves the allow-list boundary | Requires one canonical spelling in operations | **Selected in ADR-0229** |

### Tax Engine container runtime verification

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Unit tests only | Fast and deterministic | Misses image entrypoint, environment, and listener drift | Rejected as the sole gate |
| Full end-to-end stack only | Broad system coverage | Slow and poor failure localization for the Tax Engine boundary | Rejected as the only service contract |
| Host-process smoke test | Simple to run | Does not verify the Docker production boundary | Rejected |
| **Isolated pinned Docker runtime contract** | Exercises the real image with bounded focused assertions and cleanup | Requires Docker in CI/local verification | **Selected in ADR-0230** |

### Tax Engine request-rejection semantics

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Preserve Axum's default `422` response | No handler code | Diverges from the repository API contract | Rejected |
| Map every JSON rejection to `400` | Simple and predictable | Masks body-limit admission control | Rejected |
| Broad global rejection middleware | Centralized | Couples unrelated routes and obscures rejection provenance | Rejected |
| **Explicit typed-boundary mapping with body-limit `413`** | Preserves client and operational semantics with bounded error detail | Small explicit mapping function | **Selected in ADR-0231** |

### Warm Tax Engine service concurrency evidence

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Pure Criterion benchmark only | Fast and stable | Omits HTTP, container, authentication, scheduler, and serialization costs | Rejected as the sole service gate |
| Full API quote benchmark | Exercises durable storage and tenant paths | Does not isolate the warm Tax Engine adapter | Rejected for this boundary |
| Unbounded external load generator | Broad traffic shapes | Non-reproducible, resource-heavy, and difficult to keep synthetic | Rejected |
| **Bounded local Docker concurrency contract** | Measures the real image with explicit workload and cleanup | Local-only evidence and Docker dependency | **Selected in ADR-0232** |

### Warm Tax Engine cache-miss coalescing

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Compile under the LRU mutex | Simple and naturally coalesces misses | Blocks unrelated hits and distinct-rule compiles behind compilation | Rejected |
| Allow duplicate compilation | Minimal implementation | CPU stampede and avoidable p95/p99 inflation during rule bursts | Rejected |
| Distributed Redis/lock service | Cross-replica coalescing | Adds network failure, latency, and a non-authoritative dependency | Rejected |
| **Bounded process-local single-flight keyed by digest** | Coalesces identical misses without blocking unrelated cache work | Coalescing is per replica and cleanup must remain bounded | **Selected in ADR-0381** |

### Real-container Tax Engine mTLS

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Rustls unit tests only | Fast and deterministic | Misses image paths, process startup, listener, and signal delivery | Rejected as the sole gate |
| Plaintext runtime contract only | Simple and portable | Cannot detect non-local plaintext fallback or missing client authentication | Rejected |
| Sidecar/proxy-only mTLS test | Matches one deployment topology | Leaves direct Rustls configuration and image behavior unverified | Rejected as the service contract |
| **Ephemeral CA plus real pinned-container mTLS probe** | Exercises deployed binary, certificate files, client verification, ALPN, and reload | Requires Docker and host OpenSSL | **Selected in ADR-0233** |

### JWKS deployment representation

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Require the legacy single-key pair | Simple shell validation | Rejects provider-managed rotation already supported by API startup | Rejected |
| Accept both structured JWKS and the pair simultaneously | Permissive migration | Creates ambiguous identity authority and rotation drift | Rejected |
| Remove identity-key validation from preflight | Fewer deployment failures | Allows missing or local identity material to reach rollout | Rejected |
| **Accept exactly one supported representation with structural/local-value checks** | Keeps deployment parity, fail-closed mutual exclusion, and secret-safe validation | Cryptographic key validity remains a runtime/provider responsibility | **Selected in ADR-0219** |

### Jurisdiction-provider key deployment representation

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Require the legacy provider-key pair | Simple shell validation | Rejects the structured registry already supported by API startup | Rejected |
| Accept structured registry and pair simultaneously | Permissive migration | Creates ambiguous provider authority and rotation drift | Rejected |
| Remove provider-key validation from preflight | Fewer deployment failures | Allows missing or local jurisdiction evidence verification material to reach rollout | Rejected |
| **Accept exactly one supported representation with structural/local-value checks** | Keeps preflight/runtime parity and fail-closed secret-safe validation | Cryptographic validity and provider availability remain runtime/deployment responsibilities | **Selected in ADR-0220** |

### Validator-backed Split Plan conformance

| Decision | Selected | Alternatives considered | Selection reason |
| --- | --- | --- | --- |
| Runtime proof for bounded recipient fan-out | Extend the pinned external Agave validator suite (ADR-0140) | Allocator unit tests only, `solana-program-test` only, synthetic RPC fixture | Proves the deployed SBF artifact, exact remaining-account order, token CPI movement, finalized conservation, and duplicate-receipt behavior at the real program boundary |

ADR-0214 extends that selected validator boundary with an explicit
extension-free Token-2022 two-recipient case. This preserves the one
`token_interface` implementation while preventing a single-recipient test from
being mistaken for proof of multi-recipient Token-2022 compatibility.

### Validator-backed policy revocation

| Decision | Selected | Alternatives considered | Selection reason |
| --- | --- | --- | --- |
| Runtime proof that revoked policy cannot move tokens | Execute revoke then settlement against the pinned external Agave validator (ADR-0141) | Unit-test revocation helpers only, synthetic RPC fixture | Proves authority binding, revoked-PDA enforcement, deployed SBF behavior, and unchanged token balances at the CPI boundary |

### Bounded load-test resource profiling

| Decision | Selected | Alternatives considered | Selection reason |
| --- | --- | --- | --- |
| Resource evidence alongside quote latency | Bounded Docker JSON stats for the named benchmark containers (ADR-0215) | Latency only, arbitrary host-process sampling, an unpinned monitoring stack | Provides reproducible CPU/memory/runtime evidence without leaking unrelated host data or adding an unbounded dependency to the benchmark |

### Server-owned settlement-route mint precision

| Decision | Selected | Alternatives considered | Selection reason |
| --- | --- | --- | --- |
| Asset precision at the checkout boundary | Persist finalized verifier-derived `mint_decimals` on the immutable route version (ADR-0216) | Assume two decimals, accept browser/caller precision, fetch mint metadata during checkout, parse untyped evidence on every read | Keeps precision evidence-bound and O(1) on the hot path, supports Solana assets with 0–9 decimals, and prevents browser-side amount misrepresentation |

### SPA workspace focus management

| Decision | Selected | Alternatives considered | Selection reason |
| --- | --- | --- | --- |
| Keyboard/screen-reader context after workspace changes | Skip link plus stable focus boundary (ADR-0142) | Leave focus on navigation control, focus first form control, add a full router | Deterministic across lazy/error/empty states with minimal client lifecycle cost and no impact on financial authority |

### Structured Tax Engine rule workbench

| Decision | Selected | Alternatives considered | Selection reason |
| --- | --- | --- | --- |
| Rule-governance authoring surface | Schema-driven structured workbench plus advanced canonical JSON mode (ADR-0143) | Raw JSON only, jurisdiction-specific forms, graph editor only | Makes every supported DSL semantic reviewable and accessible while preserving exact canonical interchange and server authority |

### Persisted-rule simulation boundary

| Alternative | Strengths | Risks/trade-offs | Decision |
| --- | --- | --- | --- |
| Caller-supplied isolated calculation preview | Simple and useful for engine development | Does not prove the document being simulated is the governed tenant-owned draft | Retained only for simulation-mode development; rejected for governance UI |
| Browser-side calculation against a returned rule document | Immediate interaction | Duplicated tax semantics, numeric drift, and a client authority surface | Rejected |
| Persist every simulation as financial evidence | Maximum historical volume | Hot-path writes and audit noise; still needs rule binding | Rejected |
| **Server-owned simulation of persisted rule with digest/identity checks and warm cache (ADR-0144)** | Binds the exact governed artifact while preserving bounded low-latency execution | Ephemeral result is not a quote or attestation | **Selected** |

### Settlement adjustments

| Decision | Selected approach | Alternatives considered | Why selected |
| --- | --- | --- | --- |
| Original receipt binding | Tenant-scoped finalized settlement-intent join to the canonical receipt projection | Caller-supplied receipt; mutable adjustment copy; signature-only join | Preserves server ownership, finalized-slot binding, asset binding, duplicate-stream tolerance, and fail-closed conflict handling |

| Adjustment authority binding | Wallet-proof-backed subject-to-Solana-key binding plus durable compiler intent | OIDC subject as key; caller-supplied keys; server-held signer | Provides cryptographic actor binding, non-custodial signing, rotation evidence, and deterministic retry convergence |

### Canonical settlement-receipt identity projection

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Caller-supplied original receipt and chain accounts | Small request shape | Breaks server-owned authority and permits account substitution | Rejected |
| Mutable API UUID-to-receipt mapping | Simple lookup | Adds a second authority disconnected from finalized raw evidence | Rejected |
| Split-settlement projection for all payments | Existing rich identity fields | Does not cover ordinary settlement receipts | Rejected |
| **Enrich `IndexedReceipt` from finalized Anchor event bytes** | Replayable, immutable, shared by reconciliation and adjustments | Requires additive migration and backfill evidence | **Selected in ADR-0164** |

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Mutate the original settlement receipt | Minimal tables and simple customer display | Destroys historical evidence, breaks replay and reconciliation, and obscures who changed money | Rejected |
| Treat refunds as an untyped negative settlement | Reuses payment code | Loses adjustment reason/evidence, weakens bounds, and can bypass approval and duplicate fencing | Rejected |
| Put refund/correction arithmetic on-chain | Strong ledger authority | Tax semantics, chargeback evidence, and unbounded business workflow do not fit a bounded deterministic program | Rejected |
| **Immutable signed-delta adjustment domain with separate governed posting** | Exact conservation, replayable evidence, bounded arithmetic, and clear separation from token authority | Requires durable workflow, read models, and a later posting contract | **Selected in ADR-0150** |

### Durable adjustment workflow

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Mutate the original receipt row | Few reads and no new projection | Breaks immutable audit/replay semantics and hides the adjustment actor | Rejected |
| Browser/local storage workflow | Fast interaction and no API schema | Not durable, not tenant-authoritative, and unsafe after reload or retries | Rejected |
| Redis-only proposal state | Low latency and simple TTLs | Disposable cache cannot be the financial evidence authority | Rejected |
| **PostgreSQL append-only adjustment record plus transactional outbox** | Durable UUIDv7 identity, tenant fencing, atomic evidence/publication, and existing operational controls | Requires migration and bounded governance endpoints | **Selected in ADR-0151** |

### Settlement-adjustment operator workspace

**Criteria:** preserve the settlement-adjustment authority boundary, make
four-eyes evidence operable, support real loading/error/retry states, avoid
financial arithmetic in JavaScript, and fit the existing accessible SPA.

| Option | Strengths | Risks | Decision |
| --- | --- | --- | --- |
| **Dedicated settlement-adjustment workspace** | Clear bounded-context ownership, explicit evidence review, no implied chain posting | Adds one lazy-loaded workspace and API contract | **Selected in ADR-0152** |
| Embed into vault-release control | Reuses treasury UI and lifecycle patterns | Conflates adjustment evidence with token release authority and makes refund semantics ambiguous | Rejected |
| Extend rule-governance workspace | Reuses author/reviewer patterns | Tax-rule publication and financial corrections have different invariants, scopes, and audit meaning | Rejected |
| Generic admin table | Fast list/read surface | Encourages unsafe inline mutation and cannot express exact deltas, evidence, or four-eyes semantics well | Rejected |

The selected workspace is a projection and command client only. PostgreSQL and
the authenticated API remain authoritative; browser state is disposable.

### Adjustment UI validation boundary

**Criteria:** preserve exact financial values, provide immediate feedback, and
avoid introducing a second financial implementation.

| Option | Strengths | Risks | Decision |
| --- | --- | --- | --- |
| **String validation plus conservation check, API revalidation** | Safe browser UX, no floating-point conversion, server remains authority | Duplicates a small amount of validation code | **Selected in ADR-0152** |
| JavaScript `number` arithmetic | Simple controls | Loses precision for Solana atomic units and violates financial contract | Rejected |
| Browser-only validation | Minimal API work | Allows stale or malicious callers to bypass invariants | Rejected |
| Client tax/adjustment engine | Rich immediate calculation | Creates a second authority and can diverge from Rust semantics | Rejected |

### On-chain adjustment boundary

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Mutate the original settlement receipt or reuse `settle` | Few accounts and familiar code path | Destroys original evidence, conflates checkout with correction, and weakens replay/reconciliation | Rejected |
| Transfer directly from the tax vault or original payer source | Appears to complete refunds quickly | No universal lawful funder/beneficiary; unsafe across tax, chargeback, and remittance semantics | Rejected |
| Off-chain `posted` state only | Simple API and database implementation | No independently verifiable finalized chain evidence and easy to overstate payment | Rejected |
| **Separate bounded `record_adjustment` PDA with active merchant/mint authority registry and no token CPI** | Immutable chain evidence, deterministic replay identity, fixed account/compute shape, and fail-closed authority boundary | Does not itself pay a refund; requires registry governance and a later movement contract | **Selected in ADR-0153** |

### Finalized adjustment-registry verification

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Trust the database route and wallet bindings | No extra RPC call | Does not prove the active on-chain authority or program ownership | Rejected |
| Accept a registry account from the browser | Flexible integration | Allows PDA/account substitution | Rejected |
| Read at processed or confirmed commitment | Lower latency | Can admit stale or reorged authority state | Rejected |
| Read once and cache indefinitely | Low steady-state RPC cost | Hides registry rotation and makes cache an authority | Rejected |
| **Derive the PDA, read finalized account data, require provider agreement, and snapshot evidence in the intent** | Chain-bound, replayable, provider-aware, and fail closed | Adds bounded RPC admission and evidence columns | **Selected in ADR-0167** |

### Adjustment submission and finalized recording

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Mutate the immutable evidence intent with transport status | Fewer tables | Blends immutable compiler evidence with retryable transport state | Rejected |
| Reuse generic settlement intents | Existing lifecycle and routes | Settlement-specific message verification cannot prove adjustment accounts or event identity | Rejected |
| Trust client digest/signature or RPC acknowledgement | Lowest implementation cost | Does not prove message binding, finality, or emitted adjustment evidence | Rejected |
| **Separate wallet-bound chain-intent lifecycle plus typed finalized event projection** | Immutable audit boundary, bounded retries, exact message proof, and independent finalized evidence | Adds migration, decoder, and reconciliation work | **Selected in ADR-0170** |

### Adjustment wallet workspace

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Duplicate wallet/signing code in the adjustment panel | Fast local implementation | Splits security review, serialization, timeout, and rejection behavior | Rejected |
| Browser constructs or mutates `record_adjustment` bytes | Flexible UI | Creates an account/data substitution authority and numeric drift surface | Rejected |
| Persist signed bytes in browser storage | Convenient retry after reload | Increases sensitive material exposure and can cross operator/session boundaries | Rejected |
| **Reuse the shared injected-wallet boundary with server-owned compile, exact submission, and explicit reconciliation** | One wallet trust boundary, immutable server authority, bounded UX states | Requires a typed adjustment artifact parser and wallet mismatch checks | **Selected in ADR-0171** |

### Solana `u64` browser wire representation

| Alternative | Decision | Trade-off |
| --- | --- | --- |
| JSON numbers with browser safe-integer rejection | Rejected | Valid Solana counters above `2^53 - 1` cannot be represented. |
| Hex/base64 protocol counters | Rejected | Exact but inconsistent with canonical decimal financial values and less interoperable. |
| JavaScript `BigInt` as the universal client representation | Rejected | Does not solve JSON transport and complicates wallet/UI boundaries. |
| **Canonical decimal strings at API/browser boundaries; Rust `u64` internally** | **Selected** | Exact, deterministic, cross-language, and compatible with existing financial string semantics. |

### Governed adjustment refund movement

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Reuse `execute_vault_release` | Existing claim and vault CPI path | Cannot atomically bind the merchant-funded net leg; over-privileges the tax vault | Rejected |
| Transfer full gross from the tax vault | One source and one CPI | Tax vault is not a universal refund funder and may not contain net funds | Rejected |
| Transfer from the original payer source | Appears to reverse the checkout | Original token account is not stored in the immutable receipt and payer is not necessarily the funder | Rejected |
| Browser/API direct transfer | Fast to expose | Bypasses on-chain account binding, atomicity, replay fencing, and non-custodial authority | Rejected |
| **Immutable refund claim with tax-vault and merchant-funder sources, one payer-bound destination, and two atomic checked CPIs** | Exact authority topology, conservation, replay protection, and explicit first-scope semantics | Adds claim lifecycle and requires separate contracts for other adjustment kinds | **Selected in ADR-0154** |

### Refund-claim wallet orchestration

**Criteria:** preserve non-custodial wallet authority, prevent account
substitution, retain four-eyes governance, make retries idempotent, and prove
finalized token movement independently from event indexing.

| Option | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Browser-supplied refund topology | Small API surface | Client can redirect source or beneficiary; cannot establish authority | Rejected |
| Immediate claim-and-execute flow | Fewer screens and tables | Removes approval separation and makes ambiguous submission unsafe | Rejected |
| Generic settlement-intent reuse | Existing submission machinery | Does not bind claim PDA, two refund signers, or movement proof | Rejected |
| **Separate server-owned claim and execute intents with shared wallet adapter, finalized typed evidence, and transaction-bound proof** | Strong authority separation, replay fencing, and explicit recovery | More durable state and operator workflow | **Selected in ADR-0173** |

### Refund funding configuration

**Criteria:** make the source and authority topology explicit, preserve tenant
isolation and four-eyes governance, support versioned rotation, and prevent an
off-chain row from being mistaken for chain ownership evidence.

| Option | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Caller-supplied funding accounts per refund | No setup workflow | Account substitution and beneficiary ambiguity; no durable rotation evidence | Rejected |
| Reuse the generic settlement route | Existing schema | A refund funder and payer destination have different authority semantics; conflates checkout and refund funding | Rejected |
| Mutable one-row merchant funding settings | Simple reads | Breaks replay of an existing claim when accounts rotate | Rejected |
| **Versioned tenant/mint funding configuration with append-only governance and separate finalized topology verification** | Stable claim binding, four-eyes control, auditable rotation, and explicit chain authority boundary | Adds a control-plane lifecycle and verification work | **Selected in ADR-0173** |

### Refund-funding topology activation

**Criteria:** prove the complete authority topology at one finalized slot,
prevent browser- or database-controlled activation, preserve bounded provider
failover, and make verification evidence replayable.

| Option | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Trust approved database account strings | No activation RPC | Does not prove chain ownership, token layout, PDA, or finality | Rejected |
| Reuse generic settlement-route verification unchanged | Existing implementation | Omits refund-funder wallet/source authority semantics | Rejected |
| Independent account reads | Small individual responses | Can mix slots and create a topology that never existed atomically | Rejected |
| **Complete finalized account-set read with provider agreement and immutable digest** | Exact authority checks, one-slot evidence, bounded failover, and replayability | Adds one provisioning operation and starts legacy-SPL-only | **Selected in ADR-0174** |

### Finalized refund execution evidence

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Keep refund execution as opaque raw bytes | Minimal schema and decoder work | Every consumer reparses event data and can disagree on account/amount binding | Rejected |
| Trust API-submitted signature and amounts | Low implementation cost | Caller-controlled evidence can claim movement without finalized chain proof | Rejected |
| Infer execution from balances only | Direct financial observation | A balance delta cannot prove the exact claim, event, beneficiary, or approver | Rejected |
| **Typed finalized event projection bound to immutable raw evidence, followed by independent balance reconciliation** | Deterministic replay, exact event binding, auditable evidence, and fail-closed conflict handling | Adds a bounded projection and a later reconciliation step | **Selected in ADR-0155** |

### Finalized refund balance proof

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Trust the refund execution event | Lowest latency and no extra account reads | Does not prove committed three-account deltas | Rejected |
| Read only the payer destination | Confirms an inflow | Cannot prove tax-vault and merchant-funder outflows or source identity | Rejected |
| Read the three accounts independently | Reuses simple account readers | Mixed finalized slots can produce a false composite | Rejected |
| **One finalized multi-account snapshot plus exact checked-u64 delta proof** | Atomic observation boundary, deterministic arithmetic, and replayable audit evidence | Requires claim topology/account-layout verification before persistence | **Selected in ADR-0156** |

### Transaction-bound refund balance proof

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Persist a finalized `getMultipleAccounts` post-state | Simple and inexpensive | Cannot prove the pre-state or distinguish later unrelated transfers | Rejected |
| Independently read accounts before and after | Reuses account readers | Different finalized slots and races cannot prove one atomic movement | Rejected |
| Trust event amounts | Lowest latency | Does not prove committed token-account deltas or account-index binding | Rejected |
| **Finalized transaction `preTokenBalances`/`postTokenBalances` plus a separate finalized claim topology read** | Exact transaction slot, atomic pre/post observation, account-index binding, and explicit claim semantics | Requires strict bounded provider decoding and two RPC calls | **Selected in ADR-0157** |

### Refund-proof orchestration authority

| Option | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Client/operator supplies complete account topology | Smallest integration surface | Allows account substitution and cannot establish server authority | Rejected |
| Mutable database topology projection | Fast reads | Adds a second mutable authority and synchronization lifecycle | Rejected |
| **Resolve topology from the finalized claim inside a complete provider attempt** | Server-owned bindings, bounded failover, replayable evidence | Adds one bounded claim read | **Selected in ADR-0158** |

### Durable refund claim and execute intents

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Reuse the generic adjustment chain intent | Smallest schema | Collapses claim creation and execution authority and weakens proof semantics | Rejected |
| Let the browser provide refund accounts, amounts, and nonce | Minimal server orchestration | Makes financial topology and replay protection caller-controlled | Rejected |
| Server custody and immediate submission | Simple retry behavior | Expands custody and removes explicit merchant wallet consent | Rejected |
| **Separate server-owned claim/execute intents with exact signed-message verification** | Four-eyes authority, durable idempotency, replayable evidence, and non-custodial wallet flow | Additional lifecycle, evidence, and reconciliation implementation | **Selected in ADR-0175** |

### Refund claim preparation API

**Criteria:** keep financial inputs server-owned, bind the active finalized
funding topology, preserve wallet consent, and make concurrent browser retries
safe and replayable.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Browser supplies receipt, accounts, amounts, and nonce | Smallest request | Account substitution, stale evidence, and caller-controlled money | Rejected |
| Reuse the generic adjustment compiler | Existing route and response shape | Does not bind refund-funder authority or claim lifecycle | Rejected |
| Custodial server submission | Fewer wallet interactions | Key custody and no explicit merchant authorization | Rejected |
| **Server-owned idempotent claim-preparation endpoint** | Complete review artifact, tenant isolation, deterministic replay, and non-custodial signing | Requires bounded intent persistence and later submission APIs | **Selected in ADR-0176** |

### Signed refund-claim submission

**Criteria:** prevent artifact substitution, preserve wallet consent, fence
concurrent submissions, and make provider ambiguity recoverable.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Accept browser-supplied instruction and artifact | Small request | Account, signer, and digest substitution | Rejected |
| Recompile from browser fields at submit time | Flexible | Reopens stale topology and amount races | Rejected |
| Custodial server submission | Fewer wallet steps | Key custody and weaker consent boundary | Rejected |
| Treat provider acknowledgement as success | Low latency | Does not prove inclusion or finality | Rejected |
| **Persisted-artifact exact verification plus durable submission fence** | Non-custodial, replay-safe, and recoverable | Requires lifecycle storage and reconciliation | **Selected in ADR-0177** |

### Refund claim wallet workspace

**Criteria:** make two-signer authority explicit, preserve server-owned financial
inputs, support bounded recovery, and avoid implying refund completion.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Generic checkout panel | Reuses existing UI | Hides refund-funder co-signing and proof boundaries | Rejected |
| Custodial server signing | One-click experience | Adds key custody and removes wallet consent | Rejected |
| Browser-editable refund transaction | Flexible | Allows stale topology and amount substitution | Rejected |
| **Dedicated lazy two-signer workspace with memory-only artifact recovery** | Explicit review, role binding, accessible failure states, and no financial authority in the browser | Requires sequential wallet actions | **Selected in ADR-0178** |

### Durable refund-execute intent

**Criteria:** preserve claim/execute authority separation, bind execution to
finalized claim evidence and current verified topology, support safe wallet
recovery, and make proof-pending state explicit.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Reuse the claim intent | Less persistence and UI work | Collapses four-eyes authority and lifecycle evidence | Rejected |
| Compile from mutable current configuration | Simple recovery | Configuration rotation can silently change movement topology | Rejected |
| Trust signature/provider acknowledgement/event | Low latency | No finalized transaction-bound conservation proof | Rejected |
| Server custody of the funder key | Fewer wallet prompts | Violates non-custody and signer separation | Rejected |
| **Separate immutable execute intent with exact signed verification and proof-gated reconciliation** | Explicit authority, replayability, bounded retries, and fail-closed completion | Additional storage, API, UI, and reconciliation implementation | **Selected in ADR-0179** |

### Refund-claim finality reconciliation

**Criteria:** bind the persisted submission signature to the claim actually
created on Solana, retain forensic evidence, and unlock execute preparation only
after a bounded finalized proof.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Trust provider acknowledgement | Lowest latency | Not finalized evidence; ambiguous outcomes are unsafe | Rejected |
| Read the finalized claim PDA only | Simple account query | Does not bind state to the persisted transaction signature | Rejected |
| Accept browser-supplied recovery inputs | Flexible UX | Caller-controlled signature/address and substitution risk | Rejected |
| **Bounded finalized recovery plus exact raw claim-created event matching** | Signature binding, replayability, topology checks, forensic bytes, and restart-safe worker convergence | Requires event decoding, bounded RPC work, and operational backlog evidence | **Selected in ADR-0180** |

### Typed finalized refund-claim evidence boundary

**Criteria:** expose useful finalized-event proof to operators without moving
authority into the browser, weakening canonical wire validation, or implying
legal/tax compliance.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Status, signature, and slot only | Small response | The UI cannot show the exact event proof and may overstate finality | Rejected |
| Browser-decoded raw event bytes | Preserves raw data | ABI coupling and client-side authority risk | Rejected |
| Browser-supplied evidence | Flexible recovery | Evidence substitution and broken server ownership | Rejected |
| **Server-derived typed finalized-event projection with strict client validation** | Stable schema, proof binding, progressive disclosure, fail-closed parsing | Additional API contract and test surface | **Selected in ADR-0213** |

### Refund-claim reconciliation concurrency

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| No lease; rely only on finalization CAS | Minimal schema and simple retries | Duplicate replicas repeat RPC recovery and amplify provider load | Rejected |
| Process-local single-flight | Low local overhead | Does not coordinate replicas or survive restart | Rejected |
| Long database lock across RPC recovery | Strong serialization | Holds a database connection/lock across external I/O and can block unrelated work | Rejected |
| **Short PostgreSQL lease with `SKIP LOCKED`, TTL reclaim, and final CAS** | Cross-replica bounded ownership, restart recovery, no lock across RPC, safe duplicate convergence | Requires lease fields, metrics, and stale-lease tests | **Selected in ADR-0180** |

### Refund-claim cancellation reconciliation scheduling

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Browser-only or operator-only reconciliation | Minimal service work | Finality backlog is not self-healing and leaves user-visible state dependent on a manual action | Rejected |
| Broker-owned cancellation queue | Familiar delivery model | Adds a second durable work authority, duplicate delivery semantics, and queue recovery coupling | Rejected |
| Database lock held across finalized RPC recovery | Strong local serialization | Holds a pool connection and row lock across network I/O; stalls unrelated governance work | Rejected |
| **Bounded indexer worker with PostgreSQL UUIDv7 lease, `SKIP LOCKED`, stale reclaim, and token-fenced final CAS** | Self-healing convergence, replica coordination, crash recovery, replayable evidence, and no lock across RPC | Adds lease columns, worker telemetry, and a deterministic Docker drill | **Selected in ADR-0238** |
| **Forward migration with same-status lease-only trigger branch** | Repairs durable lease acquisition without changing lifecycle state or weakening immutable-field checks | Adds one corrective trigger migration and a regression drill | **Selected in ADR-0240** |

### Refund-funding topology verification concurrency

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Status transition only; allow duplicate provider reads | Minimal schema and simple retries | Replicas amplify finalized RPC traffic and stale workers can race activation | Rejected |
| Hold a PostgreSQL row lock across RPC verification | Serializes one configuration | Holds a pool connection across network I/O and can block governance or activation | Rejected |
| Redis/advisory lock | Low-latency coordination | Non-authoritative or weakly inspectable ownership; activation still needs durable fencing | Rejected |
| **Short PostgreSQL UUIDv7 lease with stale reclaim, explicit failure release, and token-fenced activation** | Durable cross-replica ownership, bounded crash recovery, no lock across RPC, exact evidence binding | Requires migration and lease telemetry | **Selected in ADR-0181** |

### Migration rollback rehearsal

**Criteria:** preserve immutable migration history, recover financial evidence
to a known pre-deploy image, keep the old application/schema compatible, and
prove forward recovery after the rollback decision.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Destructive `down` migrations | Familiar command shape; potentially fast | Can destroy evidence, is hard to make data-safe, and conflicts with immutable migration history | Rejected |
| Restore only | Proves a backup can be opened | Does not prove old-schema compatibility or safe forward recovery | Rejected |
| Corrective forward migration only | Preserves history and supports additive repair | Cannot recover an incompatible deployment without a pre-deploy image | Rejected as the sole contract |
| **Pre-deploy restore plus compatible old-schema read and forward reapply rehearsal** | Covers operator rollback, data preservation, compatibility, and redeployment recovery | Requires an isolated PostgreSQL drill and synthetic migration fixture | **Selected in ADR-0183** |

### Typed finalized adjustment proof envelope

**Criteria:** ensure the browser cannot render finalized evidence from a bare
status, preserve tenant/server authority, keep refresh recovery bounded, and
bind the response to the exact finalized typed event.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Status and signature only | Minimal response | Does not expose proof identity or prevent status-only UI success | Rejected |
| Client-supplied proof fields | Flexible client recovery | Caller-controlled evidence and substitution risk | Rejected |
| Re-query indexed evidence for every browser render | No schema change | Adds latency/availability coupling and no stable proof contract | Rejected |
| **Persist event index/digest and return a server-owned typed proof envelope** | Refresh-safe, replayable, bounded, and explicit | Additive migration and cross-language vectors required | **Selected in ADR-0184** |

### Refund-proof candidate ownership

| Option | Benefits | Costs / risks | Decision |
| --- | --- | --- | --- |
| Read-only unproved cursor | No coordination schema | Duplicate provider work and replica races | Rejected |
| Database lock held across RPC | Strong active exclusion | Couples pool capacity to external latency and stalls | Rejected |
| Redis/distributed lock | Familiar lease primitive | Disposable coordination is not evidence-bound; adds failure authority | Rejected |
| **PostgreSQL UUIDv7 lease with `SKIP LOCKED`, TTL reclaim, and final token CAS** | Durable ownership, bounded crash recovery, no database lock during RPC, stale-worker fencing | Requires additive lease columns, metrics, and contention tests | **Selected in ADR-0185** |
| **Storage-local bounded retries for refund-intent serializable conflicts** | Preserves atomic idempotency/event/outbox writes while absorbing ordinary `40001`/`40P01` contention | Adds bounded retry latency and requires measured retry telemetry | **Selected in ADR-0186** |
| **Refund-execute replay authorization** | Compiled approver public key plus immutable adjustment approver subject | Adds one bounded tenant-scoped lookup on replay/read/submit | **Selected in ADR-0187; avoids comparing Solana keys to OIDC subjects** |
| **Refund-execute contention latency evidence** | Bounded authenticated API samples with p50/p95/p99 and explicit workload metadata | Local-only evidence with a small sample; excludes live wallet/RPC and regional variance | **Selected in ADR-0188** |

### Automated indexer replay recovery contract

**Criteria:** prove that persisted finalized-slot gaps are recovered by the
running indexer, that multiple replicas coordinate through durable leases, that
forward checkpoint state is not mutated by replay, and that provider work stays
bounded and observable.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Unit tests for the pure replay queue only | Fast and deterministic | Does not prove PostgreSQL leases, service wiring, or restart behavior | Rejected as the sole gate |
| Manual SQL plus an operator replay endpoint | Easy to demonstrate | Does not prove automatic recovery or replica coordination | Rejected |
| Broker-owned replay queue | Familiar asynchronous delivery model | Adds another authority, duplicate delivery semantics, and settlement-path coupling | Rejected |
| **Isolated Docker PostgreSQL + RPC fixture + two running indexer replicas** | Exercises durable claim/lease, bounded RPC page recovery, projection persistence, checkpoint isolation, and observable completion | Synthetic chain data and local timing do not replace Linux validator/live-provider SLO evidence | **Selected in ADR-0189** |

### Indexer replay retry-exhaustion contract

**Criteria:** prove that a persistent provider failure cannot create an
unbounded retry loop, that the exact lease is released into explicit
dead-letter state after the configured attempt limit, that the last failure is
observable, and that no forward checkpoint is advanced.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Pure state-machine tests | Fast and deterministic | Misses service polling, SQL lease release, and metric refresh wiring | Rejected as the sole gate |
| Manual insertion of a pre-dead-letter row | Exercises read/alert UI | Bypasses automatic retry and exhaustion behavior | Rejected |
| Unbounded retry with operator cancellation | May eventually recover transient provider faults | Retry storms and no finite incident boundary | Rejected |
| **Two real indexer replicas plus a fixture that fails one finalized RPC method** | Proves bounded automatic attempts, lease cleanup, dead-letter persistence, fixed-cardinality metrics, and checkpoint isolation | Synthetic provider and local timing do not prove production notification delivery | **Selected in ADR-0190** |

### Alertmanager delivery contract

**Criteria:** prove that operational alerts leave Prometheus, preserve bounded
identity and ownership metadata, reach a receiver, remain outside financial
authority, and can be tested reproducibly without a production notification
provider.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Service-local notification calls | Immediate failure proximity | Couples hot paths to notification outages, duplicates policy, and expands the settlement trust boundary | Rejected |
| Prometheus rules with no receiver drill | Lowest operational cost | Proves evaluation only; delivery regressions remain invisible | Rejected |
| Direct Prometheus webhook fan-out | Fewer components | Weak grouping, retry, deduplication, silence, and escalation semantics | Rejected |
| **Prometheus → Alertmanager → approved receiver, with a bounded local Rust fixture for CI** | Standard operational boundary, explicit delivery contract, grouping/deduplication, reproducible evidence, and deployment-owned escalation | Adds an operational component and leaves production receiver selection to deployment | **Selected in ADR-0191** |

### Real-browser vault-reconciliation evidence

**Criteria:** prove the shipped SPA renders server-owned reconciliation
evidence and fails closed on authorization or mismatch without conflating
synthetic UI fixtures with live-chain evidence.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Unit/component tests only | Fast and deterministic | Misses the built SPA, lazy imports, browser focus, and network boundary | Rejected as the sole gate |
| Full live-provider browser test in CI | Closest to production | Requires credentials and unstable external state; mixes UI and chain evidence | Rejected for the deterministic browser gate |
| Fake success page | Low implementation cost | Can hide unsafe success and failure UX | Rejected |
| **Real built SPA with bounded network fixtures and explicit failure assertions** | Reproducible browser behavior, safe synthetic data, and explicit mismatch/authorization evidence | Does not replace validator, wallet, live-provider, or production identity evidence | **Selected in ADR-0192** |

### Server-owned reconciliation freshness

**Criteria:** prevent an old matched observation from being rendered as current
financial evidence while preserving append-only runs, tenant isolation, and a
bounded low-latency read path.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Browser-local timestamp comparison | No API change | Clock skew and inconsistent caller interpretation | Rejected |
| Caller-supplied threshold | Operational flexibility | Each caller can redefine evidence validity | Rejected |
| Mutate historical rows to stale | Simple UI status | Breaks append-only evidence and conflates observation with read-time freshness | Rejected |
| Timestamps with operator interpretation | Minimal schema | Unsafe ambiguity in a financial control surface | Rejected |
| **Database-clock server-owned stale boolean with fixed 90-second policy** | Deterministic, append-only compatible, and explicit | Policy changes require a superseding ADR | **Selected in ADR-0193** |

### Wallet-bound adjustment finality evidence

**Criteria:** prove the local path from server-owned instruction compilation
through real wallet transaction serialization, signed-byte verification,
bounded submission fencing, finalized observation, and exact typed-event proof
binding without credentials or nondeterministic public-RPC state.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Unit verifier tests only | Fast and precise | Does not prove API persistence, provider observation, or lifecycle fencing | Rejected as the sole gate |
| Pre-seeded finalized intent and event rows | Easy to repeat | Bypasses the wallet and submission boundaries that are highest risk | Rejected |
| Live wallet/provider test in public CI | Production-like | Requires secrets and external state; unsafe and nondeterministic | Rejected for the deterministic gate |
| **Real API and PostgreSQL with synthetic finalized RPC/indexer contracts plus real wallet serialization** | Complete local authority-boundary evidence, reproducible and credential-free | Does not close validator, live-provider, KMS, or production-SLO gates | **Selected in ADR-0194** |

### Transaction-bound refund proof evidence

| Criterion | Unit/storage-only tests | Direct proof-row seed | Public/live RPC CI | Real indexer + PostgreSQL + synthetic finalized RPC |
| --- | --- | --- | --- | --- |
| Exercises claim topology resolution | No | No | Yes, but variable | Yes |
| Exercises transaction/event/balance parser | Partial | No | Yes, but variable | Yes |
| Exercises UUIDv7 lease and worker wiring | No | No | No | Yes |
| Reproducible public CI without credentials | Yes | Yes | No | Yes |
| Evidence of live provider agreement/validator execution | No | No | Partial | No |

Selected: the deterministic Docker drill in ADR-0195. It closes the local
service-wiring and proof-persistence boundary without overclaiming live-chain
or legal evidence.

### Document-bound checkout workspace

**Criteria:** preserve the document/aggregate authority firewall, expose the
issuer and scope evidence chain to operators, keep the initial SPA bundle
bounded, and make wallet/finality failure states unambiguous.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Extend aggregate checkout with a document mode | Reuses the existing form and wallet path | Blurs authority boundaries and makes accidental aggregate compilation easier | Rejected |
| Operator-only HTTP workflow | Smallest UI change | Not a supportable, accessible product surface; increases evidence-entry errors | Rejected |
| Browser-side attestation calculation/signing | Low interaction latency | Violates issuer custody and server-owned evidence; unsafe for real money | Rejected |
| **Dedicated lazy document workspace over typed verification, compilation, wallet, and reconciliation boundaries** | Explicit type firewall, evidence-first UX, code-split delivery, and reusable finality controls | Adds a separately tested workspace | **Selected in ADR-0207** |

### Authenticated document-quote workspace

**Criteria:** keep simulation and production quote authority separate, preserve
server-owned jurisdiction and amount semantics, support exact idempotent retry,
and provide a discoverable accessible path into document settlement.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Reuse the simulation form against the production endpoint | Minimal visual duplication | Can accidentally submit caller-owned rule data and blur unattested simulation with persisted evidence | Rejected |
| Hide document quotes behind aggregate quote UI | Fewer workspaces | Conceals document-specific facts, attestation, and line-scope semantics | Rejected |
| API-client-only workflow | No SPA implementation | Undocumented, inaccessible, and operationally error-prone | Rejected |
| **Dedicated authenticated lazy workspace** | Clear authority boundary, exact server evidence, and direct document-checkout handoff | Additional UI and contract tests | **Selected in ADR-0208** |

### Retained Docker integration evidence

**Criteria:** preserve the authoritative integration command semantics, make
success and failure behavior reviewable for a specific commit, avoid leaking
credentials or private evidence, and keep the artifact bounded.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Retain only the CI exit status | Minimal configuration | Loses scenario cardinality and recovery diagnostics | Rejected |
| Upload complete Docker logs and volumes | Maximum raw detail | Noise, sensitive-data risk, and poor public-safety boundary | Rejected |
| Add a second bespoke JSON reporter | Structured output | Duplicates the authoritative gate and can drift | Rejected |
| **Pipe the authoritative bounded integration stream to a retained CI artifact** | Exact command semantics, public-safe output, success/failure review, bounded retention | Requires CI artifact storage | **Selected in ADR-0217** |

### Explicit release integrity hashes

**Criteria:** bind authority-sensitive release bytes to named machine-readable
claims, hash the effective rather than source-only deployment configuration,
preserve one checksum source of truth, and fail before release evidence is
reported complete.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Generic artifact list only | Simple and already available | Consumers must infer program/configuration paths | Rejected |
| Hash source Compose files | Easy to inspect | Does not bind profiles, interpolation, or resolved image configuration | Rejected |
| Detached configuration hash file | Small change | Creates a second source of truth outside the manifest | Rejected |
| **Named manifest integrity fields plus the common checksum set** | Explicit, reproducible, machine-readable, and reviewable | Requires resolved configuration as a bundle input | **Selected in ADR-0218** |

### Release evidence source-revision binding (ADR-0299)

**Criteria:** prevent valid validator evidence from another commit being
combined with the packaged binaries, fail before release artifacts are copied,
and preserve the existing bounded evidence schema.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Trust the workflow run or evidence directory name | Minimal change | Names do not bind evidence to source bytes | Rejected |
| Bind only the SBF artifact hash | Protects one authority-sensitive binary | Does not bind API, migrations, UI, or configuration | Rejected |
| Manual revision comparison | Easy to understand | Human omission can produce a false release package | Rejected |
| **Compare evidence metadata with the packaged Git revision** | Deterministic, bounded, auditable, compatible with current evidence | Requires one metadata field and negative test | **Selected; see ADR-0299** |

### API JWKS rotation

**Criteria:** preserve zero network and filesystem work on the JWT request
path, rotate identity keys without avoidable process downtime, retain the last
known-good configuration on failure, and make the operational boundary
auditable.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Restart-only key rotation | Simple deployment model | Adds downtime and makes emergency rotation depend on rollout scheduling | Rejected |
| Identity-provider JWKS fetch on every request | Immediate provider changes | Unbounded hot-path latency and external availability dependency | Rejected |
| Periodic background JWKS polling | Automatic rotation | Requires stale-cache, provider-authentication, backoff, and split-brain policy before safe production use | Rejected for this contract |
| Mutable `RwLock` key map | No extra dependency | Adds request-path contention and couples hot-path cost to reloads | Rejected |
| **Immutable key snapshot with atomic file reload on `SIGHUP`** | No request-path I/O, bounded validation, deterministic last-known-good behavior, explicit secret-manager ownership | Requires atomic delivery and signal/runbook discipline | **Selected in ADR-0234** |

### Jurisdiction-provider key registry rotation

**Criteria:** preserve deterministic quote authorization latency, remove a
compromised provider key without request-path network or filesystem work,
retain a last-known-good registry on malformed delivery, and keep provider
authority changes auditable.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Restart-only registry rotation | Simple deployment model | Couples evidence-authority response to process rollout and downtime | Rejected |
| Fetch provider keys from a network for every quote | Immediate provider changes | Adds external availability and unbounded latency to quote authorization | Rejected |
| Periodic background registry polling | Automatic rotation | Requires provider authentication, stale-cache, backoff, and split-brain policy | Rejected for this contract |
| Mutable registry behind `RwLock` | Simple implementation | Adds request-path lock contention and reload coupling | Rejected |
| **Immutable registry snapshot with atomic file reload on `SIGHUP`** | Bounded validation, zero request-path I/O, deterministic last-known-good behavior, explicit secret-manager ownership | Requires atomic delivery and signal/runbook discipline | **Selected in ADR-0235** |

### UUIDv7 schema invariant coverage

**Criteria:** cover clean bootstraps and upgraded PostgreSQL schemas, detect
new generated identifiers without a second manual registry, validate existing
rows before accepting the schema, and keep UUID generation and request paths
free of new coordination overhead.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Hand-written expected-column list | Easy to read and review | Drifts when a migration adds a generated identifier | Rejected |
| Migration-source text scan | Works without a database | Cannot prove live defaults, types, or validated constraints after upgrades | Rejected |
| Database trigger on all UUID writes | Central enforcement | Adds write-path cost and incorrectly couples UUIDv7 policy to external references and protocol identifiers | Rejected |
| **PostgreSQL catalog discovery plus validated corrective migration** | Covers clean/upgraded schemas, no request-path cost, fail-closed drift detection | Migration validation may scan existing tables and requires bounded rollout planning | **Selected in ADR-0239** |

### Linux conformance workflow integrity

**Criteria:** prevent workflow drift from weakening the authoritative
Linux/amd64 validator boundary, keep the check deterministic and credential
free, preserve fail-closed artifact handling, and avoid duplicating the
conformance execution path.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Reviewer checklist only | No implementation work | Human review can miss removal of runner, verifier, or retention invariants | Rejected |
| Runtime evidence verifier only | Strong postcondition after execution | Cannot prove the workflow invoked the authoritative command on the required runner | Rejected |
| Second independent conformance workflow | Extra separation | Duplicate cost and two sources of release truth | Rejected |
| **Static contract over CI and release-candidate workflows** | Deterministic, public-safe, credential-free, and runs in normal checks | Requires intentional updates with workflow changes | **Selected in ADR-0241** |

### Protocol pause coverage for vault release execution

**Criteria:** stop every protocol-controlled token movement during an
emergency pause, preserve one authoritative state, fail before CPI, and avoid
API-only or duplicated governance controls.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Pause only ordinary settlement | No release account change | Leaves treasury release as an emergency-stop bypass | Rejected |
| API-only pause check | No program change | Direct wallet/RPC callers can bypass the API | Rejected |
| Separate mutable release pause flag | Localized | Duplicates and can diverge from protocol governance state | Rejected |
| **Read the protocol pause PDA in release execution** | Uniform chain authority, fail-closed before CPI, consistent with other movement instructions | Adds one read-only account to the versioned instruction | **Selected in ADR-0242** |

### Program pause-coverage enforcement

**Criteria:** detect future token-movement entry points that omit the emergency
pause boundary, remain deterministic and credential-free, preserve the on-chain
ABI, and keep the review surface explicit.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Reviewer grep/checklist | No tooling | Future movement paths can bypass pause unnoticed | Rejected |
| Unit tests only | Precise behavior tests | Does not guarantee every entry point remains covered | Rejected |
| Runtime reflection or macro registry | Centralized metadata | Adds on-chain/build complexity and ABI risk | Rejected |
| **Explicit static entry-point coverage contract** | Deterministic, low overhead, credential-free, discovers all public functions containing token CPIs, and fails before deployment review | New movement paths require an intentional allowlist/guard update | **Selected in ADR-0243** |

### Foreign-exchange normalization

**Criteria:** exact arithmetic, replayable provider evidence, no network access
in the calculation hot path, bounded latency, and explicit source/target
currency semantics.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Browser/API floating-point conversion | Simple | Client authority, rounding drift, and weak replay evidence | Rejected |
| Live provider call from the tax engine | Fresh quote | Nondeterministic latency and impure calculation | Rejected |
| Caller-provided normalized amount without evidence | Small domain surface | Rate and rounding cannot be replayed or audited | Rejected |
| **Typed exact FX evidence and pure normalization** | Deterministic, bounded, auditable, and cache-friendly | Requires upstream provider verification and freshness policy | **Selected in ADR-0244** |

### Server-owned signed FX evidence

**Criteria:** cryptographic provider authenticity, exact payload binding,
bounded verification, atomic key rotation, replayability, and no provider
network call in the tax hot path.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Generic jurisdiction evidence envelope | Reuses signing code | Rate semantics and lifecycle become ambiguous across bounded contexts | Rejected |
| Unsigned registry/provider data | Minimal latency | No cryptographic proof of provider output | Rejected |
| FX lookup inside the tax engine | Freshness | Nondeterministic and non-replayable domain behavior | Rejected |
| **Dedicated signed `fx-evidence` context with server-owned adapter** | Typed, bounded, replayable, and independently rotatable | Requires provider key/custody operations | **Selected in ADR-0245** |

### FX provider policy and issuance

**Criteria:** exact deviation semantics, legal-policy ownership, bounded
freshness, signer custody separation, deterministic testing, low-latency
operation, and provider replacement without tax-engine changes.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Provider HTTP and signing inside the Tax Engine | Fewer processes | Impure, nondeterministic, and difficult to replay | Rejected |
| Provider-defined freshness/legal decisions | Simple integration | Inconsistent policy and weak governance | Rejected |
| Caller-provided rate with later review | Low initial latency | Forged authority and settlement ambiguity | Rejected |
| One vendor-specific adapter immediately | Fast vendor integration | Premature legal, regional, and commercial coupling | Rejected |
| **Vendor-neutral source, policy, and signer ports** | Exact, bounded, auditable, low-latency, replaceable | Requires explicit production provider and custody gates | **Selected in ADR-0246** |

### FX provider policy snapshot lifecycle

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| PostgreSQL read per issuance | Centralized | Adds hot-path latency and mutable replay behavior | Rejected |
| Remote configuration read per issuance | Centralized rollout | External dependency and tail-latency variance | Rejected |
| Restart for policy rotation | Simple | Slow rotation and broad failure blast radius | Rejected |
| Mutable shared policy object | Low code volume | Partial reads and weak digest binding | Rejected |
| **Bounded immutable digest-keyed atomic snapshot** | Fast lock-free reads, deterministic replay, fail-closed reload | Requires governed delivery and activation evidence | **Selected in ADR-0247** |

### Registry-bound FX evidence issuance

**Criteria:** prevent unapproved policy bypass, preserve exact request
authority, keep the issuance path bounded, and retain snapshot replacement
isolation.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Direct issuer construction | Minimal API surface | Allows evidence under a policy absent from the active registry | Rejected |
| Caller assertion that a policy is approved | Small request type | No cryptographic or registry-verifiable authority | Rejected |
| Database/remote policy lookup per request | Centralized | Hot-path dependency and mutable replay semantics | Rejected |
| **Exact digest lookup through the immutable registry before issuer construction** | Fail-closed, lock-free, deterministic request snapshot | Production adapters must use the bound API | **Selected in ADR-0248** |

### FX provider identity authorization

**Criteria:** bind provider authority to the approved policy, preserve tenant
and jurisdiction-specific governance, remain replayable, and fail closed on
unapproved quote sources.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Trust the candidate provider string | Minimal configuration | Any source can claim an accepted identity | Rejected |
| Global process-wide allowlist | Easy lookup | Cannot express policy-scoped approval or clean replay | Rejected |
| Caller-supplied provider authorization | Flexible | Untrusted request becomes an authority boundary | Rejected |
| **Exact bounded provider allowlist in the immutable policy** | Policy-scoped, deterministic, and fail-closed | Provider rotation requires a new policy version | **Selected in ADR-0249** |

### FX provider agreement

**Criteria:** deterministic replay, exact financial semantics, bounded provider
fan-out, explicit disagreement behavior, and no provider networking in the
tax-engine hot path.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| First successful provider | Lowest latency | Transient order/outage changes tax results | Rejected |
| Average or median rates | Tolerates market differences | Adds ambiguous legal and rounding semantics; can hide bad data | Rejected |
| Silently discard invalid/disagreeing candidates | Higher availability | Masks provider faults and weakens evidence | Rejected |
| **Exact economic agreement with policy-priority selection** | Deterministic, auditable, bounded, and conservative | Lower availability on disagreement | **Selected in ADR-0250** |

### Tax Engine delivery sequencing

**Criteria:** preserve all correctness, security, legal, conformance, and
operational gates while reducing calendar time through independent work.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Sequential workstreams | Simple coordination | Delays independent evidence and extends external-dependency exposure | Rejected |
| Implement before scope and authority are frozen | Fast apparent start | Creates rework and unsafe legal/provider assumptions | Rejected |
| Parallel work without named owners or exit artifacts | Maximum concurrency | Ambiguous accountability and weak release evidence | Rejected |
| **Three owned workstreams behind a Day 2 decision freeze** | Shortens schedule while preserving explicit exit evidence and unchanged Tax Engine gates | Requires integration governance and merge coordination | **Selected in ADR-0251** |

### FX provider acquisition

**Criteria:** preserve exact agreement semantics while bounding task count,
socket usage, tail latency, cancellation, and provider identity.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Sequential provider calls | Simple | Provider tail latency accumulates and failover is slow | Rejected |
| Unbounded async fan-out | Fast apparent success | Resource exhaustion, retry storms, and detached work | Rejected |
| First successful response | Lowest apparent latency | Nondeterministic authority and no agreement proof | Rejected |
| Broker-backed acquisition | Durable retries | Adds a dependency and expiry complexity to quote issuance | Rejected |
| **Bounded concurrent acquisition with per-provider and aggregate deadlines** | Low latency, bounded resources, explicit cancellation, and deterministic handoff to policy | Requires adapter-level retry/circuit controls | **Selected in ADR-0252** |

### FX acquisition outcome normalization

**Criteria:** preserve replayability and evidence comparability without adding
network work, unbounded memory, or a second authority for provider admission.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Preserve task completion order | Zero normalization work | Scheduler and provider latency change replay order | Rejected |
| Lexicographic provider ordering | Stable and simple | Can diverge from policy priority and admission order | Rejected |
| Normalize independently in each consumer | Consumer-specific flexibility | Duplicated security-sensitive behavior and inconsistent evidence | Rejected |
| **Normalize once by policy priority in the acquisition coordinator** | Stable, bounded, aligned with policy, and auditable | Small bounded post-processing step | **Selected in ADR-0254** |

### FX provider HTTP transport

**Criteria:** preserve provider neutrality, bound network and memory exposure,
prevent credential/endpoint misuse, and keep transport failure separate from
policy admission and evidence signing.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| HTTP inside the policy/orchestration crate | Fewer crates | Couples transport security to admission and makes testing less isolated | Rejected |
| Vendor-specific adapter first | Fast path to one integration | Freezes unresolved provider, legal, regional, and custody decisions | Rejected |
| Unbounded generic HTTP client | Flexible | Implicit body, timeout, proxy, retry, and credential behavior | Rejected |
| Broker-backed provider request | Durable asynchronous delivery | Adds expiry and delivery ambiguity to the quote path | Rejected |
| **Separate bounded vendor-neutral HTTP source** | Explicit limits, strict decoding, transport isolation, and open provider gates | Requires a later approved provider adapter/configuration | **Selected in ADR-0255** |

### FX signer HTTP transport

**Criteria:** preserve exact payload bytes, keep key identity server-owned,
bound remote signing behavior, and avoid duplicate or ambiguous signing.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| HTTP inside the FX policy crate | Fewer modules | Couples custody transport to policy and makes key-boundary review harder | Rejected |
| Direct vendor SDK | Native vendor controls | Freezes an unresolved vendor and complicates portability/testing | Rejected |
| Broker-backed signing | Durable delivery | Expiry, idempotency, and duplicate-sign ambiguity in the quote path | Rejected |
| Accept arbitrary signature bytes | Low apparent latency | No strict shape or transport contract; substitution risk | Rejected |
| **Separate bounded vendor-neutral HTTP signer** | Exact payload, bounded response, no retry, and explicit custody release gates | Requires later HSM/KMS and service-identity evidence | **Selected in ADR-0256** |

### FX issuance composition

**Criteria:** compose acquisition, policy selection, signing, and verification
without mixing registry snapshots, duplicating authority logic, or adding a
broker to the low-latency quote path.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| API calls acquisition and issuance separately | Small endpoint changes | Duplicated orchestration and policy-reload race | Rejected |
| Orchestrate inside the pure Tax Engine | One apparent flow | Violates domain purity and introduces network/custody dependencies | Rejected |
| Broker-backed issuance workflow | Durable delivery | Quote expiry, delivery, and duplicate-sign ambiguity | Rejected |
| **Server-owned bounded coordinator in `fx-provider`** | One snapshot, one verified evidence result, reusable ports, and no second authority | Requires later API persistence/authorization composition | **Selected in ADR-0257** |

### FX replay integrity and signer ownership

| Alternative | Benefits | Risks / rejection reason | Decision |
| --- | --- | --- | --- |
| Reuse current-time validity verification for historical replay | One verifier | Expired but valid historical quotes cannot be replayed | Rejected |
| Store only typed conversion fields and an evidence digest | Small persistence payload | Does not prove canonical payload or signature integrity | Rejected |
| Require signer adapters to be cloneable | Simple coordinator ownership | Couples the port to key material ownership and excludes non-exportable HSM/KMS sessions | Rejected |
| **Separate integrity verification from time validity and share signers through `Arc`** | Correct historical replay semantics and one shared custody boundary | Requires an explicit durable-envelope persistence integration | **Selected in ADR-0258** |

### Authenticated FX quote composition

**Criteria:** keep provider, policy, legal, and custody authority server-owned;
bind FX and jurisdiction evidence to one canonical facts digest; preserve the
same-currency low-latency path; and keep non-local activation fail closed while
operational decisions remain open.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Accept signed FX evidence from the browser | Simple request shape | Caller can bypass provider policy, legal admission, and custody controls | Rejected |
| Call providers directly from each quote handler | Few abstractions | Duplicated orchestration and policy snapshot races | Rejected |
| Put FX issuance in the pure Tax Engine | One apparent calculation flow | Violates deterministic domain purity and adds network/custody dependencies | Rejected |
| Queue issuance through a broker | Durable asynchronous delivery | Quote expiry and duplicate-sign ambiguity in the synchronous path | Rejected |
| Concrete vendor integration now | Immediate external connectivity | Invents unresolved provider/legal/custody decisions | Rejected |
| **Server-owned API issuance port backed by the bounded coordinator** | Shared binding contract, bounded latency, explicit fail-closed activation, and future vendor composition | Requires configuration and release evidence before non-local traffic | **Selected in ADR-0260** |

### FX issuer process bootstrap

**Criteria:** construct provider and signer adapters once at startup, keep
secrets and authority server-owned, preserve bounded hot-path behavior, and
prevent open operational decisions from enabling non-local traffic.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Keep the API issuer permanently absent | Zero configuration | The real local authenticated cross-currency path cannot be exercised | Rejected |
| Construct adapters in quote handlers | Minimal bootstrap code | Repeated allocation/configuration and secret-handling work in the latency-critical path | Rejected |
| Vendor-specific SDK bootstrap | Direct provider integration | Invents unresolved provider, legal, and custody decisions | Rejected |
| **Dedicated local-only process bootstrap over existing bounded ports** | Warm immutable topology, strict startup validation, and explicit non-local gate | Requires a versioned configuration contract and fixture endpoints | **Selected in ADR-0261** |

### FX evidence signing

**Criteria:** protect key custody, avoid blocking async executors, bind the
signature to exact canonical evidence, and fail closed under remote signer
failure or timeout.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Synchronous signer for remote HSM/KMS | Small interface | Blocks or hides executor isolation and has no explicit deadline | Rejected |
| Sign inside the tax engine | Fewer calls | Breaks purity and couples calculation to custody | Rejected |
| Trust returned signature bytes | Lower latency | Payload/key/signature substitution risk | Rejected |
| Broker-backed signing | Durable retry | Ambiguous issuance/idempotency and excessive quote latency | Rejected |
| **Async signer port with deadline and local envelope verification** | Non-blocking, vendor-neutral, bounded, and auditable | Requires explicit custody adapter and rotation ceremonies | **Selected in ADR-0253** |

### Production SaaS authorization-aware UI

**Criteria:** preserve server authority, make RBAC/PBAC/ABAC explainable,
support keyboard and responsive operation, avoid misleading state, keep the
settlement hot path unchanged, and prevent policy drift across workspaces.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Long workspace tab strip | Small change and familiar implementation | Poor information architecture, weak risk context, difficult to scale | Rejected |
| Decode JWT claims in React | Immediate local affordances | Untrusted claims, tenant/resource substitution, stale policy, false authorization | Rejected |
| Frontend BFF policy gateway | Centralized browser API | Adds a second authority, latency hop, and failure domain to protected flows | Rejected |
| Micro-frontends | Independent team releases | Runtime/policy/accessibility drift and more operational complexity | Rejected |
| **Single React SPA product shell plus server-evaluated capability contract** | Consistent SaaS UX, explicit risk context, low navigation latency, one policy catalog, server remains authoritative | Requires capability endpoint and contract tests before personalized production rollout | **Selected in ADR-0262** |

### Browser OIDC ID-token validation

**Criteria:** bind the provider response to the one-time browser transaction;
verify issuer, client, signature, nonce, and time claims; preserve the static
NGINX deployment; bound provider work and key-rotation behavior; and keep API
authorization authoritative.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Ignore the ID token and rely only on the access token | Smallest browser implementation | Leaves the requested OIDC nonce and `openid` response unverified | Rejected |
| Decode ID-token claims without signature verification | Very low latency | Claims are attacker-controlled | Rejected |
| Add a stateful BFF for token validation | Centralized validation and HttpOnly custody | New service, latency hop, failure domain, and deployment model | Deferred |
| **Bounded Web Crypto validation in the SPA with a pinned JOSE library** | Preserves static delivery, validates before session creation, and keeps key work off financial hot paths | Requires strict JWKS parsing, rotation evidence, and dependency governance | **Selected in ADR-0270** |

### Task-oriented product language and navigation

**Criteria:** make finance and tax workflows understandable on first use;
preserve exact evidence, security, and chain context; support mixed operator,
administrator, and engineering audiences; and avoid changing the underlying
authorization or settlement authority.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Keep engineering terms in the primary journey | Precise for developers | Forces operators to translate implementation concepts and makes the product feel like an internal console | Rejected |
| Remove technical language entirely | Friendly first impression | Hides important authorization, evidence, and chain boundaries | Rejected |
| Separate operator and developer applications | Each surface can optimize for one audience | Duplicates workflows and increases policy, accessibility, and support drift | Deferred |
| **Use task-oriented labels with progressive technical disclosure** | Clear outcomes for operators while retaining exact audit and support detail | Requires content governance and terminology review for every workspace | **Selected in ADR-0263** |

### Server-owned capability snapshot

**Criteria:** keep tenant and authorization authority on the server; make the
shell explainable; avoid database, RPC, broker, or identity-provider work on
navigation; bound latency and response size; and fail closed for missing
resource context.

| Alternative | Strengths | Risks / costs | Decision |
| --- | --- | --- | --- |
| Decode JWT claims in React | No extra request | Untrusted/stale claims and no resource policy evaluation | Rejected |
| Put capabilities in login tokens | One response at login | Policy staleness, large tokens, difficult revocation | Rejected |
| Query PostgreSQL for every shell load | Rich context | Adds storage latency and a failure domain to navigation | Rejected |
| Policy BFF in front of the API | Central policy surface | Second authority and an extra hop on protected operations | Rejected |
| **Bounded server snapshot from the verified principal and policy catalog** | Low latency, explainable, no persistence dependency, server remains authoritative | Resource-specific conditions remain indeterminate until workflow context exists | **Selected in ADR-0264** |

### Executable UI language contract

**Criteria:** keep primary journeys understandable to finance and tax
operators, preserve exact protocol evidence for specialist review, prevent
copy regressions in lazy workspaces, and avoid adding a policy or latency
authority to the settlement path.

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Manual design review only | No tooling or maintenance | Regressions are easy to merge and inconsistent across workspaces | Rejected |
| Ban technical terms from all frontend source | Simple search rule | Prevents accurate evidence, tests, and advanced review details | Rejected |
| Separate operator and engineering applications | Optimized language per audience | Duplicated workflows and policy/accessibility drift | Deferred |
| **Executable primary-copy contract with progressive disclosure** | Blocks primary-surface regressions while preserving exact evidence | Requires a maintained catalog and periodic human language review | **Selected in ADR-0271** |

### Payment review progressive disclosure

**Criteria:** let a finance or operations user understand the amount and
authorization decision without protocol knowledge; preserve exact route,
account, policy, compute, and intent evidence for controlled review; keep
wallet and server authority boundaries unchanged; and avoid adding work to the
settlement hot path.

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Show protocol fields in the primary payment form | Familiar to blockchain engineers | High cognitive load and unsafe copy/paste pressure for operators | Rejected |
| Hide protocol evidence | Simplest payment screen | Removes audit and incident-response evidence | Rejected |
| Tooltips only | Small visual footprint | Poor keyboard, touch, screen-reader, and support workflows | Rejected |
| **Plain-language primary review with native accessible disclosures** | Clear authorization decision with inspectable evidence and no runtime authority change | Requires copy governance and disclosure tests | **Selected in ADR-0272** |

### Explicit financial inputs in financial workspaces

**Criteria:** prevent accidental submission of invented or stale financial
facts; preserve a fast real-user workflow; keep retry identity stable; and
avoid creating a demo mode inside the production application.

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Open with realistic sample amounts and rule values | Fast first interaction | Misleading account context and accidental financial submission | Rejected |
| Add a separate demo mode | Easy exploration | Duplicated behavior and unclear production boundary | Rejected |
| **Start financial fields empty and require explicit entry** | Honest, auditable, and still supports the real quote, invoice, document-payment, and rule-simulation flows | Adds a small amount of initial operator input | **Selected in ADR-0273** |

### Explicit currency in aggregate tax workspaces

**Criteria:** keep the global tax contract honest; never infer financial
meaning from locale or a hidden default; preserve server authority for asset
support and precision; and avoid adding a client-side settlement authority.

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Hidden `USD` default | Fastest form | Silently constrains global behavior and can misstate tax facts | Rejected |
| Infer from browser locale | No extra field | Locale is a display preference, not a financial fact | Rejected |
| Read from client-side route state | Avoids duplicate entry | Client state cannot authorize supported currency or settlement compatibility | Rejected |
| **Require explicit currency and validate it on the server** | Honest global input and one authoritative compatibility boundary | Adds one required field | **Selected in ADR-0274** |

### Capability snapshot latency evidence

**Criteria:** measure the real authenticated API boundary, preserve semantic
authorization correctness under load, exercise replica behavior, keep local
evidence reproducible, and avoid turning a benchmark into a production
authority.

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| In-process Rust microbenchmark only | Fast and stable | Omits HTTP, JWT verification, serialization, and replica behavior | Rejected |
| External unbounded load platform | Broad load generation | Adds environment and supply-chain variance to a local gate | Rejected |
| Browser timing assertion only | Exercises the visible shell | Browser scheduling and network noise obscure service latency | Rejected |
| **Bounded Docker HTTP benchmark across two API replicas with semantic checks** | Reproducible, authenticated, concurrency-aware, and correctness-preserving | Local synthetic results do not prove regional production SLOs | **Selected in ADR-0275** |

### Multi-tenant capability session reset

**Criteria:** prove stale tenant context is removed in the real SPA, preserve
the server-owned authorization boundary, exercise the credential replacement
transition, and avoid confusing UI evidence with identity-provider or API
authorization evidence.

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Parser unit test only | Fast and deterministic | Does not exercise React state or rendered context | Rejected |
| Single-tenant browser journey | Proves initial capability rendering | Misses cross-tenant replacement and stale-state leakage | Rejected |
| Browser JWT claim decoding | Convenient fixture setup | Violates server-owned authorization and tenant derivation | Rejected |
| **Two-tenant real-browser capability-response journey** | Proves request replacement, reset state, and stale-context removal | Synthetic responses do not replace staging identity-provider evidence | **Selected in ADR-0276** |

### Complete workspace capability catalog

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Keep seven broad capability IDs | Small response | Leaves document payment, adjustment, refund, and reconciliation navigation without an explicit server decision | Rejected |
| Emit every protected API endpoint | Precise | Couples product navigation to transport details and expands the policy surface without bound | Rejected |
| Hide denied workspaces in the browser | Cleaner sidebar | Makes the browser an authorization authority and hides actionable explanations | Rejected |
| **Bounded primary capability per workspace with handler-level re-authorization** | Complete UX affordance coverage, stable product vocabulary, and predictable latency | A workspace status is not a substitute for endpoint authorization | **Selected; see ADR-0277** |

### Addressable workspace navigation

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Keep workspace state only in React memory | Minimal code | Refresh, bookmarks, support links, and browser history do not work | Rejected |
| Hash fragments | No server fallback required | Less useful canonical URLs and weaker support-link ergonomics | Rejected |
| Full client router | Rich nested route features | Additional dependency/lifecycle surface and unnecessary authority boundary | Rejected |
| Server-rendered routing | Mature route/data integration | Violates the static React/Vite/NGINX boundary | Rejected |
| **Typed History API adapter with an NGINX SPA fallback** | Stable paths, no new runtime, bounded state, and compatibility with lazy workspaces | Requires an explicit allow-list and direct-link test | **Selected; see ADR-0278** |

### Workflow handoff URL integrity (ADR-0318)

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Change React state only during child handoffs | Minimal implementation | Leaves the address bar, refresh, support links, and browser history stale | Rejected |
| Put calculation IDs or evidence in the URL | Makes the handoff reloadable | Leaks sensitive/resource context and creates a second authority contract | Rejected |
| Add a full client router | Rich nested route lifecycle | Adds dependency and lifecycle surface beyond the static SPA requirement | Rejected |
| **Reuse one App-owned typed History API boundary for every handoff** | Canonical URL, no sensitive disclosure, one navigation policy, low overhead | Non-URL evidence still requires an authenticated reload | **Selected in ADR-0318** |

### Bounded browser error disclosure (ADR-0319)

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Render server messages unchanged | Preserves diagnostics | Unbounded untrusted disclosure and control-character leakage | Rejected |
| Status code only | Predictable and minimal | Removes actionable operator guidance | Rejected |
| Sanitizer dependency | Broad reusable policy | Supply-chain and runtime cost for a small boundary | Rejected |
| **Pure 512-character normalizer with strict UUIDv7 support references** | Deterministic, low-latency, dependency-free, and auditable | Detailed diagnostics remain in server-side evidence | **Selected in ADR-0319** |

### On-chain stablecoin asset admission (ADR-0320)

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Off-chain route allow-list only | Smallest change and easy operations | Direct Solana callers bypass it; database state is not movement authority | Rejected |
| Hard-code USDC/USDT mint addresses in the program | Very small runtime check | No governed rotation, network/issuer ambiguity, poor multi-tenant extensibility, and upgrade pressure | Rejected |
| Infer stablecoin status from symbol, metadata, freeze authority, or price | No new PDA | Metadata and market state are mutable or unavailable on-chain; unsafe false positives | Rejected |
| Per-merchant mutable asset lists in every settlement account | Flexible tenant policy | Larger hot accounts, mutable shared state, extra contention, and more account metas | Rejected |
| **Mint-scoped immutable `AssetConfig` PDA plus server-verified route evidence** | Solana-native final authority, bounded account read, governed classification, deterministic compiler, and audit trail | Requires additive ABI/IDL update, migration, and Linux validator evidence | **Selected in ADR-0320** |

### Server-owned checkout policy binding

**Criteria:** prevent operators from typing authority identifiers into a
payment flow; bind policy data to the authenticated tenant's persisted signed
evidence; preserve exact review evidence; avoid an extra hot-path round trip;
and fail closed for stale or substituted policy inputs.

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Keep policy fields in the primary form | No API change | Copy/paste errors, technical UX, and a caller-controlled authority proposal | Rejected |
| Hide the same caller fields in an advanced section | Smaller primary surface | Still permits stale or substituted policy inputs | Rejected |
| Browser derives policy from attestation JSON | Removes manual typing | Browser becomes part of the authority boundary | Rejected |
| Separate policy lookup before compilation | Explicit lookup | Extra round trip and a lookup/compile race | Rejected |
| **Derive policy values inside each authenticated compile boundary from persisted attestation evidence** | One source of truth, fewer round trips, simpler UI, and fail-closed replay semantics | Requires a versioned request-schema change | **Selected in ADR-0279** |

### Browser credential presentation

**Criteria:** preserve the memory-only session boundary, prevent bearer-token
disclosure through the DOM or support artifacts, keep authenticated request
clients typed, and retain an honest local-development path.

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Render the token in a read-only field | Makes the request dependency visible | Credential disclosure without an authorization benefit | Rejected |
| Mask the token in a password field | Reduces casual shoulder-surfing | Still exposes a credential to the DOM and copy/paste flows | Rejected |
| Pass only a boolean session flag to every workspace | Strongest presentation boundary | Requires a broad request-client contract refactor | Deferred |
| **Pass the token in memory and render only bounded session status** | Removes disclosure while preserving the current typed request boundary | Requires a repository-wide presentation contract | **Selected in ADR-0280** |

### Quote retry-reference presentation

**Criteria:** preserve exact idempotent retries, rotate request identity when
semantic facts change, avoid exposing protocol identifiers in the primary tax
workflow, and keep the key out of persistent browser storage.

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Keep an editable UUID field | Explicit and inspectable | Technical copy/paste errors and accidental reuse after edits | Rejected |
| Generate a new key for every submit | Minimal state | Retries can create duplicate durable calculations | Rejected |
| Persist the key in browser storage | Survives reloads | Retains replay identity and complicates session isolation | Rejected |
| **In-memory CSPRNG key regenerated on semantic input change** | Stable retries, safe input mutation, no persistence, and plain-language UX | Reload starts a new request identity | **Selected in ADR-0281** |

### Primary UI language and technical disclosure

**Criteria:** keep high-consequence workflows understandable, preserve exact
audit and support evidence, avoid browser authority, and support WCAG 2.2 AA
navigation and disclosure semantics.

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Show API, blockchain, and policy terms in every field | Precise for engineers | Increases operator error and cognitive load | Rejected |
| Remove identifiers and evidence from the UI | Simple first view | Prevents audit and incident response | Rejected |
| Separate administrator application | Tailored language per audience | Duplicates policy and navigation surfaces | Deferred |
| **Plain-language primary workflow with progressive technical disclosure** | Understandable tasks, auditable records, one policy surface | Requires copy and accessibility tests | **Selected; see ADR-0282** |

### Continuous revenue streaming scope

**Criteria:** preserve atomic tax and split invariants, keep the settlement hot
path bounded, support cumulative and corrective tax semantics, prevent a stream
from becoming an ungoverned token authority, and leave a credible path for a
future continuous-payout product.

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Continuous on-chain streams as the first settlement authority | Smooth partner payouts and a familiar streaming model | Mutable accrual state, epoch/finality complexity, partial cancellation, tax-window ambiguity, and a larger token-authority surface | Rejected for the first production scope |
| **Bounded atomic per-transaction split with immutable versioned plans** | Tax, vault deposit, and recipient allocation share one conservation boundary; bounded accounts and deterministic evidence | Payouts occur per settled payment rather than continuously | **Selected for the first production settlement scope; see ADR-0283** |
| Off-chain accrual ledger with periodic batch payout | Flexible schedules and fewer on-chain accounts | Reintroduces custody/reconciliation ambiguity and delays the authoritative split | Rejected as a settlement authority; may be an integration pattern after finalized evidence |

The word “real-time” in product material therefore means a bounded split at
the authorized transaction boundary, not an unbounded continuous transfer
stream. A future stream context must be separately designed, governed, and
audited.

### Finalized revenue-streaming accrual and release claims

The evaluation material asks for partner streaming, while the current product
must preserve one authoritative settlement boundary. ADR-0285 selects a pure
domain ledger that consumes finalized split allocations and prepares bounded,
single-use release claims. It deliberately does not add a direct transfer
endpoint or expose an unfinished UI.

| Alternative | Strengths | Risks | Decision |
| --- | --- | --- | --- |
| Reuse atomic settlement for each payout | Minimal new code | Does not model accruals or payout windows | Rejected |
| Off-chain balance with direct transfer | Flexible schedule | Database becomes an unsafe movement authority | Rejected |
| Mutable on-chain stream first | Continuous payout semantics | Larger authority and tax-correction surface | Rejected for the first implementation slice |
| **Finalized accrual ledger plus governed release claim** | Finality-bound, replayable, bounded, adapter-friendly | Requires a new context and later chain/UI adapters | **Selected; see ADR-0285** |

The chain boundary is deliberately narrower than the off-chain ledger. A
bounded plan PDA, finalized-accrual references, and single-use claim PDA are
selected over reusing the generic vault claim or introducing a continuous
mutable stream. The program enforces destination, digest, nonce, cap, and
vault-authority bindings; the proof adapter remains responsible for checking
finalized chain evidence before the operator signs an accrual or claim.
See ADR-0286.

Revenue-stream plans bind their immutable chain route at proposal time using
ADR-0287; this prevents route and account substitution during later claim
compilation.

### Validator-backed revenue-stream movement conformance

Domain and synthetic RPC tests do not prove the deployed SBF artifact, Anchor
account constraints, vault-authority CPI signing, or real SPL conservation.
ADR-0301 therefore selects a pinned Agave validator case covering finalized
accruals, a single-use claim, duplicate rejection, and cancellation. Its
stable evidence label is a release-bundle requirement; local Docker success
does not close the independent Linux/amd64 gate.

Browser E2E execution uses an isolated Compose project with an ephemeral
database and no host-port publication; see ADR-0288. This preserves real
service boundaries without reusing developer state or weakening migration
checksum enforcement.

Revenue-stream signer identity uses an immutable plan subject plus the shared
wallet-proof boundary with context-qualified roles; see ADR-0289. Browser
public keys and merchant-side account topology remain forbidden inputs.

### Finalized revenue-accrual proof boundary

The revenue storage model must not treat a caller-constructed accrual as proof
that its source settlement reached Solana finalized commitment.

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Storage accepts a structurally valid accrual | Small API surface | Caller can self-attest finality and amounts | Rejected |
| RPC call inside the revenue domain | Direct chain visibility | Breaks pure-domain replay, adds hot-path network variance, and couples tax/revenue logic to providers | Rejected |
| API trusts provider JSON directly | Low implementation effort | Bypasses bounded indexer validation, provider agreement, and gap handling | Rejected |
| **Typed finalized indexer projection mapped through a pure proof adapter** | Explicit finality contract, deterministic tests, bounded application boundary | Requires the indexer/reconciliation worker integration | **Selected in ADR-0290** |

### Tax-fact normalization on the hot path

**Criteria:** preserve exact FX normalization and replay binding, avoid an
allocation in the common no-FX settlement path, keep the calculation API
read-only, and make the optimization observable through a focused invariant
test and a representative benchmark.

| Alternative | Strengths | Risks / trade-offs | Decision |
| --- | --- | --- | --- |
| Clone normalized facts for every calculation | Simple ownership model and one code path | Copies vectors and strings for every no-FX calculation; directly increases hot-path latency | Rejected |
| Always allocate a separate normalized facts structure | Makes conversion ownership explicit | Pays the allocation cost even when no conversion is present | Rejected |
| **Return `Cow` and borrow validated facts when no FX conversion is present** | Zero-copy common path, owned normalized facts only when conversion is required, one semantic boundary | Adds a lifetime-aware return type and an invariant test | **Selected; see ADR-0284** |
| Maintain separate no-FX and FX calculation implementations | Potentially minimal overhead in each branch | Duplicated tax semantics and higher drift/audit risk | Rejected |
### Finalized revenue-accrual reconciliation boundary (ADR-0291)

| Criterion | Browser/API supplied allocation | Event-supplied allocation | Storage SQL arithmetic | Typed bounded application assembler |
| --- | --- | --- | --- | --- |
| Authority safety | Fail | Fail | Partial | Pass |
| Deterministic replay | Weak | Partial | Partial | Pass |
| Shared allocation semantics | N/A | N/A | Risk of drift | Reuses canonical split-plan crate |
| Latency isolation | Partial | Partial | Weak | Pass; outside signing path |
| Selected | No | No | No | Yes |

The selected boundary is recorded in ADR-0291.

### Durable revenue-accrual reconciliation jobs (ADR-0292)

| Criterion | Re-scan chain | Hold indexer checkpoint | External broker only | PostgreSQL job in projection transaction |
| --- | --- | --- | --- | --- |
| No evidence loss after checkpoint | Partial | Pass | Partial | Pass |
| Isolation from indexing | Weak | Fail | Pass | Pass |
| Atomic link to indexed evidence | Weak | Partial | Fail | Pass |
| Existing operational model | Partial | Partial | New dependency | Reuses bounded leases and PostgreSQL |
| Selected | No | No | No | Yes |

The selected boundary is recorded in ADR-0292.

### Revenue-stream signed-message verification (ADR-0293)

| Criterion | Browser-supplied digest | Request-field rebuild | Generic transaction inspection | Typed actual-message verifier |
| --- | --- | --- | --- | --- |
| Binds proof to wallet-signed bytes | Fail | Partial | Partial | Pass |
| Prevents extra instructions and account substitution | Fail | Partial | Weak | Pass |
| Keeps revenue submission bounded | Unknown | Partial | Weak | Pass |
| Reuses existing coordinator contract | No | No | No | Pass |
| Selected | No | No | No | Yes |

The selected boundary is recorded in ADR-0293.

### Server-owned revenue payout-destination registry (ADR-0294)

| Criterion | Browser-supplied account | ATA derivation in signing path | Reused merchant destination | Unverified database record | Dedicated finalized-verified route |
| --- | --- | --- | --- | --- | --- |
| Account-substitution resistance | Fail | Partial | Fail | Fail | Pass |
| Hot-path latency isolation | Partial | Fail | Pass | Pass | Pass |
| Recipient/mint/token binding | Weak | Partial | Fail | Weak | Pass |
| Replayable operational evidence | Weak | Weak | Partial | Fail | Pass |
| Token-2022 compatibility boundary | Weak | Weak | Partial | Fail | Pass |
| Selected | No | No | No | No | **Yes; see ADR-0294** |

### Server-owned revenue-plan reference selection (ADR-0295)

| Criterion | Raw browser digests/keys | Client-side digest selection | Generic search service | Server-owned UUIDv7 catalog |
| --- | --- | --- | --- | --- |
| Business-readable operator workflow | Fail | Partial | Pass | **Pass** |
| Tenant/lifecycle authority | Weak | Weak | Partial | **Pass** |
| Canonical digest derivation | Fail | Fail | Partial | **Pass** |
| No chain RPC in create path | Pass | Pass | Partial | **Pass** |
| Bounded latency and failure surface | Pass | Pass | Weak | **Pass** |
| Selected | No | No | No | **Yes; see ADR-0295** |

### Production account and session menu (ADR-0297)

| Criterion | Inert avatar | Modal dialog | Dedicated account route | Native disclosure menu |
| --- | --- | --- | --- | --- |
| No false affordance | Fail | Pass | Pass | **Pass** |
| Keyboard and screen-reader behavior | Fail | Partial | Pass | **Pass** |
| Small failure surface | Pass | Partial | Weak | **Pass** |
| Does not create a client authority boundary | Pass | Pass | Pass | **Pass** |
| Selected | No | No | Deferred | **Yes; see ADR-0297** |

### Material 3 Expressive visual system (ADR-0298)

| Criterion | Literal component colors | Arbitrary third-party theme | Repository Material roles | Full UI framework |
| --- | --- | --- | --- | --- |
| Semantic state meaning | Fail | Partial | **Pass** | Partial |
| Light/dark accessibility review | Weak | Partial | **Pass** | Partial |
| Visual density and hierarchy control | Partial | Partial | **Pass** | Partial |
| Runtime and dependency cost | Pass | Partial | **Pass** | Fail |
| Wallet/failure UX ownership | Pass | Weak | **Pass** | Partial |
| Selected | No | No | **Yes; see ADR-0298** | Deferred |

### Semantic color-token migration (ADR-0300)

| Criterion | Final override only | Component-local variables | Third-party theme | Repository-wide semantic token contract |
| --- | --- | --- | --- | --- |
| Prevents literal-color drift | Weak | Partial | Partial | **Pass** |
| Theme consistency | Partial | Partial | Pass | **Pass** |
| State meaning remains auditable | Partial | Partial | Weak | **Pass** |
| Runtime/dependency cost | Pass | Pass | Weak | **Pass** |
| Selected | No | No | No | **Yes; see ADR-0300** |

### Tonal surfaces and no decorative gradients (ADR-0310)

| Criterion | Component gradients | Arbitrary component colors | Third-party Material theme | Repository Material roles with flat tonal surfaces |
| --- | --- | --- | --- | --- |
| Semantic state meaning | Weak | Fail | Partial | **Pass** |
| Light/dark theme auditability | Partial | Fail | Partial | **Pass** |
| Operator visual hierarchy | Partial | Weak | Partial | **Pass** |
| Runtime/dependency cost | Pass | Pass | Weak | **Pass** |
| Selected | No | No | No | **Yes; see ADR-0310** |

### Solana Aurora Material 3 role palette (ADR-0311)

| Criterion | Existing lavender/pink roles | Browser-generated dynamic color | Tenant-provided colors | Fixed Solana Aurora role matrix |
| --- | --- | --- | --- | --- |
| Product identity and visual coherence | Partial | Partial | Partial | **Pass** |
| Deterministic accessibility evidence | Pass | Weak | Fail | **Pass** |
| State-role meaning and auditability | Pass | Partial | Fail | **Pass** |
| Runtime/dependency and latency cost | Pass | Partial | Pass | **Pass** |
| Selected | No | No | No | **Yes; see ADR-0311** |

### Document-payment outcome-first language (ADR-0312)

| Criterion | Technical primary copy | Hide technical evidence | Tooltip-only explanation | Plain primary copy with inspectable evidence |
| --- | --- | --- | --- | --- |
| Finance/operator task clarity | Weak | Pass | Partial | **Pass** |
| Audit and support evidence | Pass | Fail | Weak | **Pass** |
| Keyboard, screen-reader, and localization quality | Partial | Pass | Fail | **Pass** |
| Authority and settlement semantics unchanged | **Pass** | Pass | Pass | **Pass** |
| Selected | No | No | No | **Yes; see ADR-0312** |

### Compact protection context (ADR-0313)

| Criterion | Repeated shell surfaces | Remove protection copy | Account-menu-only protection | One expandable page-context disclosure |
| --- | --- | --- | --- | --- |
| Workspace scanability | Fail | **Pass** | Pass | **Pass** |
| Discoverability of responsibility and custody | Pass | Fail | Partial | **Pass** |
| Separation of live authorization from explanatory copy | Weak | Pass | Partial | **Pass** |
| Keyboard and screen-reader access | Partial | Pass | Partial | **Pass** |
| Selected | No | No | No | **Yes; see ADR-0313** |

### Non-custodial wallet source contract (ADR-0314)

| Criterion | Provider policy only | Ban all key-related words | Bundle-only scan | Scoped source contract plus runtime wallet tests |
| --- | --- | --- | --- | --- |
| Detects request-boundary regressions early | Weak | **Pass** | Partial | **Pass** |
| False-positive control for public/server-owned keys | Pass | Fail | Partial | **Pass** |
| Diagnostic ownership in code review | Partial | Weak | Weak | **Pass** |
| Runtime wallet authority preserved | Pass | Pass | Pass | **Pass** |
| Selected | No | No | No | **Yes; see ADR-0314** |

### Executable tax-responsibility language boundary (ADR-0315)

| Criterion | Terms/counsel only | One global disclaimer | Ungoverned page copy | Executable context-specific language contract |
| --- | --- | --- | --- | --- |
| Prevents misleading compliance claims | Weak | Partial | Weak | **Pass** |
| Preserves calculation/attestation/settlement distinctions | Partial | Weak | Partial | **Pass** |
| Plain-language task clarity | Partial | Partial | Pass | **Pass** |
| Reviewable failure diagnostics | Fail | Partial | Fail | **Pass** |
| Selected | No | No | No | **Yes; see ADR-0315** |

### Versioned rule-release provenance readiness (ADR-0316)

| Criterion | Arbitrary JSON | Browser-only validation | Separate relational owner tables first | Versioned envelope plus Rust/DB publication gates |
| --- | --- | --- | --- | --- |
| Prevents incomplete publication | Fail | Partial | Pass | **Pass** |
| Direct API/SQL bypass resistance | Fail | Fail | Pass | **Pass** |
| Immutable digest and replay binding | Partial | Partial | Partial | **Pass** |
| Bounded evolution and migration scope | Partial | Partial | Weak | **Pass** |
| Selected | No | No | Deferred | **Yes; see ADR-0316** |

### Versioned local display preferences (ADR-0302)

| Criterion | Reset on reload | Persist full app state | Server-saved profile | Presentation-only local schema |
| --- | --- | --- | --- | --- |
| Operator continuity | Fail | Pass | Pass | **Pass** |
| Credential and authority safety | Pass | Fail | Partial | **Pass** |
| Reload latency/dependency cost | Pass | Pass | Weak | **Pass** |
| Cross-device continuity | Fail | Fail | **Pass** | Partial |
| Selected | No | No | Deferred | **Yes; see ADR-0302** |

### Exact-commit release browser evidence (ADR-0303)

| Criterion | Pull-request browser run only | Release run without retention | Raw retained result | Exact-commit run, binding, and checksum |
| --- | --- | --- | --- | --- |
| Evidence matches packaged source | Fail | Pass | Weak | **Pass** |
| Post-run reviewability | Partial | Fail | Pass | **Pass** |
| Missing/failing UI gate fails closed | Partial | Pass | Partial | **Pass** |
| Release artifact integrity | Weak | Weak | Partial | **Pass** |
| Added release latency and storage | Pass | Partial | Partial | Partial; bounded by the existing suite |
| Selected | No | No | No | **Yes; see ADR-0303** |

### Material role contrast contract (ADR-0304)

| Criterion | Manual review only | Browser axe only | Third-party color dependency | Dependency-free role contract plus axe |
| --- | --- | --- | --- | --- |
| Deterministic merge gate | Fail | Partial | Pass | **Pass** |
| Covers unrendered role pairs | Fail | Fail | Partial | **Pass** |
| Runtime/dependency cost | Pass | Pass | Partial | **Pass** |
| Expressive palette ownership | Pass | Pass | Weak | **Pass** |
| Selected | No | No | No | **Yes; see ADR-0304** |

### One-command Tax Engine local gate (ADR-0305)

| Criterion | Separate commands only | Opaque shell script | Parallel all gates | Ordered Make orchestration |
| --- | --- | --- | --- | --- |
| Reviewer can reproduce the complete local gate | Partial | Pass | Pass | **Pass** |
| Failure attribution and evidence ownership | Pass | Weak | Weak | **Pass** |
| Shared-resource safety | Pass | Pass | Fail | **Pass** |
| Validator capability failure remains visible | Pass | Partial | Partial | **Pass** |
| Selected | No | No | No | **Yes; see ADR-0305** |

### Collapsed advanced evidence disclosures (ADR-0306)

| Criterion | Keep expanded | Remove from form | Custom modal | Native collapsed disclosure |
| --- | --- | --- | --- | --- |
| Outcome-first operator hierarchy | Fail | Pass | Pass | **Pass** |
| Required evidence remains available | Pass | Fail | Pass | **Pass** |
| Accessibility and focus failure surface | Pass | Pass | Weak | **Pass** |
| Runtime and implementation cost | Pass | Pass | Weak | **Pass** |
| Selected | No | No | No | **Yes; see ADR-0306** |

### Project-scoped local reset (ADR-0307)

| Criterion | Destructive `make down` | Raw operator Docker commands | Automatic checksum repair | Explicit scoped reset |
| --- | --- | --- | --- | --- |
| Preserves normal local evidence | Fail | Pass | Pass | **Pass** |
| Fails closed on schema drift | Partial | Pass | Fail | **Pass** |
| Prevents cross-project deletion | Partial | Weak | Pass | **Pass** |
| Discoverability and reproducibility | Pass | Fail | Partial | **Pass** |
| Selected | No | No | No | **Yes; see ADR-0307** |

### Exact-commit authenticated integration evidence (ADR-0308)

| Criterion | Separate CI job only | Selected-drill subset | Run without retention | **Run, bind, checksum, and retain** |
| --- | --- | --- | --- | --- |
| Evidence belongs to packaged source | Fail | Partial | Pass | **Pass** |
| Covers authenticated multi-service invariants | Pass | Partial | Pass | **Pass** |
| Reviewable after release job | Pass | Pass | Fail | **Pass** |
| Failure remains visible and fail-closed | Pass | Partial | Weak | **Pass** |
| Bounded operational cost | Pass | **Pass** | Pass | Pass |
| Selected | No | No | No | **Yes; see ADR-0308** |

### Finalized asset-configuration route evidence (ADR-0321)

| Criterion | Four-account topology only | Database asset flag | Hot-path RPC verification | **Finalized PDA proof with versioned evidence** |
| --- | --- | --- | --- | --- |
| Proves the on-chain asset admission authority | Fail | Fail | Pass | **Pass** |
| Preserves quote/compile/sign latency budget | Pass | Pass | Fail | **Pass** |
| Rejects stale, disabled, or substituted assets | Fail | Fail | Partial | **Pass** |
| Replay and audit clarity | Partial | Weak | Partial | **Pass** |
| Bounded account/data surface | Pass | Pass | Weak | **Pass** |
| Selected | No | No | No | **Yes; see ADR-0321** |

### Route evidence-version migration (ADR-0322)

| Criterion | Leave legacy active | Rewrite legacy evidence | Delete legacy rows | **Suspend and re-verify with expand/contract metadata** |
| --- | --- | --- | --- | --- |
| Preserves audit history | Pass | Fail | Fail | **Pass** |
| Prevents unsafe legacy signing | Fail | Partial | Pass | **Pass** |
| Rolling deployment compatibility | Pass | Partial | Fail | **Pass** |
| Reproducible operator backfill | Weak | Fail | Fail | **Pass** |
| Recovery/rollback safety | Partial | Fail | Fail | **Pass** |
| Selected | No | No | No | **Yes; see ADR-0322** |

### Typed asset-admission projection (ADR-0325)

| Criterion | JSON evidence only | Separate asset table | Raw account bytes | **Typed route columns plus evidence** |
| --- | --- | --- | --- | --- |
| Direct SQL invariant strength | Weak | Pass | Partial | **Pass** |
| Authority and tenant/version binding | Pass | Partial | Partial | **Pass** |
| Forensic replay fidelity | Pass | Partial | **Pass** | **Pass** |
| Hot-path latency and dependency cost | Pass | Partial | Pass | **Pass** |
| Rolling migration safety | Pass | Weak | Partial | **Pass** |
| Selected | No | No | No | **Yes; see ADR-0325** |

### Material 3 Expressive operational workspace composition (ADR-0323)

| Criterion | Everything expanded | Search-only navigation | Colored per-item badges | **Grouped native disclosure and active-workspace hierarchy** |
| --- | --- | --- | --- | --- |
| Route discoverability | Pass | Fail | Pass | **Pass** |
| Operator scan cost | Fail | Pass | Partial | **Pass** |
| Authorization clarity | Partial | Partial | Weak | **Pass** |
| Palette/state discipline | Pass | Pass | Fail | **Pass** |
| Keyboard and assistive technology support | Pass | Partial | Partial | **Pass** |
| Runtime and bundle cost | Pass | Partial | Pass | **Pass** |
| Selected | No | No | No | **Yes; see ADR-0323** |

### Material 3 Expressive semantic state hierarchy (ADR-0326)

| Criterion | Per-row badges | Repeated availability text | Hide state entirely | **Accessible row context plus active-route emphasis** |
| --- | --- | --- | --- | --- |
| Calm navigation scan path | Fail | Fail | Pass | **Pass** |
| RBAC/PBAC/ABAC explanation | Partial | Pass | Fail | **Pass** |
| Stale-state and false-approval risk | Weak | Partial | Pass | **Pass** |
| Light/dark palette discipline | Weak | Partial | Pass | **Pass** |
| Keyboard and screen-reader support | Partial | Pass | Fail | **Pass** |
| Runtime and bundle cost | Pass | Pass | Pass | **Pass** |
| Selected | No | No | No | **Yes; see ADR-0326** |

### Single-open Material 3 navigation group (ADR-0337)

| Criterion | Accumulating groups | Close every group | Search-only palette | **One current-workspace group** |
| --- | --- | --- | --- | --- |
| Calm navigation scan path | Fail over time | Pass | Pass | **Pass** |
| Current route context | Pass | Fail | Partial | **Pass** |
| Route discoverability | Pass | Pass | Partial | **Pass** |
| Keyboard and native disclosure support | Pass | Pass | Partial | **Pass** |
| Authorization/presentation separation | Pass | Pass | Partial | **Pass** |
| Runtime and bundle cost | Pass | Pass | Partial | **Pass** |
| Selected | No | No | No | **Yes; see ADR-0337** |

### Forced-colors accessibility layer (ADR-0338)

| Criterion | Aurora palette only | Manual high-contrast product theme | Disable forced colors | **OS system-color role mapping** |
| --- | --- | --- | --- | --- |
| Honors user accessibility settings | Weak | Partial | Fail | **Pass** |
| Single audited application palette | Pass | Fail | Pass | **Pass** |
| Focus/error/disabled state visibility | Partial | Pass | Weak | **Pass** |
| Runtime and bundle cost | Pass | Partial | Pass | **Pass** |
| Authorization and financial-path isolation | Pass | Pass | Pass | **Pass** |
| Selected | No | No | No | **Yes; see ADR-0338** |

### Polite workspace navigation announcement (ADR-0339)

| Criterion | Heading/focus only | Assertive full-state region | Browser title mutation | **Polite bounded workspace status** |
| --- | --- | --- | --- | --- |
| Screen-reader context reliability | Partial | Pass | Partial | **Pass** |
| Interruption/noise control | Pass | Fail | Partial | **Pass** |
| Sensitive/financial data isolation | Pass | Fail | Partial | **Pass** |
| Runtime and bundle cost | Pass | Pass | Pass | **Pass** |
| Selected | No | No | No | **Yes; see ADR-0339** |

### Versioned shell and navigation localization (ADR-0340)

| Criterion | English plus formatting | Runtime machine translation | Ad-hoc component strings | **Typed versioned shell catalog** |
| --- | --- | --- | --- | --- |
| Deterministic terminology | Partial | Fail | Partial | **Pass** |
| Locale coverage and fallback | Partial | Pass | Fail | **Pass** |
| Runtime/privacy/latency cost | Pass | Fail | Pass | **Pass** |
| Financial and authorization isolation | Pass | Partial | Partial | **Pass** |
| Selected | No | No | No | **Yes; see ADR-0340** |

### Versioned shell chrome and session localization (ADR-0341)

| Criterion | English shell controls | Runtime machine translation | Ad-hoc component strings | **Typed versioned chrome/session catalog** |
| --- | --- | --- | --- | --- |
| Consistent operator language | Partial | Partial | Fail | **Pass** |
| Credential and identity isolation | Pass | Partial | Partial | **Pass** |
| Deterministic runtime/latency cost | Pass | Fail | Pass | **Pass** |
| Locale fallback and bounded keys | Pass | Partial | Fail | **Pass** |
| Selected | No | No | No | **Yes; see ADR-0341** |

### Versioned quote workspace localization (ADR-0342)

| Criterion | English-only workspace | Runtime machine translation | Independent component strings | **Typed versioned quote catalog** |
| --- | --- | --- | --- | --- |
| Deterministic financial terminology | Partial | Fail | Partial | **Pass** |
| Financial/rule/evidence value isolation | Pass | Partial | Partial | **Pass** |
| Locale fallback and testability | Partial | Partial | Fail | **Pass** |
| Hot-path runtime and bundle cost | Pass | Fail | Pass | **Pass** |
| Selected | No | No | No | **Yes; see ADR-0342** |

### Versioned document-quote localization (ADR-0343)

| Criterion | English-only invoice flow | Runtime machine translation | Aggregate catalog reuse | **Typed document-quote catalog** |
| --- | --- | --- | --- | --- |
| Deterministic tax/legal terminology | Partial | Fail | Partial | **Pass** |
| Invoice-line and evidence semantics | Pass | Partial | Fail | **Pass** |
| Financial/document value isolation | Pass | Partial | Partial | **Pass** |
| Locale fallback and testability | Partial | Partial | Partial | **Pass** |
| Selected | No | No | No | **Yes; see ADR-0343** |

### Versioned document-checkout localization (ADR-0344)

| Option | Signing ambiguity | Authority/value isolation | Reusable wallet consistency | Offline determinism | Decision |
| --- | --- | --- | --- | --- | --- |
| Keep checkout in English | Fail | Pass | Partial | Pass | Rejected |
| Runtime machine translation | Fail | Partial | Partial | Fail | Rejected |
| Reuse the document-quote catalog | Partial | Partial | Fail | Pass | Rejected |
| **Typed checkout catalog plus reusable wallet catalog** | **Pass** | **Pass** | **Pass** | **Pass** | **Selected; see ADR-0344** |

### Versioned aggregate-checkout localization (ADR-0345)

| Option | Signing clarity | Authority/value isolation | Determinism | Decision |
| --- | --- | --- | --- | --- |
| Keep aggregate checkout in English | Fail for localized operators | Pass | Pass | Rejected |
| Runtime machine translation | Fail | Partial | Fail | Rejected |
| Reuse the document-checkout catalog | Partial | Partial | Pass | Rejected |
| **Typed aggregate-checkout catalog** | **Pass** | **Pass** | **Pass** | **Selected; see ADR-0345** |

### Versioned Treasury and vault-release localization (ADR-0346)

| Option | Operator clarity | Authority/value isolation | Determinism | Decision |
| --- | --- | --- | --- | --- |
| Keep Treasury in English | Fails at a governed money-movement boundary | Pass | Pass | Rejected |
| Runtime machine translation | Unstable operational terminology | Partial | Fail | Rejected |
| Reuse aggregate checkout catalog | Partial; lifecycle and evidence semantics differ | Partial | Pass | Rejected |
| **Typed Treasury-specific catalog** | **Pass** | **Pass** | **Pass** | **Selected; see ADR-0346** |

### Versioned refund-claim localization at the wallet boundary (ADR-0347)

| Option | Signing clarity | Authority/value isolation | Determinism | Decision |
| --- | --- | --- | --- | --- |
| Keep refund claims in English | Fails at a two-signature money boundary | Pass | Pass | Rejected |
| Runtime machine translation | Unstable custody and recovery wording | Partial | Fail | Rejected |
| Reuse checkout catalog | Partial; refund roles and proof differ | Partial | Pass | Rejected |
| **Typed refund-claim catalog** | **Pass** | **Pass** | **Pass** | **Selected; see ADR-0347** |

### Versioned refund-execution localization at the final release boundary (ADR-0348)

| Option | Signing clarity | Authority/value isolation | Determinism | Decision |
| --- | --- | --- | --- | --- |
| Keep final release in English | Fails at an irreversible release boundary | Pass | Pass | Rejected |
| Runtime machine translation | Unstable signing and recovery wording | Partial | Fail | Rejected |
| Reuse the refund-claim catalog | Partial; final-approver semantics differ | Partial | Pass | Rejected |
| **Typed refund-execution catalog** | **Pass** | **Pass** | **Pass** | **Selected; see ADR-0348** |

### Versioned refund-cancellation localization at the governance boundary (ADR-0349)

| Option | Governance clarity | Authority/value isolation | Determinism | Decision |
| --- | --- | --- | --- | --- |
| Keep cancellation in English | Fails for localized operators at an approval boundary | Pass | Pass | Rejected |
| Runtime machine translation | Unstable authorization and recovery wording | Partial | Fail | Rejected |
| Reuse refund-claim catalog | Partial; cancellation authority and evidence semantics differ | Partial | Pass | Rejected |
| **Typed refund-cancellation catalog** | **Pass** | **Pass** | **Pass** | **Selected; see ADR-0349** |

### Stable semantic navigation glyphs (ADR-0326)

| Criterion | First-letter avatars | Per-item colored illustrations | Icon font dependency | **Fixed monochrome inline SVG glyphs** |
| --- | --- | --- | --- | --- |
| Task recognition | Partial | Pass | Pass | **Pass** |
| Status/color confusion risk | Partial | Fail | Partial | **Pass** |
| Accessibility and DOM stability | Pass | Partial | Partial | **Pass** |
| Offline/static SPA portability | Pass | Pass | Partial | **Pass** |
| Dependency and runtime cost | Pass | Partial | Partial | **Pass** |
| Selected | No | No | No | **Yes; see ADR-0326** |

### Fail-closed public release claims (ADR-0327)

| Criterion | Reviewer-only prose | Block all release-candidate evidence | Automatic license choice | **Executable negative contract with mutation tests** |
| --- | --- | --- | --- | --- |
| Prevents unsupported production claims | Weak | Pass | Partial | **Pass** |
| Preserves useful local/release evidence | Pass | Fail | Pass | **Pass** |
| Avoids unauthorized legal decisions | Pass | Pass | Fail | **Pass** |
| Detects workflow deployment authority drift | Weak | Pass | Weak | **Pass** |
| Reproducible in clean checkout | Partial | Partial | Pass | **Pass** |
| Selected | No | No | No | **Yes; see ADR-0327** |

### Durable route-verification run evidence (ADR-0328)

| Criterion | Logs and metrics only | Mutable singleton row | Per-route audit row | **Bounded append-only run ledger** |
| --- | --- | --- | --- | --- |
| Durable restart evidence | Weak | Partial | Pass | **Pass** |
| Write amplification | Pass | Pass | Fail | **Pass** |
| Backfill count accuracy | Weak | Partial | Pass | **Pass** |
| Avoids becoming chain authority | Pass | Pass | Partial | **Pass** |
| Bounded retention and review | Partial | Pass | Weak | **Pass** |
| Selected | No | No | No | **Yes; see ADR-0328** |

### Approval-bound route-verification backfill mode (ADR-0329)

| Criterion | Daemon timing | Unauthenticated run-once flag | Separate backfill service | **Approval-bound one-shot mode** |
| --- | --- | --- | --- | --- |
| Explicit operator intent | Weak | Partial | Pass | **Pass** |
| Durable approval evidence | Fail | Fail | Partial | **Pass** |
| Reuses verified route path | Pass | Pass | Weak | **Pass** |
| Scope isolation from unrelated verifiers | Fail | Partial | Pass | **Pass** |
| Hot-path impact | Pass | Pass | Pass | **Pass** |
| Selected | No | No | No | **Yes; see ADR-0329** |

### Route-verification operator evidence view (ADR-0336)

| Option | Benefit | Risk / trade-off | Decision |
| --- | --- | --- | --- |
| Direct SQL/runbook only | No application work | Requires database access and is not a supportable SaaS workflow | Rejected |
| New route-management workspace | Discoverable | Adds navigation clutter and mixes provisioning with merchant workflows | Rejected |
| Merchant-scoped route list | Familiar | Leaks system-wide operational evidence across tenant boundaries | Rejected |
| **Bounded disclosure in the existing system-operator workspace** | Reuses operator authority, explicit read-only boundary, no new nav item | Adds a bounded read endpoint and evidence parser | **Selected in ADR-0336** |
| Live polling dashboard | Freshness | Adds load and can imply finality | Rejected |

### Server-owned document evidence status for checkout (ADR-0330)

| Option | Protocol clutter | Browser evidence authority | Tenant/route binding | Decision |
| --- | --- | --- | --- | --- |
| Manual raw JSON paste | High | Ambiguous | Repeated manually | Rejected |
| Raw attestation read endpoint | High | Encourages client parsing | Server-selected record | Rejected |
| Browser-selected stored evidence | Medium | Client selects evidence | Ambiguous route binding | Rejected |
| **Server-owned route-bound status read** | **Low** | **No** | **Explicit** | **Selected; see ADR-0330** |

### Canonical authorization policy catalog (ADR-0331)

| Option | Drift risk | Runtime dependency | Build determinism | Decision |
| --- | --- | --- | --- | --- |
| Independent Rust and TypeScript lists | High | Low | Pass | Rejected |
| Browser-fetched policy metadata | Medium | High | Partial | Rejected |
| TypeScript generated only from Rust | Medium | Low | Pass | Rejected |
| **Versioned JSON contract consumed by API and browser** | **Low** | **None on request path** | **Pass** | **Selected; see ADR-0331** |

### Server-decision-only browser authorization affordances (ADR-0332)

| Option | Authority duplication | Missing-decision behavior | UI clarity | Decision |
| --- | --- | --- | --- | --- |
| Reconstruct authorization from browser role/scope/attribute fields | High | Optimistic or locally inferred | Misleading under stale context | Rejected |
| Hide every protected workspace until all data is present | None | Always unavailable | Loses denied versus pending distinction | Rejected |
| **Render the exact server decision; otherwise remain indeterminate** | **None** | **Fail closed with recovery state** | **Explicit and honest** | **Selected; see ADR-0332** |

### Capability decision and reason consistency (ADR-0333)

| Option | Contract safety | Browser authority | Failure behavior | Decision |
| --- | --- | --- | --- | --- |
| Accept any individually known decision/reason pair | Weak | Can render contradictory state | Ambiguous | Rejected |
| Normalize contradictory pairs in the browser | Partial | Browser repairs policy semantics | Hides server defect | Rejected |
| **Reject unknown and contradictory pairs before state creation** | **Strong** | **No inference** | **Fail closed** | **Selected; see ADR-0333** |

### Capability snapshot expiry boundary (ADR-0334)

| Option | Stale-affordance risk | Request-path cost | Recovery behavior | Decision |
| --- | --- | --- | --- | --- |
| No client expiry enforcement | High | None | None | Rejected |
| Render-time check only | Low | None | Requires a later render to refresh | Rejected |
| Fixed-interval polling | Partial | Unbounded repeated requests | Eventually refreshes independently of TTL | Rejected |
| **Server TTL boundary with one bounded refresh and render-time check** | **Low** | **One request per snapshot** | **Immediate indeterminate state, then refresh** | **Selected; see ADR-0334** |

### Material 3 Expressive quiet state hierarchy (ADR-0335)

| Criterion | Strong filled badges | Per-workspace accent colors | Hide configuration failures | **Semantic aliases with tonal hierarchy** |
| --- | --- | --- | --- | --- |
| Calm operator scan path | Fail | Partial | Pass | **Pass** |
| No random palette drift | Partial | Fail | Pass | **Pass** |
| Failure visibility and supportability | Pass | Partial | Fail | **Pass** |
| Light/dark/system determinism | Partial | Fail | Pass | **Pass** |
| Accessibility and progressive disclosure | Partial | Partial | Fail | **Pass** |
| Authority boundary unchanged | Pass | Pass | Partial | **Pass** |
| Selected | No | No | No | **Yes; see ADR-0335** |

### Material 3 Expressive Aurora v2 shell (ADR-0353)

| Criterion | Keep blue-heavy v1 | Per-workspace/status hues | Third-party theme | **Seed-derived role matrix with neutral tonal surfaces** |
| --- | --- | --- | --- | --- |
| Calm operator scan path | Partial | Fail | Partial | **Pass** |
| No random palette drift | Partial | Fail | Partial | **Pass** |
| Light/dark/system determinism | Pass | Partial | Partial | **Pass** |
| WCAG and forced-colors ownership | Pass | Weak | Partial | **Pass** |
| Runtime/dependency cost | Pass | Pass | Weak | **Pass** |
| Selected | No | No | No | **Yes; see ADR-0353** |

### Distinct tax-base sources (ADR-0361)

| Criterion | Permit repeated sources | Silently deduplicate | General expression language | **Reject duplicates; retain explicit additive sources** |
| --- | --- | --- | --- | --- |
| Deterministic meaning | Partial | Partial | Depends on new DSL | **Pass** |
| Fail-closed authoring | Fail | Fail | Partial | **Pass** |
| Tax-on-tax expressiveness | Pass | Partial | Pass | **Pass for bounded additive cases** |
| Validation and audit surface | Pass | Partial | Fail | **Pass** |
| Hot-path cost | Pass | Pass | Partial | **Pass; compile-time only** |
| Selected | No | No | No | **Yes; see ADR-0361** |

### Explicit inclusive threshold equation (ADR-0362)

| Criterion | Infer from price mode | Reject all inclusive thresholds | **Versioned `GrossExcess` equation** | Unbounded jurisdiction enum |
| --- | --- | --- | --- | --- |
| Legal meaning is explicit | Fail | Pass | **Pass** | Partial |
| Deterministic exact arithmetic | Partial | Pass | **Pass** | Partial |
| Global engine extensibility | Partial | Fail | **Pass through separately approved equations** | Partial |
| Fail-closed unsupported cases | Fail | **Pass** | **Pass** | Partial |
| Selected | No | No | **Yes; see ADR-0362** | No |

### Responsive shell popover boundary (ADR-0363)

| Criterion | Inline flex disclosure | Modal dialog | Multiple z-indexed popovers | **Bounded anchored popover with one open shell disclosure** |
| --- | --- | --- | --- | --- |
| Desktop workspace geometry | Fail; expanded forms widen the row | Pass | Partial | **Pass** |
| Narrow-screen clarity | Partial | Pass | Fail; panels overlap | **Pass** |
| Native keyboard semantics | Pass | Partial; requires focus management | Pass | **Pass** |
| Authentication and authority isolation | Pass | Pass | Pass | **Pass** |
| Implementation and runtime cost | Pass | Partial | Pass | **Pass** |
| Selected | No | No | No | **Yes; see ADR-0363** |

### Settlement-adjustment localization (ADR-0350)

| Option | Correction-review clarity | Value/authority isolation | Determinism | Decision |
| --- | --- | --- | --- | --- |
| Keep the correction workspace in English | Fails for localized finance operators | Pass | Pass | Rejected |
| Runtime machine translation | Unstable financial and recovery wording | Partial | Fail | Rejected |
| Reuse refund or checkout catalogs | Mismatched adjustment lifecycle and evidence semantics | Partial | Pass | Rejected |
| **Typed immutable settlement-adjustment catalog** | **Pass** | **Pass** | **Pass** | **Selected; see ADR-0350** |

### Replay-operations localization (ADR-0351)

| Option | Recovery decision clarity | Authority/value isolation | Determinism | Decision |
| --- | --- | --- | --- | --- |
| Keep recovery controls in English | Fails for localized operators | Pass | Pass | Rejected |
| Runtime machine translation | Unstable incident and retry semantics | Partial | Fail | Rejected |
| Reuse refund or settlement catalogs | Wrong lifecycle and evidence vocabulary | Partial | Pass | Rejected |
| **Typed immutable replay-operations catalog** | **Pass** | **Pass** | **Pass** | **Selected; see ADR-0351** |

### Vault-reconciliation localization (ADR-0352)

| Option | Release-stop clarity | Authority/value isolation | Determinism | Decision |
| --- | --- | --- | --- | --- |
| Keep reconciliation in English | Fails for localized treasury operators | Pass | Pass | Rejected |
| Runtime machine translation | Unstable incident and release-stop semantics | Partial | Fail | Rejected |
| Reuse replay or settlement catalogs | Wrong balance and reconciliation vocabulary | Partial | Pass | Rejected |
| **Typed immutable vault-reconciliation catalog** | **Pass** | **Pass** | **Pass** | **Selected; see ADR-0352** |

### Rule-governance and structured authoring localization (ADR-0354)

| Option | Review clarity | Protocol safety | Determinism | Decision |
| --- | --- | --- | --- | --- |
| Keep rule governance in English | Fails for localized operators | Pass | Pass | Rejected |
| Runtime machine translation | Unstable tax/legal wording | Partial | Fail | Rejected |
| Translate rule JSON keys and enum values | Visually uniform | Fail; mutates canonical contracts | Partial | Rejected |
| **Typed immutable presentation catalogs** | **Pass** | **Pass** | **Pass** | **Selected; see ADR-0354** |

### Revenue-streaming operator localization (ADR-0355)

| Option | Operator clarity | Protocol safety | Determinism | Decision |
| --- | --- | --- | --- | --- |
| Keep partner payouts in English | Fails for localized operators | Pass | Pass | Rejected |
| Runtime machine translation | Unstable governance and release wording | Partial | Fail | Rejected |
| Translate statuses, IDs, or request values | Superficially uniform | Fail; mutates authority inputs | Fail | Rejected |
| **Typed presentation-only catalogs** | **Pass** | **Pass** | **Pass** | **Selected; see ADR-0355** |

### Access-control workspace localization (ADR-0356)

| Option | Authorization clarity | Protocol safety | Determinism | Decision |
| --- | --- | --- | --- | --- |
| Keep access explanations in English | Fails for localized security operators | Pass | Pass | Rejected |
| Runtime machine translation | Unstable security terminology | Partial | Fail | Rejected |
| Translate policy IDs, scopes, or reason codes | Visually uniform | Fail; breaks audit and contract correlation | Fail | Rejected |
| **Typed presentation-only catalogs** | **Pass** | **Pass** | **Pass** | **Selected; see ADR-0356** |

### Shell operational localization (ADR-0357)

| Option | Operational clarity | Authority/value isolation | Determinism | Decision |
| --- | --- | --- | --- | --- |
| Leave shell diagnostics in English | Fails for multilingual operators | Pass | Pass | Rejected |
| Scattered locale conditionals | Partial | Pass | Partial | Rejected |
| Runtime machine translation | Unstable security and incident wording | Partial | Fail | Rejected |
| **Typed shell-operational presentation catalog** | **Pass** | **Pass** | **Pass** | **Selected; see ADR-0357** |

### Lazy-workspace and recovery-state localization (ADR-0358)

| Option | Recovery clarity | Information safety | Determinism | Decision |
| --- | --- | --- | --- | --- |
| Keep loading/error copy in English | Fails for localized operators | Pass | Pass | Rejected |
| Render raw chunk/error details | Partial | Fail; can disclose implementation or secrets | Partial | Rejected |
| Duplicate workspace-specific recovery catalogs | Partial | Pass | Partial | Rejected |
| **Reuse the typed shell-operational catalog** | **Pass** | **Pass** | **Pass** | **Selected; see ADR-0358** |

### Secondary rule simulation disclosure (ADR-0364)

| Option | Empty-state clarity | Keyboard/accessibility cost | Workflow discoverability | Decision |
| --- | --- | --- | --- | --- |
| Keep authoring and simulation expanded | Fail; two competing tasks | Pass | Pass | Rejected |
| Hide simulation until a rule loads | Pass | Pass | Fail; secondary review is undiscoverable | Rejected |
| Modal or custom popover | Pass | Partial; adds focus/layering state | Pass | Rejected |
| **Native collapsed details disclosure** | **Pass** | **Pass** | **Pass** | **Selected; see ADR-0364** |

### Plain-language authorization recovery copy (ADR-0365)

| Alternative | Operator clarity | Localization | Diagnostic safety | Decision |
| --- | --- | --- | --- | --- |
| Render parser/network error text directly | Fail | Weak | Weak | Rejected |
| One generic recovery sentence for every failure | Partial | Pass | Pass | Rejected; expiry needs distinct recovery guidance |
| **Typed failure key mapped by the localized shell catalog** | **Pass** | **Pass** | **Pass** | **Selected; see ADR-0365** |
| Client-visible technical diagnostics drawer | Partial | Higher surface area | Risk of response-detail leakage | Deferred |

### Typed localized authorization affordance reasons (ADR-0404)

| Alternative | Localization | Diagnostic safety | Operator recovery | Decision |
| --- | --- | --- | --- | --- |
| Render server reason strings directly | Fail | Weak | Inconsistent | Rejected |
| One generic message for every state | Pass | Pass | Weak; loses actionable context | Rejected |
| Infer a reason from translated text | Fail | Weak | Unstable | Rejected |
| **Bounded reason code mapped through the shell catalog** | **Pass** | **Pass** | **Pass** | **Selected; see ADR-0404** |

### Wallet-provider error redaction (ADR-0366)

| Alternative | Operator clarity | Information safety | Wallet authority | Decision |
| --- | --- | --- | --- | --- |
| Render provider exception text directly | Inconsistent | Weak | Pass | Rejected |
| Retry or classify provider errors in the browser | Partial | Partial | Risk of changing intent | Rejected |
| **Localized rejection copy with typed account-selection recovery** | **Pass** | **Pass** | **Pass** | **Selected; see ADR-0366** |
| Technical diagnostic drawer | Partial | Requires separate support contract | Pass | Deferred |

### Financial-workspace error redaction boundary (ADR-0367)

| Alternative | Operator clarity | Information safety | Recovery specificity | Decision |
| --- | --- | --- | --- | --- |
| Render bounded `Error.message` | Inconsistent | Fail; provenance is unknown | Partial | Rejected |
| One generic message for every failure | Partial | Pass | Fail; state-specific recovery is lost | Rejected |
| Browser-side classification of arbitrary messages | Unstable | Partial | Partial | Rejected |
| **Typed operator-facing errors with localized fallbacks** | **Pass** | **Pass** | **Pass** | **Selected; see ADR-0367** |

### Non-interruptive system-status disclosure (ADR-0368)

| Option | Interruption control | Discoverability | Accessibility semantics | Decision |
| --- | --- | --- | --- | --- |
| Assertive `role="alert"` for every system warning | Fail; competing announcements | Pass | Misrepresents context as urgent action | Rejected |
| Remove system warning content from the DOM | Pass | Fail | Fails operational transparency | Rejected |
| Polite `role="status"` inside the expandable disclosure | Partial; duplicates the navigation status region | Pass | Semantically valid but noisy in this shell | Rejected |
| **Ordinary disclosure text with no live-region role** | **Pass** | **Pass** | **Pass** | **Selected; see ADR-0368** |
| Custom live-region queue | Partial | Partial | Higher lifecycle and localization risk | Deferred |

### Explicit CI toolchain bootstrap (ADR-0369)

| Option | Reproducibility | Release evidence visibility | Supply-chain surface | Decision |
| --- | --- | --- | --- | --- |
| Trust runner preinstalls | Fail | Fail | Partial | Rejected |
| Implicit `rust-toolchain.toml` discovery | Partial | Partial | Pass | Rejected |
| Floating `stable` installation | Fail | Partial | Pass | Rejected |
| Third-party setup action without a repository contract | Partial | Pass | Partial | Rejected |
| **Explicit pinned `rustup` install plus executable contract** | **Pass** | **Pass** | **Pass** | **Selected; see ADR-0369** |

### Explicit frontend CI toolchain evidence (ADR-0370)

| Option | Version visibility | Reproducibility | Decision |
| --- | --- | --- | --- |
| Trust setup-action and Corepack defaults | Fail | Partial | Rejected |
| Infer versions from install output | Partial | Partial | Rejected |
| Floating package-manager channel | Fail | Fail | Rejected |
| **Pinned versions plus explicit executable evidence and contract** | **Pass** | **Pass** | **Selected; see ADR-0370** |

### Production-source Material color boundary (ADR-0371)

| Option | Bypass coverage | Runtime cost | Decision |
| --- | --- | --- | --- |
| Scan CSS only | Misses inline/CSS-in-JS colors | Low | Rejected |
| Broadly ban every `style` prop | Noisy and incomplete | Low | Rejected |
| Add a third-party component color system | Additional ownership/runtime surface | Medium | Rejected |
| **Scan production TypeScript/TSX for raw colors and gradients** | **Covers component source while preserving test fixtures** | **None at runtime** | **Selected; see ADR-0371** |

### Bounded operational observability dashboard (ADR-0372)

| Option | Incident triage value | Cardinality and authority safety | Decision |
| --- | --- | --- | --- |
| Keep the minimal service overview | Low | Pass | Rejected |
| Add tenant/wallet dashboard variables | High for individual debugging | Fail; identity leakage and unbounded series | Rejected |
| Use dashboard state as settlement authority | Superficially high | Fail; violates source-of-truth boundary | Rejected |
| **Fixed-cardinality read-only safety panels** | **High** | **Pass; remains non-authoritative** | **Selected; see ADR-0372** |

### Linux/amd64 conformance host preflight (ADR-0376)

| Alternative | Early failure clarity | Evidence integrity | Validator authority | Decision |
| --- | --- | --- | --- | --- |
| Inline workflow assertions only | Partial; local runs differ | Partial | Pass | Rejected |
| Allow amd64 Docker emulation on ARM hosts | Fast to start | Fail; kernel capability is unproven | Fail | Rejected |
| Independent ad-hoc `io_uring` probe | Partial; may disagree with validator | Partial | Partial | Rejected |
| **Shared native Linux/amd64 preflight plus validator evidence** | **Pass** | **Pass** | **Pass** | **Selected; see ADR-0376** |

### Dedicated Solana conformance evidence workflow (ADR-0377)

| Alternative | Evidence-owner speed | Provenance | Gate fidelity | Decision |
| --- | --- | --- | --- | --- |
| Broad CI job only | Partial | Pass | Pass | Rejected |
| Release-candidate bundle only | Low | Pass | Pass | Rejected |
| Unpinned third-party validator action or hosted RPC | High | Fail | Fail | Rejected |
| **Pinned dedicated workflow using the existing verifier** | **Pass** | **Pass** | **Pass** | **Selected; see ADR-0377** |

### Non-custodial Solidity receipt mirror (ADR-0378)

| Alternative | Solana authority isolation | EVM utility | Audit surface | Decision |
| --- | --- | --- | --- | --- |
| Documentation-only optional adapter | Pass | Fail | Low | Rejected |
| Recalculate tax or move tokens on EVM | Fail | High | Very high | Rejected |
| ERC-20 escrow or bridge | Fail | High | Very high | Rejected |
| **Governed non-custodial receipt mirror** | **Pass** | **Bounded** | **Low and separate** | **Selected; see ADR-0378** |

### Bounded taxable-base min/max (ADR-0379)

| Alternative | Semantic fidelity | Execution bound | Replay/audit | Decision |
| --- | --- | --- | --- | --- |
| Tax-result floor/cap | Fails for progressive and threshold rules | High | High | Rejected |
| Unrestricted formula/expression interpreter | Potentially high | Unbounded risk | Difficult to govern | Rejected |
| External/provider-precomputed adjusted base | Partial; shifts authority outside the rule | High | Weak | Rejected |
| **Optional exact floor/cap on the versioned linear `TaxBase`** | **High for bounded clamp regimes** | **High** | **High** | **Selected; see ADR-0379** |

### Policy issuer authority and asset-dependency evidence (ADR-0380)

| Alternative | Authority clarity | Hot-path safety | Evidence/replay | Decision |
| --- | --- | --- | --- | --- |
| One authority for policy, assets, upgrades, and custody | Low; conflated privileges create a large blast radius | Fail; compromise crosses bounded contexts | Weak attribution | Rejected |
| Database dependency rows as authority | Low; projections can be stale or substituted | Fast but unsafe activation | Weak chain/signer provenance | Rejected |
| External provider metadata as tax or redemption proof | Low; scope and legal meaning vary | Adds network dependency and ambiguity | Non-deterministic | Rejected |
| Untyped generic evidence JSON | Partial; flexible but ambiguous | Difficult to validate and bound | Weak schema evolution | Rejected |
| **Separate typed authorities with a bounded versioned evidence envelope and explicit dependency state** | **High** | **Fail-closed and bounded** | **Canonical and replayable** | **Selected; see ADR-0380** |

### Tax Engine v2 golden-vector coverage manifest (ADR-0382)

| Alternative | Scenario coverage | Drift detection | Cross-language portability | Decision |
| --- | --- | --- | --- | --- |
| Vector count only | Partial | Weak | Pass | Rejected |
| Documentation-derived list | Partial | Weak | None | Rejected |
| Second arithmetic implementation | High | Partial | Risk of semantic divergence | Rejected |
| **Explicit required IDs with exact JSON execution** | **High** | **Pass** | **Pass** | **Selected; see ADR-0382** |

### Explicit atomic display scale at the browser boundary (ADR-0383)

| Alternative | Missing-scale safety | Solana precision | Runtime/authority risk | Decision |
| --- | --- | --- | --- | --- |
| Implicit two-decimal formatter default | Fail | Fail | Low | Rejected |
| Infer scale from currency symbol or locale | Partial | Fail for arbitrary mints | Browser-dependent | Rejected |
| Fetch mint metadata while rendering | Partial | Better | Adds latency and an authority race | Rejected |
| **Required explicit scale with server-owned route precision** | **Pass** | **Pass for bound assets** | **Presentation-only and bounded** | **Selected; see ADR-0383** |

### Authoritative atomic scale in the quote contract (ADR-0384)

| Alternative | Precision safety | Backward compatibility | Settlement authority | Decision |
| --- | --- | --- | --- | --- |
| Keep the named two-decimal convention | Fail for non-2-decimal mints | High | Ambiguous | Rejected |
| Infer from ISO currency or locale | Partial and browser-dependent | Medium | Not bound to route | Rejected |
| Read mint metadata during rendering | Partial; adds RPC latency and races | Medium | Late and non-atomic | Rejected |
| **Require bounded scale in facts/results and compare with the finalized route** | **Pass** | **Explicit fail-closed migration** | **Pass** | **Selected; see ADR-0384** |

### Versioned asset-precision UI language (ADR-0385)

| Alternative | Consistency | Localization | Financial safety | Decision |
| --- | --- | --- | --- | --- |
| Keep repeated English strings | Fail | Fail | Partial | Rejected |
| Duplicate the field in each workspace catalog | Partial | Pass | Pass | Rejected |
| Put browser copy in the API contract | Partial | Partial | Risk of coupling | Rejected |
| **One browser-owned versioned semantic catalog** | **Pass** | **Pass** | **Pass** | **Selected; see ADR-0385** |

### Versioned financial-workspace error language (ADR-0386)

| Alternative | Localization | Safety-copy ownership | Diagnostic redaction | Decision |
| --- | --- | --- | --- | --- |
| Inline English strings | Fail | Partial | Partial | Rejected |
| One generic global error dictionary | Partial | Weak workspace ownership | Pass | Rejected |
| API-owned localized copy | Partial | Wrong boundary and locale coupling | Risk of leaking provider detail | Rejected |
| **Existing bounded workspace catalogs with typed dynamic values** | **Pass** | **Pass** | **Pass** | **Selected; see ADR-0386** |

### Versioned document-settlement error language (ADR-0387)

| Alternative | Recovery-action clarity | Localization ownership | Authority/redaction safety | Decision |
| --- | --- | --- | --- | --- |
| Inline English strings | Fail | Fail | Partial | Rejected |
| Reuse aggregate-checkout catalog | Partial; document evidence actions differ | Partial | Pass | Rejected |
| API-owned localized messages | Partial | Fail; couples locale to the API | Risk of leaking diagnostics | Rejected |
| **Document-checkout catalog with bounded dynamic values** | **Pass** | **Pass** | **Pass** | **Selected; see ADR-0387** |

### Versioned refund-execution recovery language (ADR-0388)

| Alternative | Two-person recovery clarity | Localization ownership | Authority/redaction safety | Decision |
| --- | --- | --- | --- | --- |
| Inline execution strings | Fail | Fail | Partial | Rejected |
| Reuse refund-claim/global checkout copy | Partial; execution has distinct signing states | Partial | Pass | Rejected |
| API-owned localized messages | Partial | Fail; couples locale to the API | Risk of leaking diagnostics | Rejected |
| **Refund-execution catalog with bounded typed messages** | **Pass** | **Pass** | **Pass** | **Selected; see ADR-0388** |

### Versioned refund-claim recovery language (ADR-0389)

| Alternative | Claim recovery clarity | Localization ownership | Authority/redaction safety | Decision |
| --- | --- | --- | --- | --- |
| Inline claim strings | Fail | Fail | Partial | Rejected |
| Reuse refund-execution/global checkout copy | Partial; claim has distinct reconciliation semantics | Partial | Pass | Rejected |
| API-owned localized messages | Partial | Fail; couples locale to the API | Risk of leaking diagnostics | Rejected |
| **Refund-claim catalog with bounded typed messages** | **Pass** | **Pass** | **Pass** | **Selected; see ADR-0389** |

### No implicit asset precision in financial workspaces (ADR-0400)

| Alternative | Financial safety | Latency/operational coupling | Explicit operator intent | Decision |
| --- | --- | --- | --- | --- |
| Keep a two-decimal default | Fail; silently misinterprets non-2-decimal assets | Pass | Fail | Rejected |
| Infer from currency or locale | Fail; currency is not mint authority | Pass | Fail | Rejected |
| Fetch mint metadata during form rendering | Partial; race and RPC dependency | Fail | Partial | Rejected |
| Rely on HTML `required` only | Partial; programmatic submission can coerce empty input | Pass | Partial | Rejected |
| **Start blank and use one exact bounded parser** | **Pass; fail closed before fact construction** | **Pass; pure local validation** | **Pass** | **Selected; see ADR-0400** |

### Complete authorization benchmark catalog (ADR-0401)

| Alternative | Coverage safety | Change detection | Operational cost | Decision |
| --- | --- | --- | --- | --- |
| Leave the benchmark's hand-maintained subset | Fail; omitted workspaces can regress unseen | Fail | Low | Rejected |
| Discover keys dynamically from each response | Partial; a server can omit a key and still self-validate | Fail; catalog shrinkage is hidden | Low | Rejected |
| Load the production catalog directly at benchmark runtime | Pass for key coverage | Partial; runtime fixture coupling can hide source drift | Medium | Rejected |
| **Exact static catalog assertions plus an executable source contract** | **Pass; every ID and decision branch is named** | **Pass; missing IDs fail before Docker load** | **Low and bounded** | **Selected; see ADR-0401** |

### Single tax-trace disclosure in the result workspace (ADR-0402)

| Alternative | Scan cost | Evidence clarity | Implementation risk | Decision |
| --- | --- | --- | --- | --- |
| Keep the duplicate trace | High | Misleading repetition | Low | Rejected |
| Remove the trace entirely | Low | Fails explainability and reviewability | Low | Rejected |
| Move the trace into technical details | Medium | Hides the operator-facing explanation | Medium | Rejected |
| **Keep one trace in “Why this tax?”** | **Low** | **Preserves plain-language explanation and exact values** | **Low** | **Selected; see ADR-0402** |

### Single persistent account context in the navigation shell (ADR-0403)

| Alternative | Visual density | Identity clarity | Authorization safety | Decision |
| --- | --- | --- | --- | --- |
| Repeat account context in sidebar, page, and account menu | Fail | Repetitive | Pass | Rejected |
| Remove persistent access state entirely | Pass | Partial | Fail; recovery state is easy to miss | Rejected |
| Move all identity and access information into a modal | Pass | Partial | Partial; adds interaction cost | Rejected |
| **Keep one persistent access state and progressive identity details** | **Pass** | **Pass** | **Pass** | **Selected; see ADR-0403** |

### Material theme-color synchronization (ADR-0405)

| Alternative | Palette consistency | First-paint behavior | Complexity | Decision |
| --- | --- | --- | --- | --- |
| Keep the legacy hard-coded browser color | Fail; outside the M3 role matrix | Pass | Low | Rejected |
| Remove browser `theme-color` metadata | Pass | Browser-defined and inconsistent | Low | Rejected |
| Reuse a component accent color | Fail; wrong semantic role | Pass | Low | Rejected |
| **Approved M3 background metadata with bounded media synchronization** | **Pass** | **Pass with safe fallback** | **Low and testable** | **Selected; see ADR-0405** |

### Material 3 mobile shell action row (ADR-0406)

| Alternative | Scan cost | Accessibility and discoverability | Workspace stability | Decision |
| --- | --- | --- | --- | --- |
| Keep the vertical stack | High; controls dominate the mobile header | Pass | Fail; pushes task content down | Rejected |
| Hide session/status controls on mobile | Low | Fail; protected recovery and service state become undiscoverable | Pass | Rejected |
| Move all controls into one overflow menu | Low | Partial; adds interaction cost and hides important state | Pass | Rejected |
| **Two-row identity plus wrapped Material action row** | **Low** | **Pass; existing controls and names remain present** | **Pass** | **Selected; see ADR-0406** |

### Evidence-bound tax applicability context (ADR-0407)

| Alternative | Applicability clarity | Replay and evidence | Latency / safety | Decision |
| --- | --- | --- | --- | --- |
| Add arbitrary applicability fields to `TaxFacts` | Partial | Fail; evidence ownership is unclear | Medium risk of digest drift | Rejected |
| Overload `regime` as product category | Fail | Partial | Low implementation cost but unsafe semantics | Rejected |
| Unbounded rule-expression language | Pass in theory | Partial | Fail; complexity and latency are unbounded | Rejected |
| External tax SaaS as authority | Partial | Partial | Fail; hot-path and provider dependency | Rejected |
| **Separate bounded applicability context and attestation** | **Pass** | **Pass** | **Pass; immutable precompiled policy** | **Selected; see ADR-0407** |

### Server-owned applicability-context persistence (ADR-0409)

| Alternative | Evidence ownership | Hot-path latency | Replay/governance | Decision |
| --- | --- | --- | --- | --- |
| Browser submits treatment and attestation JSON | Fail; caller becomes authority | Fast but unsafe | Fail | Rejected |
| External tax SaaS resolves every quote | Partial; provider becomes authority | Unbounded | Partial | Rejected |
| Full applicability graph on Solana | Strong settlement binding | Transaction/compute constrained | Partial | Rejected |
| Mutable policy rows recalculated during replay | Fail; historical meaning changes | Variable | Fail | Rejected |
| **Immutable PostgreSQL context projection plus signed attestation and bounded cache** | **Pass** | **Bounded and measurable** | **Pass** | **Selected; see ADR-0409** |

### Material state emphasis and border budget (ADR-0410)

| Alternative | Scan clarity | State semantics | Accessibility/contrast | Decision |
| --- | --- | --- | --- | --- |
| Fill every badge and outline every panel | Fail; routine content reads as alerting | Partial | Risk of competing fills | Rejected |
| Remove all status styling | Pass | Fail; operational states become harder to scan | Pass | Rejected |
| Assign an accent to each workspace | Fail; taxonomy becomes decoration | Fail; color meaning drifts | Unbounded palette | Rejected |
| **Neutral badges, explicit state fills, and tonal surfaces** | **Pass** | **Pass** | **Pass; paired M3 roles** | **Selected; see ADR-0410** |

### Versioned Solana applicability settlement binding (ADR-0411)

| Alternative | Compatibility | On-chain authority | Latency/complexity | Decision |
| --- | --- | --- | --- | --- |
| Extend the existing settlement wire layout | Fails old clients and vectors | Partial | Low code cost, high migration risk | Rejected |
| Store the context only in API metadata | Passes no message/program binding | Fails | Low | Rejected |
| Add a second post-settlement binding instruction | Payment can succeed without evidence | Fails atomicity | Medium and race-prone | Rejected |
| **Versioned instruction plus receipt-scoped binding PDA** | **Pass** | **Pass** | **Bounded extra account and bytes** | **Selected; see ADR-0411** |

### Finalized applicability-binding account verification (ADR-0412)

| Option | Finalized account proof | Settlement hot-path impact | Failure containment | Decision |
| --- | --- | --- | --- | --- |
| Trust applicability event only | **Fail** | Low | Weak; event can outlive or disagree with account state | Rejected |
| Fetch account during quote/compile/signing | Pass | **Fail** | RPC outage contaminates user authorization path | Rejected |
| Generic unbounded account read | Partial | Medium | Layout/response ambiguity | Rejected |
| Include every binding account in normal block projection | Pass | Medium/high | Inflates all indexer work | Rejected |
| Dedicated bounded finalized verifier port | **Pass** | **Pass** | Explicit fail-closed account/layout contract | **Selected; see ADR-0412** |

### Atomic applicability-binding verification projection (ADR-0413)

| Option | Replayable evidence | Legacy compatibility | Conflict safety | Decision |
| --- | --- | --- | --- | --- |
| Logs/process memory | **Fail** | Pass | Fail | Rejected |
| Mutable fields on `indexed_receipts` | Partial | **Fail** | Partial | Rejected |
| Unchecked upsert | Pass | Pass | **Fail** | Rejected |
| Dedicated serializable append-only projection | **Pass** | **Pass** | **Pass** | **Selected; see ADR-0413** |

### Bounded applicability-binding verifier worker (ADR-0414)

| Option | Concurrency safety | Recovery | Hot-path impact | Decision |
| --- | --- | --- | --- | --- |
| Scan all raw events every tick | Fail; duplicate work grows with history | Partial | Low | Rejected |
| In-memory queue | Fail across restarts and replicas | Fail | Low | Rejected |
| Hold database locks across RPC | Partial | Poor under crash | Fail | Rejected |
| Unleased bounded candidate query | Partial; duplicate provider reads | Partial | Low | Rejected |
| **Lease-fenced PostgreSQL job table** | **Pass** | **Pass; bounded retry/dead-letter** | **Pass; asynchronous** | **Selected; see ADR-0414** |

### Tenant-scoped applicability verification read model (ADR-0415)

| Option | Tenant safety | Evidence clarity | Decision |
| --- | --- | --- | --- |
| Return worker coordination rows directly | Fail; exposes implementation state | Partial | Rejected |
| Accept merchant ID from the query | Fail; caller becomes authority | Partial | Rejected |
| Return only a boolean | Partial | Fail; pending/dead-letter states disappear | Rejected |
| Read by signature only | Partial; weak intent ownership | Partial | Rejected |
| **Authenticated merchant + settlement intent + stream selector** | **Pass** | **Pass; explicit lifecycle** | **Selected; see ADR-0415** |

### Applicability-treatment settlement semantics (ADR-0416)

| Option | Determinism | Settlement clarity | Replayability | Decision |
| --- | --- | --- | --- | --- |
| Continue rejecting every non-taxable treatment | Pass | Fail; product cannot support global cases | Pass | Rejected |
| Map all non-taxable outcomes to zero tax | Fail; loses legal/reporting distinction | Fail; reverse charge and withholding are misrepresented | Partial | Rejected |
| Let API adapters reinterpret treatment after arithmetic | Fail; multiple authorities | Fail; digest omits the interpretation | Fail | Rejected |
| **Pure engine `TreatmentOutcome` with explicit collection channels** | **Pass** | **Pass** | **Pass** | **Selected; see ADR-0416** |

### Signed treatment-outcome binding (ADR-0417)

| Alternative | Evidence completeness | Legacy compatibility | Settlement hot-path impact | Decision |
| --- | --- | --- | --- | --- |
| Leave gross/tax/net as the only signed values | Fail; treatment channels are unsigned | Pass | Low | Rejected |
| Mutate the v1 canonical payload | Fail; issuer/replay ambiguity | Fail | Low | Rejected |
| Add a second outcome PDA/instruction | Partial; creates an atomicity gap | Partial | Higher account/byte cost | Rejected |
| Execute the full rule graph on Solana | Partial | Fail | Fails bounded compute goals | Rejected |
| **Versioned v2 outcome digest in the signed payload, bound by existing policy digest** | **Pass** | **Pass; dual-read rollout** | **Bounded** | **Selected; see ADR-0417** |

### Applicability-context management API (ADR-0418)

| Alternative | Tenant/authority safety | Replay/governance | Operational latency | Decision |
| --- | --- | --- | --- | --- |
| Accept applicability fields on quote requests | **Fail; caller becomes authority** | Fail | Low | Rejected |
| Keep PostgreSQL context storage without an API boundary | Partial; unreachable from a product workflow | Partial | N/A | Rejected |
| Resolve treatment through an external provider on every quote | Partial | Fail; provider availability changes replay | Unbounded | Rejected |
| **Dedicated authenticated CRUD-lite management routes over immutable storage and signed attestation verification** | **Pass** | **Pass; four-eyes lifecycle and idempotent creation** | **Bounded; off the settlement hot path** | **Selected; see ADR-0418** |

### Neutral risk taxonomy and calm active navigation (ADR-0419)

| Alternative | Semantic clarity | Scanability | Accessibility/auditability | Decision |
| --- | --- | --- | --- | --- |
| One accent per workflow risk category | Fail; taxonomy looks like incident state | Fail | Partial | Rejected |
| Remove risk labels from the product surface | Fail; action consequence becomes hidden | Pass | Fail | Rejected |
| Keep a bordered, shadowed active navigation item | Partial | Partial; selection is over-emphasized | Pass | Rejected |
| **Neutral risk taxonomy, semantic outcome fills, and tonal active selection** | **Pass** | **Pass; one visual priority** | **Pass; role-based and testable** | **Selected; see ADR-0419** |

### Applicability governance workspace (ADR-0420)

| Alternative | Workflow clarity | Authority safety | Navigation/UX cost | Decision |
| --- | --- | --- | --- | --- |
| Add treatment evidence controls to the tax-rule workbench | Fail; rule authoring and signed treatment review become one dense task | Partial; two authority models share one surface | High | Rejected |
| Build a browser-side treatment calculator or attestation editor | Fail; browser becomes a calculation or evidence authority | **Fail** | Medium | Rejected |
| Generic admin JSON console | Partial | Fail; weak role and lifecycle affordances | Low | Rejected |
| **Dedicated localized treatment-evidence workspace using server-owned signed-attestation import and lifecycle APIs** | **Pass; author/review/publish steps are explicit** | **Pass; API and capability snapshot remain authoritative** | **Bounded; one additional Rules & access workspace** | **Selected; see ADR-0420** |

### Material baseline palette and Expressive tokens (ADR-0422)

| Criterion | Keep hand-tuned Aurora | Runtime dynamic colour | Hand-copied baseline hex | **Generated from baseline seed, verified by test** |
| --- | --- | --- | --- | --- |
| Matches product direction (default Material, no teal) | No | Partly | Yes | **Yes** |
| Provenance and regeneration | Weak | None (non-deterministic) | None | **Pinned seed, variant, library** |
| Contrast auditability | Static | Unaudited | Static | **Static, 30 pairs at 4.5:1** |
| Evidence/screenshot determinism | Yes | No | Yes | **Yes** |
| Supply-chain cost | None | None | None | One pinned dev-only dependency |

Revisit if the library drops TonalSpot, or if tenants require brand theming.

### Canonical navigation and panel skin (ADR-0423)

| Criterion | Append another override layer | Rewrite whole stylesheet | **Replace repeated skin with one canonical block** |
| --- | --- | --- | --- |
| Auditability | Worse (ninth copy) | Best | **Good: one definition per skin** |
| Risk to compact rail, mobile, forced colors | Low | High | **Low: geometry untouched** |
| Verifiable in one change | Yes | No | **Yes: tests, contracts, e2e** |
| M3 drawer-item fidelity | Partial | Full | **Full** |

Revisit when the remaining legacy surfaces are migrated, at which point a full
token-first rewrite becomes cheaper.

### Canonical topbar and active navigation (ADR-0425)

| Criterion | Keep secondary-container indicator | Primary-container, no bar | Keep inset bar (ADR-0419) |
| --- | --- | --- | --- |
| Reads as "current page" | Weak | **Strong** | Strong |
| Uses a Material role | Yes | **Yes** | Yes, plus a non-M3 bar |
| Consistent in light and dark | Yes | **Yes** | Yes |
| Extra decoration | None | **None** | Inset bar |

Chosen: primary-container, no bar. Revisit if tenants need distinct brand
accents for the active item.

### Canonical form controls and supporting text (ADR-0426)

| Criterion | Filled fields (surface-container-highest) | **Outlined fields (transparent)** | Keep per-surface styles |
| --- | --- | --- | --- |
| Works on tonal panels and elevated menus | Poor (fill blends with menu tone) | **Good (outline carries the shape)** | Inconsistent |
| M3 invalid and disabled semantics | Yes | **Yes** | Missing |
| Single definition for shell and workspaces | Yes | **Yes** | No |
| Visual density | Heavier | **Lighter** | Mixed |

Chosen: outlined fields. Revisit if usability testing shows filled fields are
easier to locate in dense forms.

### Signed-out workspace gate and compact authentication control (ADR-0442)

| Criterion | Hide workspace until sign-in | Inline auth form always open | Modal auth flow | **Compact disclosure plus shared workspace gate** |
| --- | --- | --- | --- | --- |
| Clear primary next action | Partial | Weak | **Strong** | **Strong** |
| Preserves workspace context | No | Yes | Yes | **Yes** |
| Mobile/reflow complexity | Low | Poor | High | **Low** |
| Authorization semantics stay distinct | No | Partial | Yes | **Yes** |
| **Selection** | Rejected | Rejected | Rejected for this slice | **Selected in ADR-0442** |

Chosen: compact disclosure plus a shared signed-out gate. Revisit the modal
option if the authentication service gains recovery, device verification, or
multi-step identity flows that no longer fit a bounded shell disclosure.

### Authenticated workflow data gating (ADR-0443)

| Criterion | Editable fields with disabled submit | Hide workspace | **Visible workflow with explicit disabled controls and lock notice** |
| --- | --- | --- | --- |
| Communicates the missing prerequisite | Weak | Partial | **Strong** |
| Preserves task orientation | Yes | No | **Yes** |
| Keyboard and screen-reader semantics | Partial | Partial | **Strong** |
| Matches server-owned evidence boundaries | Partial | Yes | **Yes** |
| **Selection** | Rejected | Rejected | **Selected in ADR-0443** |

Chosen: visible workflow with explicit disabled controls and a localized lock
notice across payment, document quote, governance, treasury, balance checks,
refund, adjustment, recovery, and evidence workspaces. Revisit per-workspace
progressive disclosure when authenticated routes can provide safe read-only
previews with a measured product need.

The shell also fails closed while a token's server capability snapshot is
pending or unavailable: protected workspace children are not mounted until the
API confirms access. This prevents token-presence checks in individual panels
from becoming an accidental authorization boundary.

### Public simulator access boundary (ADR-0444)

| Criterion | Gate every workspace | Hard-code the simulator route | **Derive from policy requirements** |
| --- | --- | --- | --- |
| Matches the product/API contract | No | Partial | **Yes** |
| Avoids shell/policy drift | No | No | **Yes** |
| Unknown workspace behavior | Inconsistent | Inconsistent | **Fail closed** |
| **Selection** | Rejected | Rejected | **Selected in ADR-0444** |

Chosen: derive public/protected presentation from the policy catalog while
keeping server authorization authoritative.

### Authentication state hierarchy and protected workspace preview (ADR-0445)

| Criterion | Repeat shell and panel locks | Hide the whole page | **Single shell gate with protected children withheld** |
| --- | --- | --- | --- |
| Message hierarchy | Weak | Strong | **Strong** |
| Deep-link orientation | Yes | No | **Yes** |
| Keyboard and screen-reader safety | Partial | Strong | **Strong** |
| Risk of implying a usable protected form | High | Low | **Low** |
| **Selection** | Rejected | Rejected | **Selected in ADR-0445** |

Chosen: one shell-owned signed-out state, with authenticated role denial still
shown by the workspace when the server has actually evaluated the account.

### Progressive disclosure for long tax workflows (ADR-0446)

| Criterion | Expanded authoring and simulation | Separate simulation route | **Native disclosures in the Tax Rules workspace** |
| --- | --- | --- | --- |
| Initial cognitive load | High | Medium | **Low** |
| Keeps loaded-rule context | **Yes** | No | **Yes** |
| Keyboard and assistive-technology semantics | Partial | Yes | **Yes** |
| Result visibility after a successful run | Yes | Yes | **Yes, auto-opens** |
| Selection | Rejected | Rejected | **Selected** |

Chosen: keep both workflows together but require an explicit operator choice
before showing their long control sets. Native disclosures preserve semantics
and make the result state self-revealing after a successful simulation.

### Quiet navigation group metadata (ADR-0447)

| Criterion | Visible count and text symbols | Remove count metadata | **Accessible count with a visual chevron** |
| --- | --- | --- | --- |
| Task-label prominence | Low | **High** | **High** |
| Assistive-technology context | **Yes** | No | **Yes** |
| Native disclosure semantics | **Yes** | **Yes** | **Yes** |
| Selection | Rejected | Rejected | **Selected** |

Chosen: keep count context in the accessible name while removing repeated
visual metadata from the operator's primary navigation view.

### Canonical access-control and evidence surfaces (ADR-0427)

| Criterion | Migrate every legacy surface now | **Migrate only verifiable surfaces** | Leave all legacy styles |
| --- | --- | --- | --- |
| Proof that nothing regressed | None for data-only surfaces | **Snapshot diff for rendered surfaces** | N/A |
| Readability gain | Highest | **High where decisions are shown** | None |
| Risk | High | **Low** | Persistent 9-11px text |

Chosen: verifiable surfaces only. Revisit once style snapshots run inside the
Playwright journeys that render the data-dependent surfaces.

### In-journey style snapshots (ADR-0428)

| Criterion | Visual regression screenshots | Mock more data in the signed-out routes | **Computed-style snapshots in existing journeys** |
| --- | --- | --- | --- |
| Reuses existing mocked states | Yes | No | **Yes** |
| Detects intentional vs accidental changes | Pixel noise | N/A | **Per-property, per-element** |
| Determinism | Font/AA dependent | N/A | **0 diffs on identical CSS** |
| Cost to maintain | High (baselines) | High | **Low (one helper, one diff tool)** |

Chosen: computed-style snapshots. Revisit if journeys stop covering new
surfaces, or if pixel-level regressions (images, canvas) appear.

### Unit-test markup as style fixtures (ADR-0429)

| Criterion | Re-mock every API payload in Playwright | Storybook-style gallery app | **Capture markup from existing unit tests** |
| --- | --- | --- | --- |
| Reuses validated fixtures | No (duplicate) | No | **Yes** |
| New dependency or app | No | Yes | **No** |
| Markup fidelity | Exact | Approximate | **Exact (real components)** |
| Also adds behaviour tests | Partly | No | **Yes (gap states get tests)** |

Chosen: capture from unit tests. Revisit when a surface needs interaction
states (hover, focus) in its snapshot.

### Readable wallet availability guidance (ADR-0452)

| Criterion | Single-line ellipsis | Tooltip-only guidance | **Wrapped supporting text** |
| --- | --- | --- | --- |
| Full message visible by default | No | No | **Yes** |
| Keyboard/touch accessibility | Partial | Partial | **Yes** |
| Layout cost | **Low** | Low | Low |
| **Selection** | Rejected | Rejected | **Selected** |

Chosen: wrap the checkout-step supporting text so wallet prerequisites and
connected-account identity remain readable without relying on hover behavior.

### Responsive short-field layout (ADR-0453)

| Criterion | One vertical column | Viewport-specific two columns | **Container-bound auto-fit columns** |
| --- | --- | --- | --- |
| Desktop scan length | Poor | **Good** | **Good** |
| Collapsed rail / narrow workspace behavior | Good | Partial | **Good** |
| Evidence and action hierarchy | Good | Good | **Good** |
| **Selection** | Rejected | Rejected | **Selected** |

Chosen: apply bounded auto-fit columns inside form-owned workflow fieldsets so
short inputs group naturally while full-width evidence and actions remain easy
to scan.

### Single wallet identity display (ADR-0454)

| Criterion | Duplicate summary and control | Hide full address | **State summary plus owned wallet control** |
| --- | --- | --- | --- |
| Account inspectability | Good | **Poor** | **Good** |
| Review scanability | Poor | Good | **Good** |
| Ownership of provider/account details | Ambiguous | Partial | **Clear** |
| **Selection** | Rejected | Rejected | **Selected** |

Chosen: keep provider and full selected-account identity in the wallet control,
while the surrounding step summary communicates only connection state.

### Submit before chain evidence (ADR-0455)

| Criterion | Show on prepared artifact | Disabled placeholder | **Show after submitted intent** |
| --- | --- | --- | --- |
| Evidence authority | **Ambiguous** | Partial | **Clear** |
| Pre-signing scanability | Poor | Partial | **Good** |
| Supports actionable retry | Misleading | Poor | **Good** |
| **Selection** | Rejected | Rejected | **Selected** |

Chosen: begin chain-evidence verification only after a submitted settlement
intent exists, when a signature and chain observation are possible.

### Action-oriented payment-ready copy (ADR-0456)

| Criterion | Repeat safeguards | Remove supporting copy | **Give next-action guidance** |
| --- | --- | --- | --- |
| Evidence clarity | **Good** | Partial | **Good** |
| Decision hierarchy | Poor | Partial | **Good** |
| Operator next step | Poor | Poor | **Good** |
| **Selection** | Rejected | Rejected | **Selected** |

Chosen: retain the safeguard assertion once and use the ready-state supporting
copy to direct review of the exact scope and amounts before approval.

### Explicit atomic payment units (ADR-0457)

| Criterion | Bare integers | Guessed human currency | **Grouped atomic units** |
| --- | --- | --- | --- |
| Unit clarity | Poor | **Ambiguous** | **Clear** |
| Exactness | Good | Risk of conversion error | **Good** |
| Requires new API contract | No | Yes, but not present | **No** |
| **Selection** | Rejected | Rejected | **Selected** |

Chosen: improve scanability and unit clarity while preserving the artifact's
exact protocol values until an authorized display-metadata contract exists.

### Server-owned payment display metadata (ADR-0458)

| Criterion | Bare atomic artifact | Separate metadata request | **Metadata bound to compiled artifact** |
| --- | --- | --- | --- |
| Review consistency | Poor | Race-prone | **Strong** |
| Authority boundary | Ambiguous | Split | **Server-owned** |
| Human-readable amounts | No | Yes | **Yes** |
| Additional consistency boundary | None | **Yes** | **No** |
| **Selection** | Rejected | Rejected | **Selected** |

Chosen: bind the display context generated from the same quote, attestation,
and route used for the settlement instruction. Compute-budget policy remains
distinct from an exact provider-backed network-fee estimate.

### Authenticated identity presentation (ADR-0465)

| Criterion | Passive status plus account menu | **Account menu only** | Sidebar only |
| --- | --- | --- | --- |
| Identity/action hierarchy | Partial | **Clear** | Poor |
| Top-bar density | Poor | **Good** | Good |
| Access and sign-out discoverability | **Good** | **Good** | Poor |
| **Selection** | Rejected | **Selected** | Rejected |

Chosen: keep the account menu as the one authenticated identity/session
control. The sidebar can continue to show passive access posture, while the
account menu owns the identity facts and session action.

### Independent operational panel alignment (ADR-0466)

| Criterion | Stretch siblings | Fixed shared height | **Intrinsic height with top alignment** |
| --- | --- | --- | --- |
| Empty-state scanability | Poor | Poor | **Good** |
| Long/localized evidence safety | Partial | Poor | **Good** |
| Responsive behavior | Partial | Poor | **Good** |
| **Selection** | Rejected | Rejected | **Selected** |

Chosen: top-align the independent panels and preserve intrinsic content height;
the existing container-query collapse remains the narrow-layout authority.

### Narrow evidence-summary reflow (ADR-0467)

| Criterion | Permanent two-column row | Truncate/hide values | **Container-bound stacked row** |
| --- | --- | --- | --- |
| Wide-panel scanability | **Good** | Good | Good |
| Protocol evidence inspectability | **Good** | Poor | **Good** |
| Narrow/localized readability | Poor | Partial | **Good** |
| **Selection** | Rejected | Rejected | **Selected** |

Chosen: preserve the compact two-column summary at wide widths and stack the
label/value pair only when the workspace container is narrow.

### Multi-authority action grouping (ADR-0468)

| Criterion | Independent full-width buttons | Permanent two-column grid | **Flexible wrapping action row** |
| --- | --- | --- | --- |
| Authority-step relationship | Poor | **Good** | **Good** |
| Narrow/localized safety | Good | Poor | **Good** |
| Vertical scan efficiency | Poor | **Good** | **Good** |
| **Selection** | Rejected | Rejected | **Selected** |

Chosen: group related authority actions in a wrapping flex row with bounded
flexible buttons, preserving DOM order and narrow-panel safety.

### Mobile shell popover anchoring (ADR-0469)

| Criterion | Viewport-fixed dialog | Inline top-bar expansion | **Positioned top-bar containing block** |
| --- | --- | --- | --- |
| Visual reachability | Good | Good | **Good** |
| Workspace stability | Partial | Poor | **Good** |
| Focus/positioning complexity | Poor | Good | **Good** |
| Desktop interaction parity | Partial | Poor | **Good** |
| **Selection** | Rejected | Rejected | **Selected** |

Chosen: preserve the compact shell interaction and give its existing absolute
popovers the visible mobile top bar as their containing block and stacking
context.

### Typography and empty-panel rhythm (ADR-0470)

| Criterion | Runtime webfont | Local heading-only tuning | **Explicit system stack and scoped rhythm** |
| --- | --- | --- | --- |
| Cross-platform determinism | Partial | Poor | **Good** |
| Supply-chain/privacy cost | Poor | **Good** | **Good** |
| Localized heading safety | Good | Partial | **Good** |
| Workspace scanability | Good | Partial | **Good** |
| **Selection** | Rejected | Rejected | **Selected** |

Chosen: establish one readable type/layout contract without adding a runtime
font dependency, and scope empty-state compaction to multi-panel workspaces.

### Authentication error placement (ADR-0471)

| Criterion | Floating top-bar overlay | Duplicate workspace error | **In-form flow error** |
| --- | --- | --- | --- |
| Action/error association | Poor | Partial | **Strong** |
| Workspace obstruction | Poor | Partial | **None** |
| Announcement duplication | Good | Poor | **Good** |
| Desktop/mobile parity | Partial | Good | **Good** |
| **Selection** | Rejected | Rejected | **Selected** |

Chosen: keep the existing alert semantics, but render authentication failures
inside the anchored form surface that owns the request.

### Responsive calculator field rhythm (ADR-0472)

| Criterion | One column at every width | Viewport breakpoint | **Container-aware two-column rhythm** |
| --- | --- | --- | --- |
| Desktop scanability | Poor | Good | **Good** |
| Narrow/localized safety | Good | Partial | **Good** |
| Persistent-shell compatibility | Good | Poor | **Good** |
| DOM/financial semantics unchanged | **Good** | **Good** | **Good** |
| **Selection** | Rejected | Rejected | **Selected** |

Chosen: let the calculator panel use two columns only when its own inline size
can support them, preserving full-width evidence/help rows and a narrow-panel
single-column fallback.

### Stale SPA chunk recovery (ADR-0473)

| Criterion | Retain all historical chunks | Disable immutable asset caching | **Guarded reload on known chunk failure** |
| --- | --- | --- | --- |
| Deployment recovery | Good | Good | **Good** |
| Cache efficiency | Partial | Poor | **Good** |
| Bounded storage/complexity | Poor | Good | **Good** |
| Protection from reload loops | Good | Good | **Good** |
| **Selection** | Rejected | Rejected | **Selected** |

Chosen: recover only from known lazy-import failures with a session marker,
while retaining the localized manual recovery surface for other exceptions.

### Governed-rule builder field rhythm (ADR-0474)

| Criterion | One column | Viewport breakpoint | **Panel container-aware paired fields** |
| --- | --- | --- | --- |
| Authoring scanability | Poor | Good | **Good** |
| Persistent-shell compatibility | Good | Poor | **Good** |
| Localized/narrow safety | **Good** | Partial | **Good** |
| Evidence/help reading width | Good | Good | **Good** |
| **Selection** | Rejected | Rejected | **Selected** |

Chosen: pair short controls only when the governance panel itself has room,
while preserving full-width evidence and narrative rows and the existing
single-column fallback.

### Invoice quote failure recovery (ADR-0475)

| Criterion | Primary button only | Automatic retry | **Explicit in-alert retry** |
| --- | --- | --- | --- |
| Recovery discoverability | Poor | Good | **Good** |
| Retry storm risk | **Good** | Poor | **Good** |
| Provider-detail exposure | **Good** | Good | **Good** |
| Idempotency preservation | Good | Partial | **Good** |
| **Selection** | Rejected | Rejected | **Selected** |

Chosen: keep recovery explicit and local to the failure alert, reusing the
unchanged idempotency key while preserving redacted operator-facing errors.

### Payment-route failure recovery (ADR-0476)

| Criterion | Full page reload | Automatic retry | **Explicit route-alert retry** |
| --- | --- | --- | --- |
| Operator recovery | Poor | Good | **Good** |
| Preserves quote/wallet context | Poor | **Good** | **Good** |
| Authority-boundary safety | Good | Partial | **Good** |
| Outage amplification risk | Good | Poor | **Good** |
| **Selection** | Rejected | Rejected | **Selected** |

Chosen: retry only the server-owned route read from the checkout alert while
keeping compilation and signing fail-closed until a verified route is present.

### Document-checkout route failure recovery (ADR-0477)

| Criterion | Full page reload | Automatic retry | **Explicit route-alert retry** |
| --- | --- | --- | --- |
| Operator recovery | Poor | Good | **Good** |
| Preserves document/wallet context | Poor | **Good** | **Good** |
| Authority-boundary safety | Good | Partial | **Good** |
| Outage amplification risk | Good | Poor | **Good** |
| **Selection** | Rejected | Rejected | **Selected** |

Chosen: retry only the authenticated server-owned route read from the
document-checkout alert while keeping evidence verification, compilation, and
signing fail-closed until a verified route is available.

### Treasury sign-in readiness surface (ADR-0478)

| Criterion | Read-only textarea | Hide the state | **Semantic status surface** |
| --- | --- | --- | --- |
| Distinguishes state from input | Poor | Partial | **Good** |
| In-context recovery explanation | Good | Poor | **Good** |
| Credential disclosure risk | Good | Good | **Good** |
| Visual density | Poor | **Good** | **Good** |
| **Selection** | Rejected | Rejected | **Selected** |

Chosen: keep the authorization explanation in the treasury form, but render it
as a status surface rather than an input control.

### Treasury route failure recovery (ADR-0479)

| Criterion | Full page reload | Automatic retry | **Explicit route-warning retry** |
| --- | --- | --- | --- |
| Operator recovery | Poor | Good | **Good** |
| Preserves release context | Poor | **Good** | **Good** |
| Authority-boundary safety | Good | Partial | **Good** |
| Outage amplification risk | Good | Poor | **Good** |
| **Selection** | Rejected | Rejected | **Selected** |

Chosen: retry only the authenticated, server-owned treasury route read from
the warning while keeping release creation disabled until a verified route is
available.

### Treasury release-window input (ADR-0480)

| Criterion | Raw epoch seconds | Browser-local date/time | **Selected-zone date/time with exact conversion** |
| --- | --- | --- | --- |
| Operator usability | Poor | Good | **Good** |
| Time-zone correctness | Good | Poor | **Good** |
| Seconds/protocol precision | **Good** | Partial | **Good** |
| Invalid-window handling | Partial | Partial | **Good** |
| **Selection** | Rejected | Rejected | **Selected** |

Chosen: present date/time controls in the selected application time zone and
convert them to validated integer Unix seconds only at submission.

### Treasury protocol status presentation (ADR-0481)

| Criterion | Raw protocol value | Mechanical underscore formatting | **Localized catalog mapping** |
| --- | --- | --- | --- |
| Operator readability | Poor | Partial | **Good** |
| Locale support | None | None | **Good** |
| Stable API contract | **Good** | **Good** | **Good** |
| Unknown-status safety | Partial | Partial | **Good** |
| **Selection** | Rejected | Rejected | **Selected** |

Chosen: retain stable protocol values in the API and evidence model, then map
the exhaustively validated status union to localized operator copy at render
time.

### Cross-workspace protocol status presentation (ADR-0482)

| Criterion | Per-panel formatters | Raw/mechanical formatting | **Shared locale-backed presenter** |
| --- | --- | --- | --- |
| Consistent operator language | Partial | Poor | **Good** |
| Locale coverage | Partial | None | **Good** |
| Stable API/evidence values | **Good** | **Good** | **Good** |
| Safe unknown-value handling | Partial | Poor | **Good** |
| **Selection** | Rejected | Rejected | **Selected** |

Chosen: use one locale-backed web presenter for validated protocol statuses,
with an explicit localized unknown label and no raw-value fallback.

### Protected-workspace sign-in entry point (ADR-0483)

| Criterion | Instruction-only gate | Duplicate inline auth form | **Action that expands the global auth control** |
| --- | --- | --- | --- |
| Operator next-step clarity | Poor | Good | **Good** |
| Auth state ownership | **Good** | Poor | **Good** |
| Security/validation surface | **Good** | Poor | **Good** |
| Mobile and keyboard reachability | Partial | Good | **Good** |
| **Selection** | Rejected | Rejected | **Selected** |

Chosen: keep authentication owned by the global shell control while allowing a
protected workspace to open that control directly from its gate.

### Editorial operational visual direction (ADR-0484)

| Criterion | Uniform tonal shell | Reference copy | **Original semantic editorial shell** |
| --- | --- | --- | --- |
| Task hierarchy | Partial | Good | **Good** |
| Zero-mock/data truth | **Good** | Poor | **Good** |
| Product/domain clarity | **Good** | Poor | **Good** |
| Theme/accessibility compatibility | **Good** | Partial | **Good** |
| **Selection** | Rejected | Rejected | **Selected** |

Chosen: translate the reference's hierarchy, spacing, and rail/canvas
relationship into this product's semantic roles and real operational states.

### Signed-out protected-page sign-in action (ADR-0485)

| Criterion | Duplicate header and gate actions | Header action removed on protected pages | **One header action plus explanatory gate** |
| --- | --- | --- | --- |
| Cognitive load | Poor | Good | **Good** |
| Global discoverability | **Good** | Poor | **Good** |
| Auth state ownership | Good | Good | **Good** |
| Protected-state clarity | Good | Partial | **Good** |
| **Selection** | Rejected | Rejected | **Selected** |

Chosen: keep one global sign-in action in the header and use the protected gate
for local explanation and status only.

### Premium dark workspace composition (ADR-0486)

| Criterion | Flat control-plane canvas | Decorative dashboard treatment | **Semantic operational stage** |
| --- | --- | --- | --- |
| Premium visual hierarchy | Poor | Good | **Good** |
| Zero-mock/data truth | **Good** | Poor | **Good** |
| Protected-state clarity | Partial | Partial | **Good** |
| Responsive/accessibility safety | **Good** | Partial | **Good** |
| **Selection** | Rejected | Rejected | **Selected** |

Chosen: use semantic layers, editorial type, and a two-zone protected gate to
create composition without inventing dashboard content.

### Verified-context dashboard density (ADR-0487)

| Criterion | Expanded navigation plus empty canvas | Mock dashboard metrics | **Compact rail plus verified context deck** |
| --- | --- | --- | --- |
| Workspace focus | Poor | Good | **Good** |
| Data truthfulness | **Good** | Poor | **Good** |
| Accessibility/discoverability | Good | Good | **Good** |
| Signed-out usefulness | Poor | Partial | **Good** |
| **Selection** | Rejected | Rejected | **Selected** |

Chosen: use the compact rail to return horizontal space to the task and fill
the initial canvas only with access, deployment, network, and API facts already
owned by the shell. Domain metrics and records appear only when real workspace
data is available.

### Single context header and neutral rail (ADR-0488)

| Criterion | Repeated context cards | Context removed everywhere | **One verified deck plus neutral rail** |
| --- | --- | --- | --- |
| Cognitive load | Poor | Partial | **Good** |
| Authorization clarity | Good | Poor | **Good** |
| Workflow focus | Partial | Good | **Good** |
| Navigation orientation | Good | Partial | **Good** |
| **Selection** | Rejected | Rejected | **Selected** |

Chosen: keep one summary deck for stable context, retain a page-level card only
for actionable authorization uncertainty, and let the rail support orientation
without overpowering the work surface.

### Expanded first-run control plane (ADR-0489)

| Criterion | Expanded labeled rail | Compact icon rail | **Selection** |
| --- | --- | --- | --- |
| First-run discoverability | Workspace names and group hierarchy are immediately visible | Requires tooltip, memory, or exploration | **Expanded labeled rail** |
| Operator density | Adds a narrow, persistent orientation column | Maximizes canvas width | **Expanded labeled rail for default; compact remains opt-in** |
| Cognitive load | Makes the active area and available workflows explicit | Hides context and increases navigation uncertainty | **Expanded labeled rail** |
| Truthfulness | Supports real context cards and access guidance | Encourages filling empty space with decorative or fake metrics | **Expanded labeled rail** |

Chosen: keep the verified context deck, add an access sequence for protected
workspaces, and do not fabricate activity or financial KPIs.

### Licensed Metronic Demo 3 shell and dedicated authentication routes (ADR-0490)

| Criterion | Current expanded shell | Full Metronic demo copy | **Licensed Demo 3 adapter plus repository auth service** |
| --- | --- | --- | --- |
| Match to licensed reference | Partial | Good | **Good** |
| Domain/data truthfulness | **Good** | Poor | **Good** |
| Auth authority boundary | **Good** | Partial | **Good** |
| React/Vite compatibility | **Good** | Partial | **Good** |
| Privileged role safety | **Good** | Partial | **Good** |
| **Selection** | Rejected | Rejected | **Selected** |

Chosen: adapt the licensed Demo 3 shell hierarchy and use dedicated sign-in
and sign-up routes backed by the existing Rust email-OTP issuer. User sign-up
can provision only the base role; moderator and administrator roles remain
durable server-side assignments.

### Role-aware authentication scope profiles (ADR-0493)

| Option | Result | Reason |
| --- | --- | --- |
| One global scope list | Rejected | Gives every account the same authority-shaped claims. |
| Browser-composed role scopes | Rejected | The browser is not an authority boundary. |
| Hosted-provider groups only | Deferred | Not sufficient to prove the repository-owned local auth vertical slice. |
| **Auth-service role-aware scope profiles** | **Selected** | Durable roles and signed scope claims remain server-owned and deterministic. |

### Docker-backed browser authentication evidence (ADR-0494)

| Criterion | Synthetic credentials only | HTTP-only auth tests | **Dedicated Docker auth browser gate** |
| --- | --- | --- | --- |
| Form and redirect coverage | Poor | None | **Good** |
| Browser CORS and token handoff | None | Partial | **Good** |
| Isolation and repeatability | **Good** | Good | **Good** |
| Coverage of tenant roles | Poor | Partial | **Good** |
| **Selection** | Rejected | Rejected | **Selected** |

Chosen: retain the fast synthetic UI gate and add a separate isolated Docker
browser gate for the real local OTP, token, invitation, and role paths.

### Authentication return-path completion (ADR-0495)

| Criterion | Stay on auth route | Local storage return state | **Validated memory/session return path** |
| --- | --- | --- | --- |
| End-to-end task completion | Poor | Good | **Good** |
| Token exposure | **Good** | Poor | **Good** |
| Redirect safety | Good | Partial | **Good** |
| SPA and OIDC compatibility | Partial | Good | **Good** |
| **Selection** | Rejected | Rejected | **Selected** |

Chosen: keep access tokens in memory, validate the internal return path, and
use one-time session storage only across the OIDC provider redirect.

### Licensed Metronic Demo 3 shell fidelity (ADR-0497)

| Criterion | Existing custom shell | Full package copy | **Focused Demo 3 adapter** |
| --- | --- | --- | --- |
| Fidelity to licensed reference | Partial | Good | **Good** |
| Product/domain ownership | **Good** | Poor | **Good** |
| Upgrade and license risk | Good | Poor | **Good** |
| Responsive/accessibility control | **Good** | Partial | **Good** |
| **Selection** | Rejected | Rejected | **Selected** |

Chosen: reproduce the licensed Demo 3 composition—fixed header, compact icon
rail, horizontal workspace navigation, and rounded content canvas—while keeping
the repository's semantic tokens, real domain content, and accessible controls.

### Branded authentication composition (ADR-0498)

| Criterion | Generic centered form | Full Metronic auth copy | **Focused auth card plus context panel** |
| --- | --- | --- | --- |
| Sign-in task focus | **Good** | Partial | **Good** |
| Product/security context | Poor | Good | **Good** |
| Auth flow ownership | **Good** | Partial | **Good** |
| Responsive/accessibility control | **Good** | Partial | **Good** |
| **Selection** | Rejected | Rejected | **Selected** |

Chosen: adapt Metronic's branded authentication hierarchy into the existing
email-OTP/OIDC routes. The context panel is noninteractive, role-neutral, and
does not introduce a second sign-in action or browser-owned authority.

### Tenant invitation lifecycle console (ADR-0499)

| Criterion | Creation-only UI | Client-filtered records | **Auth-service lifecycle projection** |
| --- | --- | --- | --- |
| Tenant authority | Partial | Unsafe | **Server-derived** |
| Revocation discoverability | Missing | Partial | **Complete** |
| Auditability | Preserved | Unclear | **Preserved with terminal state** |
| Bounded work | Good | Unbounded client risk | **100-record server cap** |
| Idempotent mutation | Existing API unused | Existing API unused | **Existing API retained** |

Chosen: expose a bounded tenant-scoped projection and keep revoke as the
existing transactional, idempotent auth-service mutation.

### Server-sourced responsibility summary (ADR-0500)

| Option | Benefit | Cost / risk | Decision |
| --- | --- | --- | --- |
| Generic access state only | Minimal UI | Users, moderators, and administrators look identical after sign-in | Rejected |
| Decode JWT roles in the browser | No extra response work | Treats client-decoded claims as presentation authority and couples the shell to token shape | Rejected |
| Role badges across navigation | Highly visible | Repeats identity context and increases cognitive load on every workspace | Rejected |
| Use roles in the server-owned capability snapshot inside the account menu | Uses an already verified bounded response, keeps one identity surface, and clearly differentiates responsibilities | Requires localized role labels and a safe unknown-role fallback | Selected |

### Server-sourced verified email identity (ADR-0501)

| Alternative | Decision | Rationale |
| --- | --- | --- |
| Keep displaying opaque subject | Rejected | Does not give an operator a useful identity confirmation. |
| Decode JWT claims in the browser | Rejected | Duplicates token interpretation and makes client parsing part of the identity boundary. |
| Add a separate profile endpoint | Rejected | Adds another bootstrap dependency and identity authority. |
| Include the verified email in the access token and existing capability response | Selected | Reuses the authenticated server boundary and keeps the SPA identity display consistent with authorization evaluation. |

### Single actionable workspace context (ADR-0502)

| Criterion | Four visible status cards | No workspace status | **One access card plus global diagnostics** |
| --- | --- | --- | --- |
| Task focus | Poor | Good | **Good** |
| Access-boundary clarity | Good | Poor | **Good** |
| Duplication with header | Poor | Good | **Good** |
| Verified server context | Good | Partial | **Good** |
| **Selection** | Rejected | Rejected | **Selected** |

Chosen: show the actionable access state on the workspace and keep deployment,
network, API, and session diagnostics in the existing global status disclosure.

### Server-timed OTP resend recovery (ADR-0503)

| Criterion | No recovery action | Client-only timer | **Server-timed reuse of request endpoint** |
| --- | --- | --- | --- |
| Expired/delayed-code recovery | Missing | Partial | **Good** |
| Rate-limit authority | Good | Poor | **Good** |
| Account-enumeration risk | Good | Good | **Good** |
| Provider compatibility | Good | Partial | **Good** |
| **Selection** | Rejected | Rejected | **Selected** |

Chosen: reuse the auth service request contract and honor its bounded
`retry_after_seconds` response in the SPA.

### Server-sourced Access & approvals responsibility summary (ADR-0507)

| Option | Benefit | Cost / risk | Decision |
| --- | --- | --- | --- |
| Generic policy matrix only | Smallest UI | Leaves the current responsibility unclear and increases scanning cost | Rejected |
| Decode JWT roles/scopes | No additional response data | Couples presentation to token shape and confuses claims with tenant/resource authorization | Rejected |
| Client-side role-to-capability mapping | Concise role-specific UI | Can drift from server policy and create misleading permissions | Rejected |
| **Summarize the existing server capability snapshot** | Concise, verified, localized, and no new authority boundary | Depends on the bounded snapshot remaining complete and fresh | **Selected** |

Chosen: present only the roles and capability decisions already returned by the
server-owned authorization snapshot. The browser never reconstructs which role
grants which action.

### Metronic v9.5.0 composition source boundary (ADR-0508)

| Alternative | Decision | Rationale |
| --- | --- | --- |
| Copy the complete licensed React demo | Rejected | Adds unrelated routes, demo data, and identity dependencies. |
| Screenshot-only visual imitation | Rejected | Layout intent is not a reproducible or testable source contract. |
| Import Metronic authentication providers | Rejected | Conflicts with the repository-owned Rust OTP/OIDC service. |
| **Adapt the licensed Demo 3 composition into the existing SPA** | **Selected** | Preserves the requested shell/auth composition and keeps authorization, tokens, and production behavior repository-owned. |

Chosen: use the local v9.5.0 Demo 3 layout and branded-auth source files as the
auditable composition reference, expose data-layout contracts in the DOM, and
retain the repository's server-owned identity and authorization boundaries.

### Explicit production email-OTP browser method (ADR-0510)

| Alternative | Benefit | Cost / risk | Decision |
| --- | --- | --- | --- |
| Keep email OTP local-only | No additional release input | Production cannot use the repository-owned auth service | Rejected |
| Infer the method from whichever settings are present | Fewer settings | Ambiguous or accidental auth-method selection | Rejected |
| **Explicit `VITE_AUTH_METHOD` switch** | Auditable OIDC/email-OTP choice with fail-closed validation | One additional release input | **Selected** |

Chosen: make provider-backed email OTP available to staging and production only
through an explicit `VITE_AUTH_METHOD=email-otp` release setting; retain the
local alias for Docker and enforce provider mode independently in the Rust auth
service.

### One-time initial administrator enrollment (ADR-0511)

| Alternative | Benefit | Cost / risk | Decision |
| --- | --- | --- | --- |
| Manual SQL promotion | Fast bootstrap | Bypasses application audit and token contracts | Rejected |
| Normal administrator invitation | Reuses the UI | Makes global authority delegable through normal tenant controls | Rejected |
| Browser bootstrap secret | Explicit operator intent | Adds a client-visible credential boundary | Rejected |
| **Configured mailbox with one-active-admin transaction fence** | Reuses OTP and produces durable tenant/role/audit evidence | Requires controlled deployment configuration and removal | **Selected** |

Chosen: allow only the exact server-configured mailbox to complete the normal
signup path while no active administrator exists; keep administrator outside
the normal assignable role catalog.

### Metronic typography hierarchy (ADR-0512)

| Alternative | Result | Reason |
| --- | --- | --- |
| Keep display-sized workspace headings and 11px utility copy | Rejected | Creates an uneven hierarchy and weakens readability. |
| Import the complete Metronic CSS bundle | Rejected | Adds a second style authority and unrelated surface rules. |
| Remote webfont dependency | Rejected | Adds external loading/privacy variance to the static SPA. |
| **Tune repository-owned semantic type roles and layout selectors** | **Selected** | Preserves the licensed composition while improving scale, wrapping, and accessibility. |

Chosen: use a compact 32px workspace title, 16px explanatory copy, a 12/13px
minimum for supporting text, and a grid-owned authentication context so the
brand and context kicker remain separate at narrow widths.

### Metronic Demo 3 workspace density (ADR-0513)

| Alternative | Result | Reason |
| --- | --- | --- |
| Keep one narrow access card | Rejected | Creates a sparse stage and does not express the release/service context clearly. |
| Import the full Metronic CSS bundle | Rejected | Adds a second styling authority and can weaken the repository's semantic color and accessibility contracts. |
| Decorative dashboard art | Rejected | Adds noise without improving operational comprehension. |
| **Repository-owned four-card Demo 3 status deck** | **Selected** | Restores the reference layout rhythm with truthful runtime-owned status values. |

Chosen: retain the licensed Demo 3 shell geometry and express access,
deployment, network, and API state as a compact responsive card grid before the
protected workspace explanation.

### Metronic Demo 3 operational callout (ADR-0514)

| Alternative | Result | Reason |
| --- | --- | --- |
| Preserve the oversized authentication stage | Rejected | It consumes the visual hierarchy and repeats the access state. |
| Add a second sign-in button in the workspace | Rejected | It increases cognitive load and creates competing authentication actions. |
| Remove the protected-workspace explanation | Rejected | The user still needs a concise reason and next-step model. |
| **Compact access callout with a process list** | **Selected** | It keeps the security explanation while matching Demo 3's dense operational rhythm. |

Chosen: keep sign-in global, compress protected-workspace guidance into a
two-column callout, and reflow it to one column on narrow screens.

### Operator-facing responsibility and invitation labels (ADR-0504)

| Criterion | Raw protocol values | Client-defined semantics | **Bounded presentation mapping** |
| --- | --- | --- | --- |
| Administrator readability | Poor | Good | **Good** |
| Contract ownership | Good | Poor | **Good** |
| Version-skew visibility | Good | Poor | **Good** |
| Auditability | Good | Partial | **Good** |
| **Selection** | Rejected | Rejected | **Selected** |

Chosen: preserve server-owned role/status codes for all protocol behavior and
map only their presentation in the administrator directory, with a visible
fallback for unknown values.

### Production control-plane UI rewrite (ADR-0515)

| Alternative | Benefit | Cost / risk | Decision |
| --- | --- | --- | --- |
| Incrementally patch the existing dense shell | Small diff | Preserves duplicated context, ambiguous hierarchy, and overlapping auth affordances | Rejected |
| Decode JWT roles/scopes in the browser | Immediate labels without a capability request | Confuses identity claims with tenant/resource/policy authorization | Rejected |
| Hide unauthorized navigation entries | Less visual noise | Hides governed product capability and makes support/audit harder | Rejected |
| **Replace the presentation shell while preserving typed domain/auth boundaries** | Clear operator model, bounded scope, and no change to settlement authority | Requires new UI contract and regression coverage | **Selected** |

Chosen: rewrite the React shell and auth route from scratch, retain the
repository-owned auth/authorization contracts and workflow panels, and render
server-owned RBAC/ABAC/PBAC decisions as explicit access states.

### Reference-aligned dashboard composition (ADR-0515)

| Alternative | Benefit | Cost / risk | Decision |
| --- | --- | --- | --- |
| Copy the reference's business metrics and chart values | Highest visual similarity | Fabricates product data and creates misleading operational state | Rejected |
| Keep only status cards and the protected-workspace gate | Lowest implementation risk | Does not match the supplied dashboard composition or provide a useful overview | Rejected |
| **Use the reference geometry with server-derived policy metrics and explicit empty states** | Preserves the rail/card/chart rhythm while remaining truthful and auditable | Requires responsive chart primitives and additional accessibility coverage | **Selected** |

Chosen: reproduce the reference's light rail, rounded card rhythm, chart areas,
and lower activity surfaces using authorization snapshot counts and service
posture only. Business analytics remain absent until a versioned API contract
provides verified evidence.

### Invitation-bound sign-up (ADR-0505)

| Criterion | Generic link and email lookup | Role/tenant URL fields | **Bound invitation reference** |
| --- | --- | --- | --- |
| End-to-end invitation traceability | Partial | Good | **Good** |
| Server authority | Good | Poor | **Good** |
| Credential exposure | Good | Poor | **Good** |
| Backward compatibility | **Good** | Partial | **Good** |
| **Selection** | Rejected | Rejected | **Selected** |

Chosen: carry only the server-issued invitation reference, bind it to the OTP
challenge after matching email/status/expiry, and keep role/tenant authority on
the server.
### Server-owned domain role catalog and explicit provisioning (ADR-0509)

| Alternative | Result | Reason |
| --- | --- | --- |
| Map administrator/moderator to every domain scope | Rejected | Collapses identity administration into tax and money-moving authority. |
| Browser-composed role-to-scope mapping | Rejected | The browser is not an authority boundary and can drift from the API. |
| Arbitrary tenant policy database | Rejected for this slice | Adds a second mutable policy authority before ownership and approval semantics are settled. |
| **Versioned role contract with explicit server-side assignment** | **Selected** | Keeps role semantics reviewable, scope derivation deterministic, tenant-scoped, and auditable. |

### Full admin application rewrite (ADR-0516)

| Area | Existing shell/panels | New React admin application | Selection reason |
| --- | --- | --- | --- |
| Presentation ownership | Shared legacy CSS and workflow tree | Dedicated admin route/application boundary | Makes the requested discard explicit and prevents legacy selectors from becoming dependencies |
| Styling | Hand-authored global CSS | Tailwind utility composition with semantic tokens | Responsive composition, local ownership, and auditable design tokens |
| Routing/server state | Browser pathname plus local fetch state | TanStack Router + TanStack Query | Typed routes, bounded cache lifecycle, retries, stale state, and invalidation |
| Data tables | Per-panel ad hoc lists | TanStack Table | Consistent sorting, filtering, pagination, and accessible row semantics |
| Visualization | CSS-only status blocks | Apache ECharts with typed API series | Production chart lifecycle and explicit loading/empty/error states |

Chosen: mount the new admin application from `main.tsx`, keep the Rust/API and
authorization contracts as integration boundaries, and stop importing the
legacy React presentation tree into the production entrypoint.

### Admin typography delivery and auth density (ADR-0517)

| Alternative | Benefit | Cost / risk | Decision |
| --- | --- | --- | --- |
| Unbundled Inter fallback with the existing display scale | No asset change | Host-dependent rendering and oversized auth hierarchy | Rejected |
| Runtime remote web font | Fast to trial | Third-party request, privacy/performance dependency, and fallback variance | Rejected |
| System UI font only | No font asset | Cross-platform metrics and brand character vary | Rejected |
| **Bundled Plus Jakarta Sans with responsive auth scale** | Deterministic delivery, modern geometry, and compact mobile hierarchy | Small font asset and explicit typography contract | **Selected** |

Chosen: ship the font with the SPA, retain a resilient system fallback, keep
controls at 1rem, use a 2rem auth heading until the desktop two-column layout
is active, and reserve full-height composition for desktop.

### OTP expiry and resend recovery (ADR-0518)

| Alternative | Benefit | Cost / risk | Decision |
| --- | --- | --- | --- |
| Full page reload to retry | Small implementation | Loses context and obscures the active challenge | Rejected |
| Client-side expiry extension | Appears forgiving | Violates server-owned authentication and replay boundaries | Rejected |
| **Server-bounded resend with visible expiry** | Recoverable operator experience while preserving rate limits and authority | Adds bounded challenge state and UI contract coverage | **Selected** |

Chosen: render the server expiry and cooldown, request a new server challenge
after the cooldown, clear the old code, and provide a different-email reset.

### Bounded OTP rate-limit window (ADR-0521)

| Alternative | Benefit | Cost / risk | Decision |
| --- | --- | --- | --- |
| Hard-coded 15-minute window everywhere | Conservative abuse control | Slows controlled staging recovery and hides policy in SQL | Rejected |
| Hard-coded five-minute window everywhere | Faster recovery | Weakens production protection without an environment guard | Rejected |
| Provider-only throttling | Less application code | Gives up application-owned abuse and cost controls | Rejected |
| **Bounded environment setting with a 5-minute staging value and 15-minute production default** | Testable staging with a production floor | Requires explicit deployment configuration and tests | **Selected** |

Chosen: keep the server-owned request count, source limit, and resend cooldown;
make only the rolling window configurable with a non-local five-minute minimum.

### Initial administrator account setup (ADR-0522)

| Option | Benefit | Cost / risk | Decision |
| --- | --- | --- | --- |
| Keep generic authentication failure | Minimizes account-state disclosure | Makes valid bootstrap OTPs look expired and causes retry loops | Rejected |
| Provision on sign-in | Lowest user friction | Blurs signup/sign-in authority and creates state from the wrong intent | Rejected |
| **Explicit setup response after verified OTP** | Actionable recovery while preserving server-owned provisioning | Requires one additional typed error contract | **Selected** |

The selected path is restricted to the configured initial-admin bootstrap email;
unknown-account sign-in attempts retain the generic failure response.

### Single-browser session coordination (ADR-0528)

| Alternative | Benefit | Cost / risk | Decision |
| --- | --- | --- | --- |
| Persist an access or refresh token in browser storage | Reload and cross-tab continuity | Browser-storage credential exposure and a larger XSS blast radius | Rejected |
| Broadcast bearer tokens between tabs | Immediate cross-tab reuse | Exposes credentials to every same-origin tab and does not provide server revocation | Rejected |
| **Broadcast a non-secret session claim and clear conflicting tabs** | Removes identity confusion without persisting credentials | Reload still requires authentication; the second tab must sign in | **Selected in ADR-0528** |
| HttpOnly Secure cookie with server-side session records | Standard persistence and central revocation | Requires a dedicated BFF/session migration, CSRF contract, and rollout | Deferred |

Chosen: keep access tokens memory-only, coordinate one active browser identity
with `BroadcastChannel` plus a storage-event fallback, and treat the signal as
UI coordination only. API signature, expiry, tenant, scope, and role checks
remain authoritative.

### Bounded open self-signup (ADR-0523)

| Alternative | Benefit | Cost / risk | Decision |
| --- | --- | --- | --- |
| Closed signup with manual bootstrap only | Smallest abuse surface | Blocks a public staging evaluation flow | Rejected as the only mode; remains the default |
| Frontend or edge counter | Easy to add | Bypassable and not authoritative across instances | Rejected |
| Per-email counter | Limits one mailbox | Does not bound aggregate tenant creation | Rejected |
| **PostgreSQL-serialized global daily quota** | Authoritative, auditable, and race-safe | Couples the quota decision to the auth database | **Selected** |

Chosen: explicitly opt in with `AUTH_SELF_SIGNUP=true`, count only committed
regular self-signup provisions, and cap the UTC-day total at 25 by default.
Invitations and the configured initial administrator bootstrap remain separate
server-owned provisioning paths.

### Capability environment context and unnamed workspace presentation (ADR-0530)

| Decision area | Alternative | Benefit | Cost / risk | Decision |
| --- | --- | --- | --- | --- |
| Capability attributes | Treat every catalog attribute as unresolved until a browser selects a resource | Simple evaluator and fail-closed behavior | Blocks capabilities whose context is already known to the service, including quote preparation | Rejected |
| Capability attributes | Grant every capability to `administrator` | Removes operator friction | Bypasses scope, role, resource, and policy boundaries; unsafe for money and governance actions | Rejected |
| Capability attributes | Resolve deployment `environment` in the API and leave resource-dependent attributes such as `jurisdiction` indeterminate | Preserves server authority, keeps the snapshot database-free, and allows quote preparation to reach its existing handler-level checks | Requires an explicit distinction between process context and selected resource context | **Selected** |
| Workspace identity | Invent a tenant name from an email address or domain | Removes the placeholder immediately | Creates an unreviewed tenant identity and can leak or misrepresent organization data | Rejected |
| Workspace identity | Mutate production tenant data to a hard-coded product name | Fast visual result | Not tenant-owned, not portable, and not auditable as an operator decision | Rejected |
| Workspace identity | Keep the durable profile incomplete until a tenant administrator names it, but render a clear setup label instead of `Untitled workspace` | Truthful readiness, no fabricated identity, and an explicit recovery path | Requires one administrator action | **Selected** |

Chosen: the API treats `environment` as a resolved process attribute because
the deployment preflight requires `API_ENV`; `jurisdiction` and other selected
resource attributes remain indeterminate until the relevant server-owned
resource context exists. The browser never grants the resulting capability.
The tenant database keeps its explicit incomplete-profile marker, while the
SPA presents “Name your workspace” and links the administrator to the audited
profile mutation instead of exposing an internal placeholder.

### Hosted Solana network and browser RPC binding (ADR-0524)

| Alternative | Benefit | Cost / risk | Decision |
| --- | --- | --- | --- |
| Infer RPC from the browser network label | Minimal configuration | A label is not a provider endpoint and can leave signing unavailable or misdirected | Rejected |
| Proxy wallet RPC through the API | Centralized endpoint control | Adds a settlement-path dependency and expands the API authority boundary | Rejected |
| Hard-code a public Devnet RPC in the SPA | Simple deployment | Provider coupling and accidental cross-network reuse | Rejected |
| **Explicit public build-time RPC, non-local preflight, and finalized program verification** | Auditable network binding, usable wallet signing, and fail-closed release evidence | Requires release configuration and an independent chain check | **Selected** |

Chosen: bind the hosted SPA to an explicit HTTPS RPC URL, keep server-side RPC
provider lists separate, declare the canonical program for Devnet, and require
finalized executable-program evidence before calling a release Solana-ready.

### Public customer checkout handoff (ADR-0525)

| Alternative | Benefit | Cost / risk | Decision |
| --- | --- | --- | --- |
| **Short-lived opaque checkout session bound to an authenticated quote and route** | Lets a guest customer pay with their own wallet without receiving merchant authority; easy to expire and audit | Requires a narrowly scoped public token contract and customer-facing page | **Selected** |
| Customer JWT/OIDC account | Reusable identity and profile support | Adds PII, account lifecycle, recovery, and authorization surface that is not needed to settle one quote | Rejected for this slice |
| Merchant access token in the customer browser | Small implementation | Leaks tenant capabilities and lets a customer call merchant APIs | Rejected |
| Client-supplied quote, route, and amount | Simple link payload | Client data is not settlement authority and can be tampered with | Rejected |

Chosen: a merchant creates a bounded public session from an already persisted,
attested quote and a finalized active route. The API returns the raw opaque
token once, stores only its SHA-256 digest, and accepts it only on dedicated
public checkout endpoints. The customer page supplies only their wallet public
key and nonce; all merchant accounts, mint, tax vault, amounts, policy digest,
program ID, and transaction instruction bytes remain server-owned. A durable
settlement intent is bound to the session so a session cannot compile multiple
independent payments.

### Role-aware tenant portals (ADR-0526)

| Alternative | Benefit | Cost / risk | Decision |
| --- | --- | --- | --- |
| Client-selected `/admin` and `/merchant` login modes | Immediately visible distinction | Lets an untrusted browser request an authority boundary and duplicates authentication state | Rejected |
| Keep one identical shell and rely only on capability counters | Smallest change | Users cannot understand whether they are administrators or operators; denied navigation remains noisy | Rejected |
| Create a new platform-admin hierarchy now | Clear long-term separation | Expands scope into a new authority, tenant lifecycle, and control-plane product without a defined contract | Rejected |
| **Derive role-aware tenant/admin presentation from the server snapshot while keeping one authentication boundary** | Clear UX, no client-granted authority, backward-compatible contract, and separate guest checkout remains explicit | Requires bounded snapshot-aware navigation and degraded-state handling | **Selected** |

Chosen: use server-confirmed roles and capability decisions to label the
tenant administrator and merchant operations portals, filter denied navigation,
and show a fail-closed loading/unavailable state. This is a presentation
boundary only; every protected API remains independently authorized. A future
platform administrator requires a separate role and ADR rather than being
implicitly mapped to tenant administration.

### OTP verification outcome and recovery (ADR-0529)

| Alternative | Benefit | Cost / risk | Decision |
| --- | --- | --- | --- |
| Check account existence during OTP request | Immediate feedback | Enables account enumeration and changes request timing | Rejected |
| Return raw server error messages to the SPA | Minimal client work | Leaks diagnostics and bypasses reviewed product copy | Rejected |
| Leave valid challenges reusable after account-policy rejection | Lets a user retry without a new email | Violates single-use proof semantics and weakens auditability | Rejected |
| **Consume valid proofs and expose allow-listed auth codes with bounded recovery actions** | Privacy-preserving request behavior, terminal replay semantics, and actionable UX | Requires an auth-specific browser error contract | **Selected** |

Chosen: keep account resolution after OTP proof, consume every valid challenge
once, and let the SPA render only reviewed recovery actions for known auth
codes. Unknown or disabled-account sign-in attempts remain generic.

### Explicit tenant lifecycle and readiness (ADR-0527)

| Alternative | Benefit | Cost / risk | Decision |
| --- | --- | --- | --- |
| Keep `merchant_id` as an implicit workspace | No migration | Cannot express profile, readiness, or safe tenant administration | Rejected |
| Let the browser select a tenant or administrator role | Fast UI flow | Untrusted client becomes an authority input | Rejected |
| Make every self-signup an administrator | Simple onboarding | Collapses governance and invitation boundaries | Rejected |
| Create a platform-admin hierarchy now | Long-term separation | Expands into a new authority model before its threat model exists | Rejected |
| **Explicit tenant entity with derived readiness and dedicated audited admin promotion** | Truthful onboarding, recoverable tenant lifecycle, and clear authority boundary | Additive schema and bounded auth-service contracts | **Selected** |

Chosen: backfill and provision `auth_tenants`, expose a server-owned tenant
context, derive setup steps from durable route/policy evidence, and keep
administrator promotion separate from the assignable role catalog. Readiness
does not authorize money movement, and no platform-admin semantics are implied.

### Settlement permission and route-loading UX (ADR-0531)

| Alternative | Benefit | Cost / risk | Decision |
| --- | --- | --- | --- |
| Grant settlement scope to every administrator | Fewer blocked screens | Collapses tenant administration and money-moving responsibility | Rejected |
| Keep the existing generic badge and query pending state | No code change | Misleading loading message when the query is disabled | Rejected |
| **Keep server-gated settlement access and map disabled-query state to truthful operator copy** | Preserves least privilege, fail-closed behavior, and clear recovery guidance | Requires capability-aware UI state mapping and regression coverage | **Selected** |

Chosen: settlement access remains a separately assigned responsibility. The
frontend never treats an unauthorized or disabled route query as loading, and
it never synthesizes a route when finalized evidence is unavailable.

### Stale admin lazy-chunk recovery (ADR-0532)

| Alternative | Benefit | Cost / risk | Decision |
| --- | --- | --- | --- |
| Ask users to hard-refresh after every deployment | Minimal implementation | Poor tier-1 UX and does not contain raw chunk errors | Rejected |
| Disable code splitting | Avoids lazy chunk mismatch | Increases initial bundle and delays every route | Rejected |
| **Wrap lazy surfaces in the existing bounded error boundary** | Recovers open tabs once, preserves redacted UX, and keeps immutable asset delivery | Does not replace a future blue/green asset retention strategy | **Selected** |

Chosen: the admin and checkout lazy surfaces use a shared error boundary around
`Suspense`. The server and edge remain authoritative; no client fallback
fabricates workspace data.
