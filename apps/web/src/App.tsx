import { FormEvent, Suspense, lazy, useEffect, useRef, useState } from "react";
import { fetchWithTimeout, REQUEST_TIMEOUT_MS, responseError } from "./api";
import {
  DEFAULT_TIME_ZONE,
  directionForLocale,
  formatAtomic as formatAtomicValue,
  isValidTimeZone,
} from "./formatters";
import WorkspaceErrorBoundary from "./WorkspaceErrorBoundary";
import { createClientUuid } from "./clientIdentity";
import ProductShell, { ProductWorkspace } from "./ProductShell";
import { isAuthenticationCallback, pathForWorkspace, workspaceFromPath } from "./navigation";
import AccessControlPanel from "./AccessControlPanel";
import { authorizationSnapshotForToken, AuthorizationSnapshot, parseAuthorizationCapabilities } from "./authorization";
import { BROWSER_RUNTIME_CONTEXT } from "./runtimeContext";
import { BROWSER_AUTH_CONFIG } from "./authConfig";
import { AuthSessionStatus } from "./AuthSessionControl";
import { beginOidcAuthorization, completeOidcCallback, PKCE_TRANSACTION_STORAGE_KEY } from "./oidcSession";
import { loadDisplayPreferences, saveDisplayPreferences } from "./displayPreferences";
import {
  Calculation,
  parseCalculationResponse,
  parseReplayVerificationResponse,
  ReplayVerification,
} from "./taxResponses";

const CheckoutPanel = lazy(() => import("./CheckoutPanel"));
const DocumentQuotePanel = lazy(() => import("./DocumentQuotePanel"));
const DocumentCheckoutPanel = lazy(() => import("./DocumentCheckoutPanel"));
const GovernancePanel = lazy(() => import("./GovernancePanel"));
const ReplayOpsPanel = lazy(() => import("./ReplayOpsPanel"));
const VaultReleasePanel = lazy(() => import("./VaultReleasePanel"));
const VaultReconciliationPanel = lazy(() => import("./VaultReconciliationPanel"));
const RevenueStreamingPanel = lazy(() => import("./RevenueStreamingPanel"));
const SettlementAdjustmentPanel = lazy(() => import("./SettlementAdjustmentPanel"));
const RefundClaimPanel = lazy(() => import("./RefundClaimPanel"));
const RefundExecutePanel = lazy(() => import("./RefundExecutePanel"));
const RefundCancellationPanel = lazy(() => import("./RefundCancellationPanel"));

type Workspace = ProductWorkspace;
type ServiceState = "checking" | "online" | "offline";

const API_BASE = import.meta.env.VITE_API_BASE ?? "/api";

function shortDigest(value: number[]): string {
  return value.map((byte) => byte.toString(16).padStart(2, "0")).join("").slice(0, 16) + "…";
}

