import { FormEvent, Suspense, lazy, useEffect, useRef, useState } from "react";
import { fetchWithTimeout, REQUEST_TIMEOUT_MS, responseError } from "./api";
import {
  DEFAULT_LOCALE,
  DEFAULT_TIME_ZONE,
  directionForLocale,
  formatAtomic as formatAtomicValue,
  isValidTimeZone,
  SUPPORTED_LOCALES,
  SupportedLocale,
} from "./formatters";
import WorkspaceErrorBoundary from "./WorkspaceErrorBoundary";
import { createClientUuid } from "./clientIdentity";

const CheckoutPanel = lazy(() => import("./CheckoutPanel"));
const GovernancePanel = lazy(() => import("./GovernancePanel"));
const ReplayOpsPanel = lazy(() => import("./ReplayOpsPanel"));
const VaultReleasePanel = lazy(() => import("./VaultReleasePanel"));
const VaultReconciliationPanel = lazy(() => import("./VaultReconciliationPanel"));
const SettlementAdjustmentPanel = lazy(() => import("./SettlementAdjustmentPanel"));
const RefundClaimPanel = lazy(() => import("./RefundClaimPanel"));

type Workspace = "quote" | "preview" | "checkout" | "governance" | "replay-ops" | "vault-release" | "vault-reconciliation" | "adjustments" | "refund-claims";
type ServiceState = "checking" | "online" | "offline";

type Calculation = {
  calculation_id: string;
  status: "calculated_unattested" | "attested" | "simulated_unattested";
  expires_at_unix?: number;
  result: {
    rule_set_id: string;
    rule_version: number;
    engine_version: string;
    currency: string;
    jurisdiction: string;
    facts_digest: number[];
    rule_digest: number[];
    gross_atomic: string;
    total_tax_atomic: string;
    total_net_atomic: string;
    jurisdiction_resolution?: {
      jurisdiction: string;
      source: string;
      candidate_set_digest: number[];
    };
    components: Array<{
      id: string;
      label: string;
      base_atomic: string;
      tax_atomic: string;
      explanation: string;
    }>;
  };
};

type ReplayVerification = {
  status: "replay_verified";
  calculation_id: string;
  engine_version: string;
  facts_digest: number[];
  rule_digest: number[];
};

const API_BASE = import.meta.env.VITE_API_BASE ?? "/api";

function shortDigest(value: number[]): string {
  return value.map((byte) => byte.toString(16).padStart(2, "0")).join("").slice(0, 16) + "…";
}

