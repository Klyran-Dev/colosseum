import { FormEvent, useEffect, useState } from "react";

type Workspace = "quote" | "preview";
type ServiceState = "checking" | "online" | "offline";

type Calculation = {
  calculation_id: string;
  status: "calculated_unattested" | "simulated_unattested";
  result: {
    currency: string;
    jurisdiction: string;
    facts_digest: number[];
    rule_digest: number[];
    total_tax_atomic: string;
    total_net_atomic: string;
    components: Array<{
      id: string;
      label: string;
      base_atomic: string;
      tax_atomic: string;
      explanation: string;
    }>;
  };
};

const API_BASE = import.meta.env.VITE_API_BASE ?? "/api";

function formatAtomic(value: string, currency: string): string {
  const atomic = BigInt(value);
  const sign = atomic < 0n ? "-" : "";
  const absolute = atomic < 0n ? -atomic : atomic;
  const whole = absolute / 100n;
  const fraction = (absolute % 100n).toString().padStart(2, "0");
  return `${sign}${currency} ${new Intl.NumberFormat(undefined).format(whole)}.${fraction}`;
}

function responseError(payload: unknown, fallback: string): string {
  if (
    typeof payload === "object" &&
    payload !== null &&
    "error" in payload &&
    typeof payload.error === "object" &&
    payload.error !== null &&
    "message" in payload.error &&
    typeof payload.error.message === "string"
  ) {
    return payload.error.message;
  }
  return fallback;
}