export default function App() {
  const [serviceState, setServiceState] = useState<ServiceState>("checking");
  const [workspace, setWorkspace] = useState<Workspace>(() => workspaceFromPath(window.location.pathname));
  const workspaceContentRef = useRef<HTMLDivElement>(null);
  const initialWorkspaceRender = useRef(true);
  const [gross, setGross] = useState("");
  const [net, setNet] = useState("");
  const [currency, setCurrency] = useState("");
  const [rate, setRate] = useState("");
  const [jurisdiction, setJurisdiction] = useState("");
  const [ruleSetId, setRuleSetId] = useState("");
  const [ruleVersion, setRuleVersion] = useState("");
  const [regime, setRegime] = useState("");
  const [jurisdictionEvidenceJson, setJurisdictionEvidenceJson] = useState("[]");
  const [accessToken, setAccessToken] = useState("");
  const [authSessionStatus, setAuthSessionStatus] = useState<AuthSessionStatus>(() => BROWSER_AUTH_CONFIG.mode === "unconfigured" ? "error" : "signed-out");
  const [authSessionError, setAuthSessionError] = useState<string | null>(null);
  const [authSessionExpiresAt, setAuthSessionExpiresAt] = useState<number | null>(null);
  const [authorizationSnapshot, setAuthorizationSnapshot] = useState<AuthorizationSnapshot>(() => authorizationSnapshotForToken(""));
  const [authorizationError, setAuthorizationError] = useState<string | null>(null);
  const [idempotencyKey, setIdempotencyKey] = useState<string>(() => createClientUuid());
  const [calculation, setCalculation] = useState<Calculation | null>(null);
  const [documentCalculationId, setDocumentCalculationId] = useState("");
  const [replayVerification, setReplayVerification] = useState<ReplayVerification | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [isReplaying, setIsReplaying] = useState(false);
  const [displayPreferences, setDisplayPreferences] = useState(loadDisplayPreferences);
  const { locale, timeZone, theme: themePreference } = displayPreferences;

  const effectiveTimeZone = isValidTimeZone(timeZone) ? timeZone : DEFAULT_TIME_ZONE;
  const direction = directionForLocale(locale);

  useEffect(() => {
    const initialWorkspace = workspaceFromPath(window.location.pathname);
    const canonicalPath = pathForWorkspace(initialWorkspace);
    if (!isAuthenticationCallback(window.location.search) && window.location.pathname !== canonicalPath) {
      window.history.replaceState({ workspace: initialWorkspace }, "", canonicalPath);
    }
    const onPopState = () => setWorkspace(workspaceFromPath(window.location.pathname));
    window.addEventListener("popstate", onPopState);
    return () => window.removeEventListener("popstate", onPopState);
  }, []);

  function setBrowserCredential(value: string) {
    setAccessToken(value);
    setAuthSessionExpiresAt(null);
    setAuthSessionError(null);
    setAuthSessionStatus(value.trim() ? "signed-in" : "signed-out");
  }

  async function signIn() {
    if (BROWSER_AUTH_CONFIG.mode !== "oidc" || !BROWSER_AUTH_CONFIG.oidc) {
      setAuthSessionStatus("error");
      setAuthSessionError("Sign-in is not configured for this deployment.");
      return;
    }
    setAuthSessionStatus("signing-in");
    setAuthSessionError(null);
    try {
      const authorizationUrl = await beginOidcAuthorization(BROWSER_AUTH_CONFIG.oidc, window.sessionStorage);
      window.location.assign(authorizationUrl);
    } catch (cause) {
      setAuthSessionStatus("error");
      setAuthSessionError(cause instanceof Error ? cause.message : "Sign-in could not be started.");
    }
  }

  function signOut() {
    setAccessToken("");
    setAuthSessionExpiresAt(null);
    setAuthSessionError(null);
    setAuthSessionStatus("signed-out");
    setAuthorizationSnapshot(authorizationSnapshotForToken(""));
    try {
      window.sessionStorage.removeItem(PKCE_TRANSACTION_STORAGE_KEY);
    } catch {
      // A failed cleanup cannot preserve an access token: it is memory-only.
    }
    if (window.location.search) window.history.replaceState(null, document.title, `${window.location.pathname}${window.location.hash}`);
  }

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
    document.documentElement.dataset.theme = themePreference;
    try {
      saveDisplayPreferences(window.localStorage, displayPreferences);
    } catch {
      // Appearance remains usable when browser persistence is unavailable.
    }
  }, [displayPreferences, themePreference]);

  useEffect(() => {
    if (BROWSER_AUTH_CONFIG.mode !== "oidc" || !BROWSER_AUTH_CONFIG.oidc) return;
    const callbackUrl = new URL(window.location.href);
    if (!callbackUrl.searchParams.has("code") && !callbackUrl.searchParams.has("error")) return;
    let active = true;
    setAuthSessionStatus("signing-in");
    setAuthSessionError(null);
    try {
      completeOidcCallback(BROWSER_AUTH_CONFIG.oidc, callbackUrl, window.sessionStorage)
        .then((token) => {
          if (!active) return;
          setAccessToken(token.accessToken);
          setAuthSessionExpiresAt(token.expiresAtUnix);
          setAuthSessionStatus("signed-in");
          window.history.replaceState(null, document.title, `${callbackUrl.pathname}${callbackUrl.hash}`);
        })
        .catch((cause: unknown) => {
          if (!active) return;
          setAccessToken("");
          setAuthSessionExpiresAt(null);
          setAuthSessionStatus("error");
          setAuthSessionError(cause instanceof Error ? cause.message : "Sign-in could not be completed.");
        });
    } catch (cause) {
      setAccessToken("");
      setAuthSessionExpiresAt(null);
      setAuthSessionStatus("error");
      setAuthSessionError(cause instanceof Error ? cause.message : "Sign-in could not be completed.");
    }
    return () => { active = false; };
  }, []);

  useEffect(() => {
    if (!authSessionExpiresAt) return;
    const delay = Math.max(0, (authSessionExpiresAt - Math.floor(Date.now() / 1000)) * 1000);
    const timer = globalThis.setTimeout(() => {
      setAccessToken("");
      setAuthSessionExpiresAt(null);
      setAuthSessionStatus("error");
      setAuthSessionError("Your sign-in expired. Sign in again before continuing.");
    }, delay);
    return () => globalThis.clearTimeout(timer);
  }, [authSessionExpiresAt]);

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
    let active = true;
    const controller = new AbortController();
    const token = accessToken.trim();
    if (!token) {
      setAuthorizationSnapshot(authorizationSnapshotForToken(""));
      setAuthorizationError(null);
      return () => controller.abort();
    }
    setAuthorizationSnapshot(authorizationSnapshotForToken(token));
    setAuthorizationError(null);
    const timer = globalThis.setTimeout(() => {
      fetchWithTimeout(`${API_BASE}/v1/authorization/capabilities`, {
        headers: { authorization: `Bearer ${token}` },
        signal: controller.signal,
      }, REQUEST_TIMEOUT_MS.default)
        .then(async (response) => {
          const payload: unknown = await response.json().catch(() => null);
          if (!response.ok) throw new Error(responseError(response, payload, "Access could not be checked."));
          return parseAuthorizationCapabilities(payload);
        })
        .then((snapshot) => {
          if (active) setAuthorizationSnapshot(snapshot);
        })
        .catch((cause: unknown) => {
          if (active && !(cause instanceof DOMException && cause.name === "AbortError")) {
            setAuthorizationError(cause instanceof Error ? cause.message : "Access could not be checked.");
          }
        });
    }, 250);
    return () => {
      active = false;
      globalThis.clearTimeout(timer);
      controller.abort();
    };
  }, [accessToken]);

  useEffect(() => {
    if (initialWorkspaceRender.current) {
      initialWorkspaceRender.current = false;
      return;
    }
    workspaceContentRef.current?.focus();
  }, [workspace]);

  useEffect(() => {
    if (workspace === "quote") setIdempotencyKey(createClientUuid());
  }, [workspace, gross, net, currency, ruleSetId, ruleVersion, jurisdiction, regime, jurisdictionEvidenceJson]);

  function clearResult() {
    setError(null);
    setCalculation(null);
    setReplayVerification(null);
  }

  function navigateToWorkspace(nextWorkspace: Workspace, preserveCurrentResult = false) {
    const nextPath = pathForWorkspace(nextWorkspace);
    if (window.location.pathname !== nextPath) {
      window.history.pushState({ workspace: nextWorkspace }, "", nextPath);
    }
    setWorkspace(nextWorkspace);
    if (!preserveCurrentResult) clearResult();
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
          facts: { gross_atomic: gross.trim(), net_atomic: net.trim(), currency: currency.trim().toUpperCase(), jurisdiction },
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
      setCalculation(parseCalculationResponse(payload));
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
          facts: { gross_atomic: gross.trim(), net_atomic: net.trim(), currency: currency.trim().toUpperCase(), jurisdiction },
          jurisdiction_evidence: jurisdictionEvidence
        })
      });
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw new Error(responseError(response, payload, `Quote request failed (${response.status})`));
      setCalculation(parseCalculationResponse(payload));
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
      setReplayVerification(parseReplayVerificationResponse(payload));
    } catch (cause) {
      setReplayVerification(null);
      setError(cause instanceof Error ? cause.message : "Replay verification failed");
    } finally {
      setIsReplaying(false);
    }
  }

  const submit = workspace === "quote" ? createQuote : calculatePreview;
  const formatAtomic = (value: string, currency: string) => formatAtomicValue(value, currency, locale);

  return (
    <ProductShell
      workspace={workspace}
      onNavigate={(nextWorkspace) => {
        const preservesQuoteHandoff = (workspace === "quote" && nextWorkspace === "checkout") || (workspace === "checkout" && nextWorkspace === "quote");
        navigateToWorkspace(nextWorkspace, preservesQuoteHandoff);
      }}
      serviceState={serviceState}
      accessToken={accessToken}
      authorizationSnapshot={authorizationSnapshot}
      authorizationError={authorizationError}
      runtimeContext={BROWSER_RUNTIME_CONTEXT}
      authConfig={BROWSER_AUTH_CONFIG}
      authSessionStatus={authSessionStatus}
      authSessionError={authSessionError}
      onCredentialChange={setBrowserCredential}
      onSignIn={signIn}
      onSignOut={signOut}
      locale={locale}
      timeZone={timeZone}
      onLocaleChange={(nextLocale) => setDisplayPreferences((current) => ({ ...current, locale: nextLocale }))}
      onTimeZoneChange={(nextTimeZone) => setDisplayPreferences((current) => ({ ...current, timeZone: nextTimeZone }))}
      themePreference={themePreference}
      onThemeChange={(nextTheme) => setDisplayPreferences((current) => ({ ...current, theme: nextTheme }))}
    >
        <div id="workspace-content" ref={workspaceContentRef} className="workspace-content" role="region" aria-label="Current tax workspace" tabIndex={-1}>
          <WorkspaceErrorBoundary key={workspace} workspace={workspace}>
          {workspace === "access-control" ? <AccessControlPanel accessToken={accessToken} authorizationSnapshot={authorizationSnapshot} authorizationError={authorizationError} /> : workspace === "revenue-streaming" ? <Suspense fallback={<section className="vault-layout" aria-live="polite"><article className="panel empty-state"><strong>Loading partner payouts…</strong><p>Governed payout plans and verified balance controls are loaded only when selected.</p></article></section>}><RevenueStreamingPanel accessToken={accessToken} authorizationSnapshot={authorizationSnapshot} authorizationError={authorizationError} /></Suspense> : workspace === "document-quote" ? <Suspense fallback={<section className="workspace-grid" aria-live="polite"><article className="panel empty-state"><strong>Loading document quote…</strong><p>Authenticated document facts and evidence controls are loaded only when selected.</p></article></section>}><DocumentQuotePanel
          apiBase={API_BASE}
          serviceOnline={serviceState === "online"}
          accessToken={accessToken}
          formatAtomic={formatAtomic}
          locale={locale}
          timeZone={effectiveTimeZone}
          onOpenCheckout={(calculationId) => { setDocumentCalculationId(calculationId); navigateToWorkspace("document-checkout", true); }}
        /></Suspense> : workspace === "document-checkout" ? <Suspense fallback={<section className="checkout-layout" aria-live="polite"><article className="panel empty-state"><strong>Loading document checkout…</strong><p>Document attestation and payment scope controls are loaded only when selected.</p></article></section>}><DocumentCheckoutPanel
          apiBase={API_BASE}
          serviceOnline={serviceState === "online"}
          accessToken={accessToken}
          initialCalculationId={documentCalculationId}
          locale={locale}
          timeZone={effectiveTimeZone}
          /></Suspense> : workspace === "vault-reconciliation" ? <Suspense fallback={<section className="vault-layout" aria-live="polite"><article className="panel empty-state"><strong>Loading vault reconciliation…</strong><p>Finalized evidence is loaded only when selected.</p></article></section>}><VaultReconciliationPanel
          apiBase={API_BASE}
          serviceOnline={serviceState === "online"}
          accessToken={accessToken}
          locale={locale}
          timeZone={effectiveTimeZone}
        /></Suspense> : workspace === "governance" ? <Suspense fallback={<section className="governance-layout" aria-live="polite"><article className="panel empty-state"><strong>Loading tax rules…</strong><p>The rule review workspace is loaded only when selected.</p></article></section>}><GovernancePanel
          apiBase={API_BASE}
          serviceOnline={serviceState === "online"}
          accessToken={accessToken}
          locale={locale}
          timeZone={effectiveTimeZone}
        /></Suspense> : workspace === "replay-ops" ? <Suspense fallback={<section className="replay-layout" aria-live="polite"><article className="panel empty-state"><strong>Loading exceptions and recovery…</strong><p>The recovery workspace is loaded only when selected.</p></article></section>}><ReplayOpsPanel
          apiBase={API_BASE}
          serviceOnline={serviceState === "online"}
          accessToken={accessToken}
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
        /></Suspense> : workspace === "refund-claims" ? <Suspense fallback={<section className="adjustment-layout" aria-live="polite"><article className="panel empty-state"><strong>Loading refund request…</strong><p>The approval and wallet steps are loaded only when selected.</p></article></section>}><RefundClaimPanel
          apiBase={API_BASE}
          serviceOnline={serviceState === "online"}
          accessToken={accessToken}
          locale={locale}
          /></Suspense> : workspace === "refund-cancellations" ? <Suspense fallback={<section className="adjustment-layout" aria-live="polite"><article className="panel empty-state"><strong>Loading refund cancellation…</strong><p>The approval step is loaded only when selected.</p></article></section>}><RefundCancellationPanel
          apiBase={API_BASE}
          serviceOnline={serviceState === "online"}
          accessToken={accessToken}
          /></Suspense> : workspace === "refund-executions" ? <Suspense fallback={<section className="adjustment-layout" aria-live="polite"><article className="panel empty-state"><strong>Loading refund payment…</strong><p>Payment is loaded only when selected and remains evidence-gated.</p></article></section>}><RefundExecutePanel
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
            <div className="panel-heading"><div><p className="eyebrow">{workspace === "quote" ? "Customer quote" : "Try a scenario"}</p><h2>{workspace === "quote" ? "Create a tax quote" : "Explore a tax scenario"}</h2></div><span className="status-pill">{workspace === "quote" ? "Ready to review" : "No live changes"}</span></div>
            <p className="panel-copy">{workspace === "quote" ? "Create a quote from an approved tax rule. Your account and the quote details are checked by the service, and the result can be reviewed later." : "Explore a what-if scenario. It does not change a live rule or authorize a payment."}</p>
            {workspace === "quote" && BROWSER_AUTH_CONFIG.mode === "unconfigured" && <p className="inline-warning" role="alert">Sign-in is not configured for this deployment. Protected quote actions stay unavailable.</p>}
            {workspace === "quote" && <details className="route-evidence full-field"><summary>Location verification</summary><p className="field-help">A production quote requires a record from an approved location provider. This advanced exchange keeps the signed record intact for server verification.</p><label htmlFor="aggregate-jurisdiction-evidence"><span>Provider-signed location record (advanced)</span><textarea id="aggregate-jurisdiction-evidence" value={jurisdictionEvidenceJson} onChange={(event) => setJurisdictionEvidenceJson(event.target.value)} placeholder="Paste the signed record from your approved provider" autoComplete="off" spellCheck={false} rows={6} aria-describedby="jurisdiction-evidence-help" /><small id="jurisdiction-evidence-help">Required for production quotes. The service checks the source, account, dates, and location before it uses this record.</small></label></details>}
            <form onSubmit={submit} className="form-grid">
              <label><span>Total before tax</span><input inputMode="numeric" min="0" value={gross} onChange={(event) => setGross(event.target.value)} placeholder="Enter the total amount" aria-describedby="amount-help" required /></label>
              <label><span>Amount subject to tax</span><input inputMode="numeric" min="0" value={net} onChange={(event) => setNet(event.target.value)} placeholder="Enter the taxable amount" aria-describedby="amount-help" required /></label>
              <label><span>Currency</span><input value={currency} onChange={(event) => setCurrency(event.target.value.toUpperCase())} placeholder="Enter the currency code" maxLength={12} required /></label>
              {workspace === "preview" ? <label><span>Tax rate (%)</span><input inputMode="decimal" min="0" step="0.01" value={rate} onChange={(event) => setRate(event.target.value)} placeholder="Enter a rate" required /></label> : <label><span>Approved tax rule</span><input value={ruleSetId} onChange={(event) => setRuleSetId(event.target.value)} placeholder="Enter the approved rule reference" required /></label>}
              {workspace === "preview" ? <label><span>Jurisdiction</span><input value={jurisdiction} onChange={(event) => setJurisdiction(event.target.value.toUpperCase())} placeholder="Enter a jurisdiction" required /></label> : <label><span>Rule version</span><input inputMode="numeric" min="1" value={ruleVersion} onChange={(event) => setRuleVersion(event.target.value)} placeholder="Enter the rule version" required /></label>}
              {workspace === "quote" && <><label><span>Jurisdiction</span><input value={jurisdiction} onChange={(event) => setJurisdiction(event.target.value.toUpperCase())} placeholder="Enter a jurisdiction" required /></label><label><span>Tax category</span><input value={regime} onChange={(event) => setRegime(event.target.value)} placeholder="Enter the tax category" required /></label><div className="route-evidence full-field"><strong>Retry protection selected automatically</strong><small>A secure retry reference is assigned automatically and stays the same while these quote details remain unchanged.</small></div></>}
              <small id="amount-help" className="field-help full-field">Enter amounts in the smallest supported account unit. The service keeps the calculation exact and does not infer missing values.</small>
              <button type="submit" disabled={isSubmitting || serviceState !== "online" || (workspace === "quote" && !accessToken.trim())}>{isSubmitting ? "Preparing…" : workspace === "quote" ? "Prepare quote" : "Calculate scenario"}</button>
            </form>
            {serviceState === "offline" && <p className="inline-warning" role="alert">The service is unavailable. No client-side fallback is used.</p>}
            {error && <p className="inline-error" role="alert">{error}</p>}
          </article>

          <article className="panel result-panel" aria-live="polite">
            <div className="panel-heading"><div><p className="eyebrow">Your tax breakdown</p><h2>Review the result</h2></div>{calculation && <span className={`status-pill ${calculation.status === "attested" ? "status-pill-success" : "status-pill-warning"}`}>{calculation.status === "attested" ? "Verified" : calculation.status === "calculated_unattested" ? "Needs verification" : "Scenario only"}</span>}</div>
            {calculation && <details className="tax-explanation"><summary>Why this tax?</summary><p>The service applied the approved tax rule to the location and amounts you entered. This explanation helps you review the result; it is not tax advice, a filing, or a compliance guarantee.</p><dl className="summary-list"><div><dt>Approved tax rule</dt><dd>{calculation.result.rule_set_id} v{calculation.result.rule_version}</dd></div>{calculation.result.fx_conversion && <div><dt>Currency conversion</dt><dd>{calculation.result.fx_conversion.source_currency} → {calculation.result.fx_conversion.target_currency} · {calculation.result.fx_conversion.provider} · {calculation.result.fx_conversion.quote_id}</dd></div>}</dl><div className="trace-list">{calculation.result.components.map((component) => <div className="trace-row" key={component.id}><div><strong>{component.label}</strong><small>{component.explanation}</small></div><strong>{formatAtomic(component.tax_atomic, calculation.result.currency)}</strong></div>)}</div><details className="technical-details"><summary>Record fingerprints</summary><dl className="summary-list"><div><dt>Rule fingerprint</dt><dd><code>{shortDigest(calculation.result.rule_digest)}</code></dd></div><div><dt>Input fingerprint</dt><dd><code>{shortDigest(calculation.result.facts_digest)}</code></dd></div></dl></details></details>}
            {!calculation ? <div className="empty-state"><span className="empty-icon">∑</span><strong>Your result will appear here</strong><p>Prepare a quote or explore a scenario to review the tax amount and its explanation.</p></div> : <div className="result-content"><div className="total-card"><span>Total tax</span><strong>{formatAtomic(calculation.result.total_tax_atomic, calculation.result.currency)}</strong><small>Reference {calculation.calculation_id}</small></div><dl className="summary-list"><div><dt>Jurisdiction</dt><dd>{calculation.result.jurisdiction}</dd></div><div><dt>Amount after tax</dt><dd>{formatAtomic(calculation.result.total_net_atomic, calculation.result.currency)}</dd></div><div><dt>Result status</dt><dd>{calculation.status === "attested" ? "Verified against approved evidence" : calculation.status === "calculated_unattested" ? "Calculated; verification still needed" : "Scenario supplied for exploration"}</dd></div></dl><details className="technical-details"><summary>Technical and audit details</summary><dl className="summary-list"><div><dt>Calculation reference</dt><dd><code>{calculation.calculation_id}</code></dd></div><div><dt>Engine version</dt><dd>{calculation.result.engine_version || "Legacy evidence"}</dd></div><div><dt>Policy state</dt><dd>{calculation.status === "attested" ? "Published rule and issuer attestation" : calculation.status === "calculated_unattested" ? "Published rule, not attested" : "Caller-supplied preview"}</dd></div></dl></details>{["calculated_unattested", "attested"].includes(calculation.status) && <div className="evidence-actions"><button type="button" onClick={verifyPersistedReplay} disabled={isReplaying || !accessToken.trim()}>{isReplaying ? "Checking record…" : replayVerification ? "Record checked" : "Check saved record"}</button><p className="evidence-note">This checks the saved facts, approved rule, engine version, and full component breakdown. It does not approve or move funds.</p>{replayVerification && <dl className="summary-list evidence-summary"><div><dt>Check status</dt><dd>{replayVerification.status}</dd></div><div><dt>Facts digest</dt><dd><code>{shortDigest(replayVerification.facts_digest)}</code></dd></div><div><dt>Rule digest</dt><dd><code>{shortDigest(replayVerification.rule_digest)}</code></dd></div></dl>}</div>}{error && <p className="inline-error" role="alert">{error}</p>}<div className="trace-list">{calculation.result.components.map((component) => <div className="trace-row" key={component.id}><div><strong>{component.label}</strong><small>{component.explanation}</small></div><strong>{formatAtomic(component.tax_atomic, calculation.result.currency)}</strong></div>)}</div></div>}
          </article>
          </section>}
          </WorkspaceErrorBoundary>
        </div>
    </ProductShell>
  );
}
