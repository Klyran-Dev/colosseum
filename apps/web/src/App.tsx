import { FormEvent, Suspense, lazy, useEffect, useRef, useState } from "react";
import { fetchWithTimeout, operatorResponseError, REQUEST_TIMEOUT_MS } from "./api";
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
import { AuthorizationCapabilitiesError, authorizationSnapshotForToken, authorizationSnapshotRefreshDelayMs, AuthorizationSnapshot, parseAuthorizationCapabilities } from "./authorization";
import { BROWSER_RUNTIME_CONTEXT } from "./runtimeContext";
import { BROWSER_AUTH_CONFIG } from "./authConfig";
import { AuthSessionStatus } from "./AuthSessionControl";
import { beginOidcAuthorization, completeOidcCallback, PKCE_TRANSACTION_STORAGE_KEY } from "./oidcSession";
import { loadDisplayPreferences, saveDisplayPreferences, syncBrowserThemeColor } from "./displayPreferences";
import { quoteMessagesFor } from "./quoteMessages";
import { assetScaleMessagesFor, parseAssetScale } from "./assetScaleMessages";
import { revenueStreamingMessagesFor } from "./revenueStreamingMessages";
import { shellMessagesFor } from "./shellMessages";
import { shellOperationalMessagesFor } from "./shellOperationalMessages";
import { operatorErrorMessage, operatorFacingError } from "./operatorError";
import {
  Calculation,
  parseCalculationResponse,
  parseReplayVerificationResponse,
  ReplayVerification,
} from "./taxResponses";
import TaxTreatmentOutcome from "./TaxTreatmentOutcome";

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
const EvidenceAccessPanel = lazy(() => import("./EvidenceAccessPanel"));

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
  const [atomicDecimals, setAtomicDecimals] = useState("");
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
  const [authorizationRefreshNonce, setAuthorizationRefreshNonce] = useState(0);
  const [authorizationErrorKey, setAuthorizationErrorKey] = useState<"unavailable" | "expired" | "missingExpiry" | null>(null);
  const [idempotencyKey, setIdempotencyKey] = useState<string>(() => createClientUuid());
  const [calculation, setCalculation] = useState<Calculation | null>(null);
  const [documentCalculationId, setDocumentCalculationId] = useState("");
  const [replayVerification, setReplayVerification] = useState<ReplayVerification | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [isReplaying, setIsReplaying] = useState(false);
  const [displayPreferences, setDisplayPreferences] = useState(loadDisplayPreferences);
  const { locale, timeZone, theme: themePreference } = displayPreferences;
  const revenueMessages = revenueStreamingMessagesFor(locale);
  const shellMessages = shellMessagesFor(locale);
  const operationalMessages = shellOperationalMessagesFor(locale);
  const authorizationError = authorizationErrorKey ? operationalMessages.authorization[authorizationErrorKey] : null;

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
      setAuthSessionError(operatorErrorMessage(cause, "Sign-in could not be started."));
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

  useEffect(() => syncBrowserThemeColor(themePreference), [themePreference]);

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
          setAuthSessionError(operatorErrorMessage(cause, "Sign-in could not be completed."));
        });
    } catch (cause) {
      setAccessToken("");
      setAuthSessionExpiresAt(null);
      setAuthSessionStatus("error");
      setAuthSessionError(operatorErrorMessage(cause, "Sign-in could not be completed."));
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
      setAuthorizationErrorKey(null);
      return () => controller.abort();
    }
    setAuthorizationSnapshot(authorizationSnapshotForToken(token));
    setAuthorizationErrorKey(null);
    const timer = globalThis.setTimeout(() => {
      fetchWithTimeout(`${API_BASE}/v1/authorization/capabilities`, {
        headers: { authorization: `Bearer ${token}` },
        signal: controller.signal,
      }, REQUEST_TIMEOUT_MS.default)
        .then(async (response) => {
          const payload: unknown = await response.json().catch(() => null);
          if (!response.ok) throw operatorResponseError(response, payload, "Access could not be checked.");
          return parseAuthorizationCapabilities(payload);
        })
        .then((snapshot) => {
          if (active) setAuthorizationSnapshot(snapshot);
        })
        .catch((cause: unknown) => {
          if (active && !(cause instanceof DOMException && cause.name === "AbortError")) {
            setAuthorizationErrorKey(cause instanceof AuthorizationCapabilitiesError && cause.code === "expired" ? "expired" : "unavailable");
          }
        });
    }, 250);
    return () => {
      active = false;
      globalThis.clearTimeout(timer);
      controller.abort();
    };
  }, [accessToken, authorizationRefreshNonce]);

  useEffect(() => {
    if (!accessToken.trim() || authorizationSnapshot.state !== "server-allowed") return;
    const delayMs = authorizationSnapshotRefreshDelayMs(authorizationSnapshot);
    if (delayMs === null) {
      return;
    }
    if (delayMs === 0 && !Number.isSafeInteger(authorizationSnapshot.expiresAtUnix)) {
      setAuthorizationSnapshot(authorizationSnapshotForToken(accessToken));
      setAuthorizationErrorKey("missingExpiry");
      return;
    }
    const timer = globalThis.setTimeout(() => {
      setAuthorizationSnapshot(authorizationSnapshotForToken(accessToken));
      setAuthorizationErrorKey("expired");
      setAuthorizationRefreshNonce((current) => current + 1);
    }, delayMs);
    return () => globalThis.clearTimeout(timer);
  }, [accessToken, authorizationSnapshot]);

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
      const atomicDecimalsValue = parseAssetScale(atomicDecimals, assetScaleMessages.invalid);
      const response = await fetchWithTimeout(`${API_BASE}/v1/tax/calculate`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          facts: { gross_atomic: gross.trim(), net_atomic: net.trim(), atomic_decimals: atomicDecimalsValue, currency: currency.trim().toUpperCase(), jurisdiction },
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
      if (!response.ok) throw operatorResponseError(response, payload, "Preview calculation failed");
      setCalculation(parseCalculationResponse(payload));
      setReplayVerification(null);
    } catch (cause) {
      setError(operatorErrorMessage(cause, "Preview calculation failed"));
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
        throw operatorFacingError("At least one provider-signed jurisdiction evidence envelope is required.");
      }
    } catch (cause) {
      setError(operatorErrorMessage(cause, "Jurisdiction evidence must be valid JSON."));
      return;
    }
    setIsSubmitting(true);
    try {
      const atomicDecimalsValue = parseAssetScale(atomicDecimals, assetScaleMessages.invalid);
      const response = await fetchWithTimeout(`${API_BASE}/v1/tax/quote`, {
        method: "POST",
        headers: { "content-type": "application/json", authorization: `Bearer ${accessToken.trim()}` },
        body: JSON.stringify({
          idempotency_key: idempotencyKey.trim(),
          rule_set_id: ruleSetId.trim(),
          rule_version: Number(ruleVersion),
          jurisdiction,
          regime: regime.trim(),
          facts: { gross_atomic: gross.trim(), net_atomic: net.trim(), atomic_decimals: atomicDecimalsValue, currency: currency.trim().toUpperCase(), jurisdiction },
          jurisdiction_evidence: jurisdictionEvidence
        })
      });
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw operatorResponseError(response, payload, "Quote request failed");
      setCalculation(parseCalculationResponse(payload));
      setReplayVerification(null);
    } catch (cause) {
      setError(operatorErrorMessage(cause, "Quote request failed"));
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
      if (!response.ok) throw operatorResponseError(response, payload, "Replay verification failed");
      setReplayVerification(parseReplayVerificationResponse(payload));
    } catch (cause) {
      setReplayVerification(null);
      setError(operatorErrorMessage(cause, "Replay verification failed"));
    } finally {
      setIsReplaying(false);
    }
  }

  const submit = workspace === "quote" ? createQuote : calculatePreview;
  const formatAtomic = (value: string, currency: string, atomicDecimals: number) => formatAtomicValue(value, currency, locale, atomicDecimals);
  const quoteMessages = quoteMessagesFor(locale);
  const assetScaleMessages = assetScaleMessagesFor(locale);
  const modeMessages = workspace === "quote" ? quoteMessages.quoteMode : quoteMessages.previewMode;

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
          <WorkspaceErrorBoundary key={workspace} workspace={workspace} locale={locale}>
          {workspace === "access-control" ? <AccessControlPanel accessToken={accessToken} authorizationSnapshot={authorizationSnapshot} authorizationError={authorizationError} locale={locale} /> : workspace === "evidence-access" ? <Suspense fallback={<section className="evidence-layout" aria-live="polite"><article className="panel empty-state"><strong>{operationalMessages.loading.title(shellMessages.navigation["evidence-access"].label)}</strong><p>{operationalMessages.loading.copy}</p></article></section>}><EvidenceAccessPanel
          apiBase={API_BASE}
          serviceOnline={serviceState === "online"}
          accessToken={accessToken}
          locale={locale}
          timeZone={effectiveTimeZone}
          /></Suspense> : workspace === "revenue-streaming" ? <Suspense fallback={<section className="vault-layout" aria-live="polite"><article className="panel empty-state"><strong>{revenueMessages.shell.title}</strong><p>{revenueMessages.shell.copy}</p></article></section>}><RevenueStreamingPanel accessToken={accessToken} authorizationSnapshot={authorizationSnapshot} authorizationError={authorizationError} locale={locale} /></Suspense> : workspace === "document-quote" ? <Suspense fallback={<section className="workspace-grid" aria-live="polite"><article className="panel empty-state"><strong>{operationalMessages.loading.title(shellMessages.navigation["document-quote"].label)}</strong><p>{operationalMessages.loading.copy}</p></article></section>}><DocumentQuotePanel
          apiBase={API_BASE}
          serviceOnline={serviceState === "online"}
          accessToken={accessToken}
          formatAtomic={formatAtomic}
          locale={locale}
          timeZone={effectiveTimeZone}
          onOpenCheckout={(calculationId) => { setDocumentCalculationId(calculationId); navigateToWorkspace("document-checkout", true); }}
        /></Suspense> : workspace === "document-checkout" ? <Suspense fallback={<section className="checkout-layout" aria-live="polite"><article className="panel empty-state"><strong>{operationalMessages.loading.title(shellMessages.navigation["document-checkout"].label)}</strong><p>{operationalMessages.loading.copy}</p></article></section>}><DocumentCheckoutPanel
          apiBase={API_BASE}
          serviceOnline={serviceState === "online"}
          accessToken={accessToken}
          initialCalculationId={documentCalculationId}
          locale={locale}
          timeZone={effectiveTimeZone}
          /></Suspense> : workspace === "vault-reconciliation" ? <Suspense fallback={<section className="vault-layout" aria-live="polite"><article className="panel empty-state"><strong>{operationalMessages.loading.title(shellMessages.navigation["vault-reconciliation"].label)}</strong><p>{operationalMessages.loading.copy}</p></article></section>}><VaultReconciliationPanel
          apiBase={API_BASE}
          serviceOnline={serviceState === "online"}
          accessToken={accessToken}
          locale={locale}
          timeZone={effectiveTimeZone}
        /></Suspense> : workspace === "governance" ? <Suspense fallback={<section className="governance-layout" aria-live="polite"><article className="panel empty-state"><strong>{operationalMessages.loading.title(shellMessages.navigation.governance.label)}</strong><p>{operationalMessages.loading.copy}</p></article></section>}><GovernancePanel
          apiBase={API_BASE}
          serviceOnline={serviceState === "online"}
          accessToken={accessToken}
          locale={locale}
          timeZone={effectiveTimeZone}
        /></Suspense> : workspace === "replay-ops" ? <Suspense fallback={<section className="replay-layout" aria-live="polite"><article className="panel empty-state"><strong>{operationalMessages.loading.title(shellMessages.navigation["replay-ops"].label)}</strong><p>{operationalMessages.loading.copy}</p></article></section>}><ReplayOpsPanel
          apiBase={API_BASE}
          serviceOnline={serviceState === "online"}
          accessToken={accessToken}
          locale={locale}
          timeZone={effectiveTimeZone}
          /></Suspense> : workspace === "vault-release" ? <Suspense fallback={<section className="vault-layout" aria-live="polite"><article className="panel empty-state"><strong>{operationalMessages.loading.title(shellMessages.navigation["vault-release"].label)}</strong><p>{operationalMessages.loading.copy}</p></article></section>}><VaultReleasePanel
          apiBase={API_BASE}
          serviceOnline={serviceState === "online"}
          accessToken={accessToken}
          locale={locale}
          timeZone={effectiveTimeZone}
          /></Suspense> : workspace === "adjustments" ? <Suspense fallback={<section className="adjustment-layout" aria-live="polite"><article className="panel empty-state"><strong>{operationalMessages.loading.title(shellMessages.navigation.adjustments.label)}</strong><p>{operationalMessages.loading.copy}</p></article></section>}><SettlementAdjustmentPanel
          apiBase={API_BASE}
          serviceOnline={serviceState === "online"}
          accessToken={accessToken}
          locale={locale}
          /></Suspense> : workspace === "refund-claims" ? <Suspense fallback={<section className="adjustment-layout" aria-live="polite"><article className="panel empty-state"><strong>{operationalMessages.loading.title(shellMessages.navigation["refund-claims"].label)}</strong><p>{operationalMessages.loading.copy}</p></article></section>}><RefundClaimPanel
          apiBase={API_BASE}
          serviceOnline={serviceState === "online"}
          accessToken={accessToken}
          locale={locale}
          /></Suspense> : workspace === "refund-cancellations" ? <Suspense fallback={<section className="adjustment-layout" aria-live="polite"><article className="panel empty-state"><strong>{operationalMessages.loading.title(shellMessages.navigation["refund-cancellations"].label)}</strong><p>{operationalMessages.loading.copy}</p></article></section>}><RefundCancellationPanel
          apiBase={API_BASE}
          serviceOnline={serviceState === "online"}
          accessToken={accessToken}
          locale={locale}
          /></Suspense> : workspace === "refund-executions" ? <Suspense fallback={<section className="adjustment-layout" aria-live="polite"><article className="panel empty-state"><strong>{operationalMessages.loading.title(shellMessages.navigation["refund-executions"].label)}</strong><p>{operationalMessages.loading.copy}</p></article></section>}><RefundExecutePanel
          apiBase={API_BASE}
          serviceOnline={serviceState === "online"}
          accessToken={accessToken}
          locale={locale}
          /></Suspense> : workspace === "checkout" ? <Suspense fallback={<section className="checkout-layout" aria-live="polite"><article className="panel empty-state"><strong>{operationalMessages.loading.title(shellMessages.navigation.checkout.label)}</strong><p>{operationalMessages.loading.copy}</p></article></section>}><CheckoutPanel
          apiBase={API_BASE}
          serviceOnline={serviceState === "online"}
          accessToken={accessToken}
          calculation={calculation}
          formatAtomic={formatAtomic}
          locale={locale}
          timeZone={effectiveTimeZone}
        /></Suspense> : <section className="workspace-grid">
          <article className="panel calculator-panel">
            <div className="panel-heading"><div><p className="eyebrow">{modeMessages.eyebrow}</p><h2>{modeMessages.title}</h2></div><span className="status-pill">{modeMessages.status}</span></div>
            <p className="panel-copy">{modeMessages.description}</p>
            {workspace === "quote" && BROWSER_AUTH_CONFIG.mode === "unconfigured" && <p className="inline-warning" role="alert">{quoteMessages.readOnlyWarning}</p>}
            {workspace === "quote" && <details className="route-evidence full-field"><summary>{quoteMessages.location.summary}</summary><p className="field-help">{quoteMessages.location.description}</p><label htmlFor="aggregate-jurisdiction-evidence"><span>{quoteMessages.location.label}</span><textarea id="aggregate-jurisdiction-evidence" value={jurisdictionEvidenceJson} onChange={(event) => setJurisdictionEvidenceJson(event.target.value)} placeholder={quoteMessages.location.placeholder} autoComplete="off" spellCheck={false} rows={6} aria-describedby="jurisdiction-evidence-help" /><small id="jurisdiction-evidence-help">{quoteMessages.location.help}</small></label></details>}
            <form onSubmit={submit} className="form-grid">
              <label><span>{quoteMessages.fields.grossLabel}</span><input inputMode="numeric" min="0" value={gross} onChange={(event) => setGross(event.target.value)} placeholder={quoteMessages.fields.grossPlaceholder} aria-describedby="amount-help" required /></label>
              <label><span>{quoteMessages.fields.netLabel}</span><input inputMode="numeric" min="0" value={net} onChange={(event) => setNet(event.target.value)} placeholder={quoteMessages.fields.netPlaceholder} aria-describedby="amount-help" required /></label>
              <label><span>{quoteMessages.fields.currencyLabel}</span><input value={currency} onChange={(event) => setCurrency(event.target.value.toUpperCase())} placeholder={quoteMessages.fields.currencyPlaceholder} maxLength={12} required /></label>
              <label><span>{assetScaleMessages.label}</span><input inputMode="numeric" min="0" max="9" value={atomicDecimals} onChange={(event) => setAtomicDecimals(event.target.value)} aria-describedby="amount-help asset-precision-help" required /><small id="asset-precision-help" className="field-help">{assetScaleMessages.help}</small></label>
              {workspace === "preview" ? <label><span>{quoteMessages.fields.rateLabel}</span><input inputMode="decimal" min="0" step="0.01" value={rate} onChange={(event) => setRate(event.target.value)} placeholder={quoteMessages.fields.ratePlaceholder} required /></label> : <label><span>{quoteMessages.fields.approvedRuleLabel}</span><input value={ruleSetId} onChange={(event) => setRuleSetId(event.target.value)} placeholder={quoteMessages.fields.approvedRulePlaceholder} required /></label>}
              {workspace === "preview" ? <label><span>{quoteMessages.fields.jurisdictionLabel}</span><input value={jurisdiction} onChange={(event) => setJurisdiction(event.target.value.toUpperCase())} placeholder={quoteMessages.fields.jurisdictionPlaceholder} required /></label> : <label><span>{quoteMessages.fields.ruleVersionLabel}</span><input inputMode="numeric" min="1" value={ruleVersion} onChange={(event) => setRuleVersion(event.target.value)} placeholder={quoteMessages.fields.ruleVersionPlaceholder} required /> </label>}
              {workspace === "quote" && <><label><span>{quoteMessages.fields.jurisdictionLabel}</span><input value={jurisdiction} onChange={(event) => setJurisdiction(event.target.value.toUpperCase())} placeholder={quoteMessages.fields.jurisdictionPlaceholder} required /></label><label><span>{quoteMessages.fields.categoryLabel}</span><input value={regime} onChange={(event) => setRegime(event.target.value)} placeholder={quoteMessages.fields.categoryPlaceholder} required /></label><div className="route-evidence full-field"><strong>{quoteMessages.retry.title}</strong><small>{quoteMessages.retry.description}</small></div></>}
              <small id="amount-help" className="field-help full-field">{quoteMessages.amountHelp}</small>
              <button type="submit" disabled={isSubmitting || serviceState !== "online" || (workspace === "quote" && !accessToken.trim())}>{isSubmitting ? quoteMessages.preparing : modeMessages.submit}</button>
            </form>
            {serviceState === "offline" && <p className="inline-warning" role="alert">{quoteMessages.offlineWarning}</p>}
            {error && <p className="inline-error" role="alert">{error}</p>}
          </article>

          <article className="panel result-panel" aria-live="polite">
            <div className="panel-heading"><div><p className="eyebrow">{quoteMessages.result.eyebrow}</p><h2>{quoteMessages.result.title}</h2></div>{calculation && <span className={`status-pill ${calculation.status === "attested" ? "status-pill-success" : "status-pill-warning"}`}>{calculation.status === "attested" ? quoteMessages.result.verified : calculation.status === "calculated_unattested" ? quoteMessages.result.needsVerification : quoteMessages.result.scenarioOnly}</span>}</div>
            {!calculation ? <div className="empty-state"><span className="empty-icon">∑</span><strong>{quoteMessages.result.emptyTitle}</strong><p>{quoteMessages.result.emptyCopy}</p></div> : <div className="result-content">
              <details className="tax-explanation"><summary>{quoteMessages.result.whyTax}</summary><p>{quoteMessages.result.explanation}</p><dl className="summary-list"><div><dt>{quoteMessages.result.approvedRule}</dt><dd>{calculation.result.rule_set_id} v{calculation.result.rule_version}</dd></div>{calculation.result.fx_conversion && <div><dt>{quoteMessages.result.currencyConversion}</dt><dd>{calculation.result.fx_conversion.source_currency} → {calculation.result.fx_conversion.target_currency} · {calculation.result.fx_conversion.provider} · {calculation.result.fx_conversion.quote_id}</dd></div>}</dl><div className="trace-list">{calculation.result.components.map((component) => <div className="trace-row" key={component.id}><div><strong>{component.label}</strong><small>{component.explanation}</small></div><strong>{formatAtomic(component.tax_atomic, calculation.result.currency, calculation.result.atomic_decimals)}</strong></div>)}</div><details className="technical-details"><summary>{quoteMessages.result.recordFingerprints}</summary><dl className="summary-list"><div><dt>{quoteMessages.result.ruleFingerprint}</dt><dd><code>{shortDigest(calculation.result.rule_digest)}</code></dd></div><div><dt>{quoteMessages.result.inputFingerprint}</dt><dd><code>{shortDigest(calculation.result.facts_digest)}</code></dd></div></dl></details></details>
              <div className="total-card"><span>{quoteMessages.result.totalTax}</span><strong>{formatAtomic(calculation.result.total_tax_atomic, calculation.result.currency, calculation.result.atomic_decimals)}</strong><small>{quoteMessages.result.reference} {calculation.calculation_id}</small></div>
              <TaxTreatmentOutcome outcome={calculation.result.treatment_outcome} currency={calculation.result.currency} atomicDecimals={calculation.result.atomic_decimals} locale={locale} formatAtomic={formatAtomic} />
              <dl className="summary-list"><div><dt>{quoteMessages.result.jurisdiction}</dt><dd>{calculation.result.jurisdiction}</dd></div><div><dt>{quoteMessages.result.amountAfterTax}</dt><dd>{formatAtomic(calculation.result.total_net_atomic, calculation.result.currency, calculation.result.atomic_decimals)}</dd></div><div><dt>{quoteMessages.result.resultStatus}</dt><dd>{calculation.status === "attested" ? quoteMessages.result.verifiedStatus : calculation.status === "calculated_unattested" ? quoteMessages.result.calculatedStatus : quoteMessages.result.scenarioStatus}</dd></div></dl>
              <details className="technical-details"><summary>{quoteMessages.result.technicalDetails}</summary><dl className="summary-list"><div><dt>{quoteMessages.result.calculationReference}</dt><dd><code>{calculation.calculation_id}</code></dd></div><div><dt>{quoteMessages.result.engineVersion}</dt><dd>{calculation.result.engine_version || quoteMessages.result.legacyEvidence}</dd></div><div><dt>{quoteMessages.result.policyState}</dt><dd>{calculation.status === "attested" ? quoteMessages.result.publishedAttestation : calculation.status === "calculated_unattested" ? quoteMessages.result.publishedNotAttested : quoteMessages.result.callerPreview}</dd></div><div><dt>{assetScaleMessages.label}</dt><dd>{calculation.result.atomic_decimals}</dd></div></dl></details>
              {["calculated_unattested", "attested"].includes(calculation.status) && <div className="evidence-actions"><button type="button" onClick={verifyPersistedReplay} disabled={isReplaying || !accessToken.trim()}>{isReplaying ? quoteMessages.result.checkingRecord : replayVerification ? quoteMessages.result.recordChecked : quoteMessages.result.checkRecord}</button><p className="evidence-note">{quoteMessages.result.evidenceNote}</p>{replayVerification && <dl className="summary-list evidence-summary"><div><dt>{quoteMessages.result.checkStatus}</dt><dd>{replayVerification.status}</dd></div><div><dt>{quoteMessages.result.factsDigest}</dt><dd><code>{shortDigest(replayVerification.facts_digest)}</code></dd></div><div><dt>{quoteMessages.result.ruleDigest}</dt><dd><code>{shortDigest(replayVerification.rule_digest)}</code></dd></div></dl>}</div>}
            </div>}
          </article>
          </section>}
          </WorkspaceErrorBoundary>
        </div>
    </ProductShell>
  );
}