export default function App() {
  const [serviceState, setServiceState] = useState<ServiceState>("checking");
  const [workspace, setWorkspace] = useState<Workspace>("quote");
  const [gross, setGross] = useState("1100");
  const [net, setNet] = useState("1000");
  const [rate, setRate] = useState("10");
  const [jurisdiction, setJurisdiction] = useState("US-CA");
  const [ruleSetId, setRuleSetId] = useState("operator-simulator-flat-rate");
  const [ruleVersion, setRuleVersion] = useState("1");
  const [regime, setRegime] = useState("sales-tax");
  const [accessToken, setAccessToken] = useState("");
  const [idempotencyKey, setIdempotencyKey] = useState<string>(() => crypto.randomUUID());
  const [calculation, setCalculation] = useState<Calculation | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);

  useEffect(() => {
    let active = true;
    fetch(`${API_BASE}/healthz`, { signal: AbortSignal.timeout(2_000) })
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

  function clearResult() {
    setError(null);
    setCalculation(null);
  }

  async function calculatePreview(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    clearResult();
    setIsSubmitting(true);
    try {
      const response = await fetch(`${API_BASE}/v1/tax/calculate`, {
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
      if (!response.ok) throw new Error(responseError(payload, "Preview calculation failed"));
      setCalculation(payload as Calculation);
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
    setIsSubmitting(true);
    try {
      const response = await fetch(`${API_BASE}/v1/tax/quote`, {
        method: "POST",
        headers: { "content-type": "application/json", authorization: `Bearer ${accessToken.trim()}` },
        body: JSON.stringify({
          idempotency_key: idempotencyKey.trim(),
          rule_set_id: ruleSetId.trim(),
          rule_version: Number(ruleVersion),
          jurisdiction,
          regime: regime.trim(),
          facts: { gross_atomic: gross.trim(), net_atomic: net.trim(), currency: "USD", jurisdiction }
        })
      });
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw new Error(responseError(payload, `Quote request failed (${response.status})`));
      setCalculation(payload as Calculation);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Quote request failed");
    } finally {
      setIsSubmitting(false);
    }
  }

  const serviceLabel = { checking: "Checking service", online: "Calculation service online", offline: "Calculation service unavailable" }[serviceState];
  const submit = workspace === "quote" ? createQuote : calculatePreview;
  const statusLabel = calculation?.status === "calculated_unattested" ? "Calculated" : calculation ? "Preview" : "Awaiting input";

  return (
    <div className="app-shell">
      <header className="topbar">
        <a className="brand" href="/" aria-label="Solana Tax Protocol home"><span className="brand-mark">S</span><span><strong>Solana Tax Protocol</strong><small>Tax and settlement operations</small></span></a>
        <div className={`service-status service-${serviceState}`} role="status" aria-live="polite"><span className="status-dot" />{serviceLabel}</div>
      </header>

      <main className="content">
        <section className="hero">
          <div><p className="eyebrow">Merchant operations / evidence first</p><h1>Make every withheld cent explainable.</h1><p className="hero-copy">Create an authenticated, idempotent quote from a published rule or inspect a clearly isolated preview. The browser never treats a calculation as wallet authorization or settlement confirmation.</p></div>
          <div className="hero-badge"><span className="badge-label">Current boundary</span><strong>{workspace === "quote" ? "Authenticated quote" : "Rule preview"}</strong><span>{workspace === "quote" ? "Persisted evidence and exact replay." : "Caller-supplied rules; never settlement authority."}</span></div>
        </section>

        <nav className="workspace-tabs" aria-label="Tax workspace">
          <button type="button" className={workspace === "quote" ? "tab-active" : ""} onClick={() => { setWorkspace("quote"); clearResult(); }}>Production quote</button>
          <button type="button" className={workspace === "preview" ? "tab-active" : ""} onClick={() => { setWorkspace("preview"); clearResult(); }}>Isolated preview</button>
        </nav>

        <section className="metrics" aria-label="Platform status">
          <article className="metric-card"><span>Calculation status</span><strong>{statusLabel}</strong><small>Authoritative Rust domain engine</small></article>
          <article className="metric-card"><span>Settlement authority</span><strong>Wallet + Solana</strong><small>No client-side success is inferred</small></article>
          <article className="metric-card"><span>Evidence state</span><strong>{calculation ? (calculation.status === "calculated_unattested" ? "Unattested" : "Preview only") : "Not generated"}</strong><small>Attestation and finality are separate states</small></article>
        </section>

        <section className="workspace-grid">
          <article className="panel calculator-panel">
            <div className="panel-heading"><div><p className="eyebrow">{workspace === "quote" ? "Published-rule quote" : "Scenario builder"}</p><h2>{workspace === "quote" ? "Create a persisted quote" : "Preview a tax calculation"}</h2></div><span className="status-pill">{workspace === "quote" ? "Authenticated" : "Preview"}</span></div>
            <p className="panel-copy">{workspace === "quote" ? "The API derives merchant scope from the bearer token, selects an effective published rule, and commits evidence with idempotency." : "This explicitly isolated route accepts caller-supplied rules for design analysis. It cannot publish policy or authorize funds."}</p>
            {workspace === "quote" && <label className="full-field"><span>OIDC access token (memory-only)</span><textarea value={accessToken} onChange={(event) => setAccessToken(event.target.value)} placeholder="Token with tax:quote scope" autoComplete="off" spellCheck={false} rows={3} /></label>}
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
            <div className="panel-heading"><div><p className="eyebrow">Calculation evidence</p><h2>Result trace</h2></div>{calculation && <span className="status-pill status-pill-warning">{calculation.status === "calculated_unattested" ? "Unattested" : "Preview only"}</span>}</div>
            {!calculation ? <div className="empty-state"><span className="empty-icon">∑</span><strong>No calculation yet</strong><p>Submit the real workflow or run the isolated preview to inspect the exact evidence state.</p></div> : <div className="result-content"><div className="total-card"><span>Total tax withheld</span><strong>{formatAtomic(calculation.result.total_tax_atomic, calculation.result.currency)}</strong><small>Calculation ID {calculation.calculation_id}</small></div><dl className="summary-list"><div><dt>Jurisdiction</dt><dd>{calculation.result.jurisdiction}</dd></div><div><dt>Net after tax</dt><dd>{formatAtomic(calculation.result.total_net_atomic, calculation.result.currency)}</dd></div><div><dt>Policy state</dt><dd>{calculation.status === "calculated_unattested" ? "Published rule, not attested" : "Caller-supplied preview"}</dd></div></dl><div className="trace-list">{calculation.result.components.map((component) => <div className="trace-row" key={component.id}><div><strong>{component.label}</strong><small>{component.explanation}</small></div><strong>{formatAtomic(component.tax_atomic, calculation.result.currency)}</strong></div>)}</div></div>}
          </article>
        </section>
      </main>
      <footer className="footer"><span>Calculation authority: Rust service</span><span>Tax calculation ≠ tax filing or legal advice</span></footer>
    </div>
  );
}