export default function App() {
  const [serviceState, setServiceState] = useState<ServiceState>("checking");
  const [workspace, setWorkspace] = useState<Workspace>("quote");
  const workspaceContentRef = useRef<HTMLDivElement>(null);
  const initialWorkspaceRender = useRef(true);
  const [gross, setGross] = useState("1100");
  const [net, setNet] = useState("1000");
  const [rate, setRate] = useState("10");
  const [jurisdiction, setJurisdiction] = useState("US-CA");
  const [ruleSetId, setRuleSetId] = useState("operator-simulator-flat-rate");
  const [ruleVersion, setRuleVersion] = useState("1");
  const [regime, setRegime] = useState("sales-tax");
  const [jurisdictionEvidenceJson, setJurisdictionEvidenceJson] = useState("[]");
  const [accessToken, setAccessToken] = useState("");
  const [idempotencyKey, setIdempotencyKey] = useState<string>(() => createClientUuid());
  const [calculation, setCalculation] = useState<Calculation | null>(null);
  const [replayVerification, setReplayVerification] = useState<ReplayVerification | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [isReplaying, setIsReplaying] = useState(false);
  const [locale, setLocale] = useState<SupportedLocale>(DEFAULT_LOCALE);
  const [timeZone, setTimeZone] = useState(() => Intl.DateTimeFormat().resolvedOptions().timeZone || DEFAULT_TIME_ZONE);

  const effectiveTimeZone = isValidTimeZone(timeZone) ? timeZone : DEFAULT_TIME_ZONE;
  const direction = directionForLocale(locale);

  useEffect(() => {
    const root = document.documentElement;
    const previousLanguage = root.lang;
    const previousDirection = root.dir;
    root.lang = locale.split("-")[0];
    root.dir = direction;
    return () => {
      root.lang = previousLanguage;
      root.dir = previousDirection;
    };
  }, [direction, locale]);

  useEffect(() => {
    let active = true;
    fetchWithTimeout(`${API_BASE}/healthz`, {}, REQUEST_TIMEOUT_MS.health)
      .then((response) => {
        if (!response.ok) throw new Error("health check failed");
        if (active) setServiceState("online");
      })
      .catch(() => {
        if (active) setServiceState("offline");
      });
    return () => {
      active = false;
    };
  }, []);

  useEffect(() => {
    if (initialWorkspaceRender.current) {
      initialWorkspaceRender.current = false;
      return;
    }
    workspaceContentRef.current?.focus();
  }, [workspace]);

  function clearResult() {
    setError(null);
    setCalculation(null);
    setReplayVerification(null);
  }

  async function calculatePreview(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    clearResult();
    setIsSubmitting(true);
    try {
      const response = await fetchWithTimeout(`${API_BASE}/v1/tax/calculate`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          facts: { gross_atomic: gross.trim(), net_atomic: net.trim(), currency: "USD", jurisdiction },
          rule_set: {
            id: "operator-simulator-flat-rate",
            version: 1,
            components: [{
              id: "sales-tax",
              label: "Sales tax",
              base: { sources: ["Net"] },
              rate: { Flat: { numerator: rate.trim(), denominator: "100" } },
              price_mode: "Exclusive",
              credit_atomic: "0",
              floor_atomic: null,
              cap_atomic: null,
              rounding: { mode: "HalfUp" }
            }]
          }
        })
      });
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw new Error(responseError(response, payload, "Preview calculation failed"));
      setCalculation(payload as Calculation);
      setReplayVerification(null);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Preview calculation failed");
    } finally {
      setIsSubmitting(false);
    }
  }

  async function createQuote(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    clearResult();
    if (!accessToken.trim()) {
      setError("An access token with tax:quote scope is required. It is held in memory only.");
      return;
    }
    let jurisdictionEvidence: unknown;
    try {
      jurisdictionEvidence = JSON.parse(jurisdictionEvidenceJson);
      if (!Array.isArray(jurisdictionEvidence) || jurisdictionEvidence.length === 0) {
        throw new Error("At least one provider-signed jurisdiction evidence envelope is required.");
      }
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Jurisdiction evidence must be valid JSON.");
      return;
    }
    setIsSubmitting(true);
    try {
      const response = await fetchWithTimeout(`${API_BASE}/v1/tax/quote`, {
        method: "POST",
        headers: { "content-type": "application/json", authorization: `Bearer ${accessToken.trim()}` },
        body: JSON.stringify({
          idempotency_key: idempotencyKey.trim(),
          rule_set_id: ruleSetId.trim(),
          rule_version: Number(ruleVersion),
          jurisdiction,
          regime: regime.trim(),
          facts: { gross_atomic: gross.trim(), net_atomic: net.trim(), currency: "USD", jurisdiction },
          jurisdiction_evidence: jurisdictionEvidence
        })
      });
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw new Error(responseError(response, payload, `Quote request failed (${response.status})`));
      setCalculation(payload as Calculation);
      setReplayVerification(null);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Quote request failed");
    } finally {
      setIsSubmitting(false);
    }
  }

  async function verifyPersistedReplay() {
    if (!calculation || !["calculated_unattested", "attested"].includes(calculation.status)) return;
    if (!accessToken.trim()) {
      setError("An access token with tax:quote scope is required for evidence replay.");
      return;
    }
    setError(null);
    setIsReplaying(true);
    try {
      const response = await fetchWithTimeout(`${API_BASE}/v1/tax/quotes/${calculation.calculation_id}/replay`, {
        headers: { authorization: `Bearer ${accessToken.trim()}` },
      }, REQUEST_TIMEOUT_MS.default);
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw new Error(responseError(response, payload, `Replay verification failed (${response.status})`));
      setReplayVerification(payload as ReplayVerification);
    } catch (cause) {
      setReplayVerification(null);
      setError(cause instanceof Error ? cause.message : "Replay verification failed");
    } finally {
      setIsReplaying(false);
    }
  }

  const serviceLabel = { checking: "Checking service", online: "Calculation service online", offline: "Calculation service unavailable" }[serviceState];
  const submit = workspace === "quote" ? createQuote : calculatePreview;
  const formatAtomic = (value: string, currency: string) => formatAtomicValue(value, currency, locale);
  const statusLabel = workspace === "governance" ? "Governance workspace" : workspace === "replay-ops" ? "Replay operations" : workspace === "vault-release" ? "Vault release control" : workspace === "vault-reconciliation" ? "Vault reconciliation" : workspace === "adjustments" ? "Settlement adjustments" : workspace === "refund-claims" ? "Refund claim wallet boundary" : calculation?.status === "attested" ? "Attested" : calculation?.status === "calculated_unattested" ? "Calculated" : calculation ? "Preview" : "Awaiting input";

  return (
    <div className="app-shell" dir={direction}>
      <a className="skip-link" href="#workspace-content">Skip to workspace content</a>
      <header className="topbar">
        <a className="brand" href="/" aria-label="Solana Tax Protocol home"><span className="brand-mark">S</span><span><strong>Solana Tax Protocol</strong><small>Tax and settlement operations</small></span></a>
        <div className="topbar-actions">
          <div className={`service-status service-${serviceState}`} role="status" aria-live="polite"><span className="status-dot" />{serviceLabel}</div>
          <details className="display-settings">
            <summary>Display settings</summary>
            <div className="display-settings-menu">
              <label><span>Language</span><select value={locale} onChange={(event) => setLocale(event.target.value as SupportedLocale)}>{SUPPORTED_LOCALES.map((option) => <option value={option} key={option}>{option}</option>)}</select></label>
              <label><span>Time zone</span><input value={timeZone} onChange={(event) => setTimeZone(event.target.value)} list="supported-time-zones" aria-invalid={!isValidTimeZone(timeZone)} /></label>
              <datalist id="supported-time-zones"><option value="UTC" /><option value="Asia/Kolkata" /><option value="America/Los_Angeles" /><option value="Europe/Berlin" /></datalist>
              {!isValidTimeZone(timeZone) && <small className="inline-warning">Unknown time zone; UTC is used for rendering.</small>}
            </div>
          </details>
        </div>
      </header>

      <main className="content" id="main-content">
        <section className="hero">
          <div><p className="eyebrow">Merchant operations / evidence first</p><h1>Make every withheld cent explainable.</h1><p className="hero-copy">Create an authenticated, idempotent quote, inspect a clearly isolated preview, review a governed rule release or adjustment, inspect finalized vault evidence, or repair a bounded indexer projection. The browser never treats a calculation as wallet authorization or settlement confirmation.</p></div>
          <div className="hero-badge"><span className="badge-label">Current boundary</span><strong>{workspace === "quote" ? "Authenticated quote" : workspace === "preview" ? "Rule preview" : workspace === "governance" ? "Rule governance" : workspace === "replay-ops" ? "Replay operations" : workspace === "vault-release" ? "Vault release control" : workspace === "vault-reconciliation" ? "Vault reconciliation" : workspace === "adjustments" ? "Settlement adjustments" : workspace === "refund-claims" ? "Refund claim wallet" : "Checkout authorization"}</strong><span>{workspace === "quote" ? "Persisted evidence and exact replay." : workspace === "preview" ? "Caller-supplied rules; never settlement authority." : workspace === "governance" ? "Server-owned lifecycle and audit trail." : workspace === "replay-ops" ? "Dead-letter review and bounded repair." : workspace === "vault-release" ? "Two-person claim lifecycle and reconciliation." : workspace === "vault-reconciliation" ? "Finalized read-only balance evidence." : workspace === "adjustments" ? "Governed financial evidence; no token movement." : workspace === "refund-claims" ? "Two wallet signatures; finality remains separate." : "Wallet review before any signature."}</span></div>
        </section>

        <nav className="workspace-tabs" aria-label="Tax workspace">
          <button type="button" className={workspace === "quote" ? "tab-active" : ""} aria-pressed={workspace === "quote"} aria-controls="workspace-content" onClick={() => { setWorkspace("quote"); clearResult(); }}>Production quote</button>
          <button type="button" className={workspace === "preview" ? "tab-active" : ""} aria-pressed={workspace === "preview"} aria-controls="workspace-content" onClick={() => { setWorkspace("preview"); clearResult(); }}>Isolated preview</button>
          <button type="button" className={workspace === "governance" ? "tab-active" : ""} aria-pressed={workspace === "governance"} aria-controls="workspace-content" onClick={() => setWorkspace("governance")}>Rule governance</button>
          <button type="button" className={workspace === "replay-ops" ? "tab-active" : ""} aria-pressed={workspace === "replay-ops"} aria-controls="workspace-content" onClick={() => setWorkspace("replay-ops")}>Replay operations</button>
          <button type="button" className={workspace === "vault-release" ? "tab-active" : ""} aria-pressed={workspace === "vault-release"} aria-controls="workspace-content" onClick={() => setWorkspace("vault-release")}>Vault releases</button>
          <button type="button" className={workspace === "vault-reconciliation" ? "tab-active" : ""} aria-pressed={workspace === "vault-reconciliation"} aria-controls="workspace-content" onClick={() => setWorkspace("vault-reconciliation")}>Vault reconciliation</button>
          <button type="button" className={workspace === "adjustments" ? "tab-active" : ""} aria-pressed={workspace === "adjustments"} aria-controls="workspace-content" onClick={() => setWorkspace("adjustments")}>Settlement adjustments</button>
          <button type="button" className={workspace === "refund-claims" ? "tab-active" : ""} aria-pressed={workspace === "refund-claims"} aria-controls="workspace-content" onClick={() => setWorkspace("refund-claims")}>Refund claims</button>
          <button type="button" className={workspace === "checkout" ? "tab-active" : ""} aria-pressed={workspace === "checkout"} aria-controls="workspace-content" onClick={() => setWorkspace("checkout")}>Checkout authorization</button>
        </nav>

        <div id="workspace-content" ref={workspaceContentRef} className="workspace-content" role="region" aria-label="Current tax workspace" tabIndex={-1}>
          <section className="metrics" aria-label="Platform status">
            <article className="metric-card"><span>Calculation status</span><strong>{statusLabel}</strong><small>Authoritative Rust domain engine</small></article>
            <article className="metric-card"><span>Settlement authority</span><strong>Wallet + Solana</strong><small>No client-side success is inferred</small></article>
            <article className="metric-card"><span>Evidence state</span><strong>{calculation ? (calculation.status === "attested" ? "Attested" : calculation.status === "calculated_unattested" ? "Unattested" : "Preview only") : "Not generated"}</strong><small>Attestation and finality are separate states</small></article>
          </section>

          <WorkspaceErrorBoundary key={workspace} workspace={workspace}>
          {workspace === "vault-reconciliation" ? <Suspense fallback={<section className="vault-layout" aria-live="polite"><article className="panel empty-state"><strong>Loading vault reconciliation…</strong><p>Finalized evidence is loaded only when selected.</p></article></section>}><VaultReconciliationPanel
          apiBase={API_BASE}
          serviceOnline={serviceState === "online"}
          locale={locale}
          timeZone={effectiveTimeZone}
        /></Suspense> : workspace === "governance" ? <Suspense fallback={<section className="governance-layout" aria-live="polite"><article className="panel empty-state"><strong>Loading governance workspace…</strong><p>The rule control plane is loaded only when selected.</p></article></section>}><GovernancePanel
          apiBase={API_BASE}
          serviceOnline={serviceState === "online"}
          locale={locale}
          timeZone={effectiveTimeZone}
        /></Suspense> : workspace === "replay-ops" ? <Suspense fallback={<section className="replay-layout" aria-live="polite"><article className="panel empty-state"><strong>Loading replay operations…</strong><p>The indexer control plane is loaded only when selected.</p></article></section>}><ReplayOpsPanel
          apiBase={API_BASE}
          serviceOnline={serviceState === "online"}
          locale={locale}
          timeZone={effectiveTimeZone}
        /></Suspense> : workspace === "vault-release" ? <Suspense fallback={<section className="vault-layout" aria-live="polite"><article className="panel empty-state"><strong>Loading vault release control…</strong><p>The governed treasury workspace is loaded only when selected.</p></article></section>}><VaultReleasePanel
          apiBase={API_BASE}
          serviceOnline={serviceState === "online"}
          accessToken={accessToken}
          locale={locale}
          timeZone={effectiveTimeZone}
          /></Suspense> : workspace === "adjustments" ? <Suspense fallback={<section className="adjustment-layout" aria-live="polite"><article className="panel empty-state"><strong>Loading settlement adjustments…</strong><p>The financial-evidence workspace is loaded only when selected.</p></article></section>}><SettlementAdjustmentPanel
          apiBase={API_BASE}
          serviceOnline={serviceState === "online"}
          accessToken={accessToken}
        /></Suspense> : workspace === "refund-claims" ? <Suspense fallback={<section className="adjustment-layout" aria-live="polite"><article className="panel empty-state"><strong>Loading refund claim wallet boundary…</strong><p>The two-signer wallet workflow is loaded only when selected.</p></article></section>}><RefundClaimPanel
          apiBase={API_BASE}
          serviceOnline={serviceState === "online"}
          accessToken={accessToken}
          locale={locale}
        /></Suspense> : workspace === "checkout" ? <Suspense fallback={<section className="checkout-layout" aria-live="polite"><article className="panel empty-state"><strong>Loading wallet checkout boundary…</strong><p>The signing client is loaded only when checkout is selected.</p></article></section>}><CheckoutPanel
          apiBase={API_BASE}
          serviceOnline={serviceState === "online"}
          accessToken={accessToken}
          calculation={calculation}
          formatAtomic={formatAtomic}
          locale={locale}
          timeZone={effectiveTimeZone}
        /></Suspense> : <section className="workspace-grid">
          <article className="panel calculator-panel">
            <div className="panel-heading"><div><p className="eyebrow">{workspace === "quote" ? "Published-rule quote" : "Scenario builder"}</p><h2>{workspace === "quote" ? "Create a persisted quote" : "Preview a tax calculation"}</h2></div><span className="status-pill">{workspace === "quote" ? "Authenticated" : "Preview"}</span></div>
            <p className="panel-copy">{workspace === "quote" ? "The API derives merchant scope from the bearer token, selects an effective published rule, and commits evidence with idempotency." : "This explicitly isolated route accepts caller-supplied rules for design analysis. It cannot publish policy or authorize funds."}</p>
            {workspace === "quote" && <label className="full-field"><span>OIDC access token (memory-only)</span><textarea value={accessToken} onChange={(event) => setAccessToken(event.target.value)} placeholder="Token with tax:quote scope" autoComplete="off" spellCheck={false} rows={3} /></label>}
            {workspace === "quote" && <label className="full-field"><span>Provider-signed jurisdiction evidence (JSON array)</span><textarea value={jurisdictionEvidenceJson} onChange={(event) => setJurisdictionEvidenceJson(event.target.value)} placeholder='[{"unsigned":...,"provider_public_key":"...","payload_digest":"...","signature":"..."}]' autoComplete="off" spellCheck={false} rows={6} aria-describedby="jurisdiction-evidence-help" /><small id="jurisdiction-evidence-help">Required for production quotes. The API verifies provider signatures, merchant binding, facts binding, validity, and the published tenant policy. The browser cannot assert jurisdiction by submitting a digest.</small></label>}
            <form onSubmit={submit} className="form-grid">
              <label><span>Gross amount (atomic units)</span><input inputMode="numeric" min="0" value={gross} onChange={(event) => setGross(event.target.value)} required /></label>
              <label><span>Net taxable base (atomic units)</span><input inputMode="numeric" min="0" value={net} onChange={(event) => setNet(event.target.value)} required /></label>
              {workspace === "preview" ? <label><span>Flat rate (%)</span><input inputMode="decimal" min="0" step="0.01" value={rate} onChange={(event) => setRate(event.target.value)} required /></label> : <label><span>Published rule set ID</span><input value={ruleSetId} onChange={(event) => setRuleSetId(event.target.value)} required /></label>}
              {workspace === "preview" ? <label><span>Jurisdiction</span><input value={jurisdiction} onChange={(event) => setJurisdiction(event.target.value.toUpperCase())} required /></label> : <label><span>Rule version</span><input inputMode="numeric" min="1" value={ruleVersion} onChange={(event) => setRuleVersion(event.target.value)} required /></label>}
              {workspace === "quote" && <><label><span>Jurisdiction</span><input value={jurisdiction} onChange={(event) => setJurisdiction(event.target.value.toUpperCase())} required /></label><label><span>Tax regime</span><input value={regime} onChange={(event) => setRegime(event.target.value)} required /></label><label className="full-field"><span>Idempotency key</span><input value={idempotencyKey} onChange={(event) => setIdempotencyKey(event.target.value)} required /></label></>}
              <button type="submit" disabled={isSubmitting || serviceState !== "online" || (workspace === "quote" && !accessToken.trim())}>{isSubmitting ? "Working…" : workspace === "quote" ? "Create quote" : "Calculate preview"}</button>
            </form>
            {serviceState === "offline" && <p className="inline-warning" role="alert">The service is unavailable. No client-side fallback is used.</p>}
            {error && <p className="inline-error" role="alert">{error}</p>}
          </article>

          <article className="panel result-panel" aria-live="polite">
            <div className="panel-heading"><div><p className="eyebrow">Calculation evidence</p><h2>Result trace</h2></div>{calculation && <span className={`status-pill ${calculation.status === "attested" ? "status-pill-success" : "status-pill-warning"}`}>{calculation.status === "attested" ? "Attested" : calculation.status === "calculated_unattested" ? "Unattested" : "Preview only"}</span>}</div>
            {calculation && <details className="tax-explanation"><summary>Why this tax?</summary><p>The Rust tax engine applied the published rule release to the stated jurisdiction and facts. This evidence explains the calculation; it is not tax advice, filing, or a compliance guarantee.</p><dl className="summary-list"><div><dt>Published rule</dt><dd>{calculation.result.rule_set_id} v{calculation.result.rule_version}</dd></div><div><dt>Rule digest</dt><dd><code>{shortDigest(calculation.result.rule_digest)}</code></dd></div><div><dt>Facts digest</dt><dd><code>{shortDigest(calculation.result.facts_digest)}</code></dd></div></dl><div className="trace-list">{calculation.result.components.map((component) => <div className="trace-row" key={component.id}><div><strong>{component.label}</strong><small>{component.explanation}</small></div><strong>{formatAtomic(component.tax_atomic, calculation.result.currency)}</strong></div>)}</div></details>}
            {!calculation ? <div className="empty-state"><span className="empty-icon">∑</span><strong>No calculation yet</strong><p>Submit the real workflow or run the isolated preview to inspect the exact evidence state.</p></div> : <div className="result-content"><div className="total-card"><span>Total tax withheld</span><strong>{formatAtomic(calculation.result.total_tax_atomic, calculation.result.currency)}</strong><small>Calculation ID {calculation.calculation_id}</small></div><dl className="summary-list"><div><dt>Jurisdiction</dt><dd>{calculation.result.jurisdiction}</dd></div><div><dt>Net after tax</dt><dd>{formatAtomic(calculation.result.total_net_atomic, calculation.result.currency)}</dd></div><div><dt>Engine version</dt><dd>{calculation.result.engine_version || "Legacy evidence"}</dd></div><div><dt>Policy state</dt><dd>{calculation.status === "attested" ? "Published rule and issuer attestation" : calculation.status === "calculated_unattested" ? "Published rule, not attested" : "Caller-supplied preview"}</dd></div></dl>{["calculated_unattested", "attested"].includes(calculation.status) && <div className="evidence-actions"><button type="button" onClick={verifyPersistedReplay} disabled={isReplaying || !accessToken.trim()}>{isReplaying ? "Verifying evidence…" : replayVerification ? "Replay verified" : "Verify persisted replay"}</button><p className="evidence-note">Replays the database-owned facts, published rule, engine version, and complete component trace. This does not attest or settle funds.</p>{replayVerification && <dl className="summary-list evidence-summary"><div><dt>Replay status</dt><dd>{replayVerification.status}</dd></div><div><dt>Facts digest</dt><dd><code>{shortDigest(replayVerification.facts_digest)}</code></dd></div><div><dt>Rule digest</dt><dd><code>{shortDigest(replayVerification.rule_digest)}</code></dd></div></dl>}</div>}{error && <p className="inline-error" role="alert">{error}</p>}<div className="trace-list">{calculation.result.components.map((component) => <div className="trace-row" key={component.id}><div><strong>{component.label}</strong><small>{component.explanation}</small></div><strong>{formatAtomic(component.tax_atomic, calculation.result.currency)}</strong></div>)}</div></div>}
          </article>
          </section>}
          </WorkspaceErrorBoundary>
        </div>
      </main>
      <footer className="footer"><span>Calculation authority: Rust service</span><span>Tax calculation ≠ tax filing or legal advice</span></footer>
    </div>
  );
}
