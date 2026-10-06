import { FormEvent, useEffect, useState } from "react";
import { PublicKey } from "@solana/web3.js";
import { fetchWithTimeout, operatorResponseError, REQUEST_TIMEOUT_MS } from "./api";
import { DEFAULT_LOCALE, DEFAULT_TIME_ZONE, formatEpoch } from "./formatters";
import {
  CompiledSettlementArtifact,
  SettlementRoute,
  SettlementSplitPlan,
  SettlementSubmitResponse,
  parseCompiledSettlementArtifact,
  parseSettlementRoutes,
  parseSettlementStatusResponse,
  parseSettlementSplitPlans,
  signAndEncodeSettlement,
  settlementQuoteExpired,
  settlementIntentAllowsRetry,
} from "./checkout";
import WalletPicker from "./WalletPicker";
import { createClientNonce } from "./clientIdentity";
import { checkoutMessagesFor } from "./checkoutMessages";
import { assetScaleMessagesFor } from "./assetScaleMessages";
import { operatorErrorMessage } from "./operatorError";
import TaxTreatmentOutcome from "./TaxTreatmentOutcome";
import type { TreatmentOutcome } from "./taxResponses";

type Calculation = {
  calculation_id: string;
  status: string;
  expires_at_unix?: number;
  result: {
    rule_set_id: string;
    rule_version: number;
    currency: string;
    jurisdiction: string;
    rule_digest: number[];
    gross_atomic: string;
    total_tax_atomic: string;
    total_net_atomic: string;
    treatment_outcome?: TreatmentOutcome;
  };
};

type CheckoutPanelProps = {
  apiBase: string;
  serviceOnline: boolean;
  accessToken: string;
  calculation: Calculation | null;
  formatAtomic: (value: string, currency: string, atomicDecimals: number) => string;
  locale?: string;
  timeZone?: string;
};

type FlowState = "idle" | "connecting" | "compiling" | "ready" | "signing" | "submitting" | "complete";

export default function CheckoutPanel({
  apiBase,
  serviceOnline,
  accessToken,
  calculation,
  formatAtomic,
  locale = DEFAULT_LOCALE,
  timeZone = DEFAULT_TIME_ZONE,
}: CheckoutPanelProps) {
  const messages = checkoutMessagesFor(locale);
  const assetScaleMessages = assetScaleMessagesFor(locale);
  const [walletAddress, setWalletAddress] = useState<PublicKey | null>(null);
  const [routes, setRoutes] = useState<SettlementRoute[]>([]);
  const [routeId, setRouteId] = useState("");
  const [routeState, setRouteState] = useState<"idle" | "loading" | "ready" | "error">("idle");
  const [routeError, setRouteError] = useState<string | null>(null);
  const [nonce, setNonce] = useState(() => createClientNonce());
  const [artifact, setArtifact] = useState<CompiledSettlementArtifact | null>(null);
  const [settlement, setSettlement] = useState<SettlementSubmitResponse | null>(null);
  const [flow, setFlow] = useState<FlowState>("idle");
  const [error, setError] = useState<string | null>(null);
  const [settlementMode, setSettlementMode] = useState<"single" | "split">("single");
  const [splitPlans, setSplitPlans] = useState<SettlementSplitPlan[]>([]);
  const [splitPlanId, setSplitPlanId] = useState("");
  const [splitPlanState, setSplitPlanState] = useState<"idle" | "loading" | "ready" | "error">("idle");
  const [splitPlanError, setSplitPlanError] = useState<string | null>(null);
  const [splitPlanReload, setSplitPlanReload] = useState(0);
  const [nowUnix, setNowUnix] = useState(() => Math.floor(Date.now() / 1000));

  useEffect(() => {
    if (calculation?.expires_at_unix === undefined) return;
    const refresh = () => setNowUnix(Math.floor(Date.now() / 1000));
    refresh();
    const timer = window.setInterval(refresh, 1_000);
    return () => window.clearInterval(timer);
  }, [calculation?.expires_at_unix]);

  useEffect(() => {
    if (!calculation) {
      setArtifact(null);
      setSettlement(null);
    }
  }, [calculation?.calculation_id]);

  useEffect(() => {
    setArtifact(null);
    setSettlement(null);
    setFlow("idle");
    setNonce(createClientNonce());
  }, [routeId, settlementMode, splitPlanId]);

  useEffect(() => {
    if (!serviceOnline || !accessToken.trim()) {
      setRoutes([]);
      setRouteId("");
      setRouteState("idle");
      setRouteError(null);
      return;
    }
    const controller = new AbortController();
    const timer = window.setTimeout(async () => {
      setRouteState("loading");
      setRouteError(null);
      try {
        const response = await fetchWithTimeout(`${apiBase}/v1/settlement/routes`, {
          headers: { authorization: `Bearer ${accessToken.trim()}` },
          signal: controller.signal,
        }, REQUEST_TIMEOUT_MS.default);
        const payload: unknown = await response.json().catch(() => null);
        if (!response.ok) throw operatorResponseError(response, payload, messages.errors.routeRequestFailed(response.status));
        const loaded = parseSettlementRoutes(payload);
        setRoutes(loaded);
        setRouteId((current) => loaded.some((route) => route.id === current) ? current : loaded[0]?.id ?? "");
        setRouteState("ready");
      } catch (cause) {
        if (cause instanceof DOMException && cause.name === "AbortError") return;
        setRoutes([]);
        setRouteId("");
        setRouteState("error");
        setRouteError(operatorErrorMessage(cause, messages.route.unavailable));
      }
    }, 300);
    return () => {
      window.clearTimeout(timer);
      controller.abort();
    };
  }, [accessToken, apiBase, serviceOnline, locale]);

  useEffect(() => {
    if (!serviceOnline || !accessToken.trim()) {
      setSplitPlans([]);
      setSplitPlanId("");
      setSplitPlanState("idle");
      setSplitPlanError(null);
      return;
    }
    const controller = new AbortController();
    const timer = window.setTimeout(async () => {
      setSplitPlanState("loading");
      setSplitPlanError(null);
      try {
        const response = await fetchWithTimeout(`${apiBase}/v1/settlement/split-plans`, {
          headers: { authorization: `Bearer ${accessToken.trim()}` },
          signal: controller.signal,
        }, REQUEST_TIMEOUT_MS.default);
        const payload: unknown = await response.json().catch(() => null);
        if (!response.ok) throw operatorResponseError(response, payload, messages.errors.splitPlanRequestFailed(response.status));
        const loaded = parseSettlementSplitPlans(payload);
        setSplitPlans(loaded);
        setSplitPlanId((current) => loaded.some((plan) => plan.id === current) ? current : loaded[0]?.id ?? "");
        setSplitPlanState("ready");
      } catch (cause) {
        if (cause instanceof DOMException && cause.name === "AbortError") return;
        setSplitPlans([]);
        setSplitPlanId("");
        setSplitPlanState("error");
        setSplitPlanError(operatorErrorMessage(cause, messages.split.unavailable));
      }
    }, 300);
    return () => {
      window.clearTimeout(timer);
      controller.abort();
    };
  }, [accessToken, apiBase, serviceOnline, splitPlanReload, locale]);

  async function compile(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);
    if (!serviceOnline) return setError(messages.errors.serviceUnavailable);
    if (!accessToken.trim()) return setError(messages.errors.authorizationRequired);
    if (!calculation || !["calculated_unattested", "attested"].includes(calculation.status)) {
      return setError(messages.errors.quoteRequired);
    }
    if (settlementQuoteExpired(calculation.expires_at_unix, nowUnix)) {
      return setError(messages.errors.quoteExpired);
    }
    if (!walletAddress) return setError(messages.errors.walletRequired);
    if (!routeId) {
      return setError(messages.errors.routeRequired);
    }
    if (settlementMode === "split" && (!splitPlanId || !selectedPlan)) {
      return setError(messages.errors.splitPlanRequired);
    }
    if (settlementMode === "split" && selectedPlan && selectedRoute
        && (selectedPlan.merchant_account !== selectedRoute.merchant_account || selectedPlan.mint !== selectedRoute.mint)) {
      return setError(messages.errors.splitPlanMismatch);
    }
    let canonicalNonce: string;
    try {
      const numericValue = BigInt(nonce);
      if (numericValue <= 0n || numericValue > 18_446_744_073_709_551_615n) {
        throw new Error(messages.errors.invalidFields);
      }
      canonicalNonce = numericValue.toString();
    } catch (cause) {
      return setError(operatorErrorMessage(cause, messages.errors.invalidFields));
    }

    setFlow("compiling");
    try {
      const response = await fetchWithTimeout(`${apiBase}/v1/settlement/${settlementMode === "split" ? "split/" : ""}compile-from-quote`, {
        method: "POST",
        headers: { "content-type": "application/json", authorization: `Bearer ${accessToken.trim()}` },
        body: JSON.stringify({
          calculation_id: calculation.calculation_id,
          route_id: routeId,
          payer: walletAddress.toBase58(),
          nonce: canonicalNonce,
          ...(settlementMode === "split" ? { split_plan_id: splitPlanId } : {}),
        }),
      }, REQUEST_TIMEOUT_MS.default);
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw operatorResponseError(response, payload, messages.errors.compileRequestFailed(response.status));
      const compiled: CompiledSettlementArtifact = parseCompiledSettlementArtifact(payload);
      if (!compiled.intent_id) throw new Error(messages.errors.intentRequired);
      setArtifact(compiled);
      setSettlement(null);
      setFlow("ready");
    } catch (cause) {
      setFlow("idle");
      setError(operatorErrorMessage(cause, messages.errors.compileFailed));
    }
  }

  async function signAndSubmit() {
    setError(null);
    if (!artifact || !artifact.intent_id || !walletAddress) return setError(messages.errors.intentRequired);
    if (settlementQuoteExpired(calculation?.expires_at_unix, nowUnix)) {
      setFlow("ready");
      return setError(messages.errors.signatureExpired);
    }
    const rpcUrl = import.meta.env.VITE_SOLANA_RPC_URL ?? "";
    let submitAttempted = false;
    setFlow("signing");
    try {
      const signedTransactionBase64 = await signAndEncodeSettlement(artifact, walletAddress, rpcUrl);
      setFlow("submitting");
      submitAttempted = true;
      const response = await fetchWithTimeout(`${apiBase}/v1/settlement/submit`, {
        method: "POST",
        headers: { "content-type": "application/json", authorization: `Bearer ${accessToken.trim()}` },
        body: JSON.stringify({ intent_id: artifact.intent_id, signed_transaction_base64: signedTransactionBase64, instruction: artifact }),
      }, REQUEST_TIMEOUT_MS.settlementSubmit);
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw operatorResponseError(response, payload, messages.errors.submissionRequestFailed(response.status));
      setSettlement(parseSettlementStatusResponse(payload));
      setFlow("complete");
    } catch (cause) {
      if (submitAttempted) {
        try {
          const recovered = await loadIntentStatus(artifact.intent_id);
          setSettlement(recovered);
          setFlow(settlementIntentAllowsRetry(recovered.status) ? "ready" : "complete");
          setError(messages.errors.intentRecovery(recovered.status));
          return;
        } catch {
          try {
            setSettlement(parseSettlementStatusResponse({
              intent_id: artifact.intent_id,
              status: "unknown",
              signature: null,
              provider_index: null,
            }));
          } catch {
            setSettlement(null);
          }
          setFlow("complete");
          setError(messages.errors.submissionUnknown);
          return;
        }
      }
      setFlow("ready");
      setError(operatorErrorMessage(cause, messages.errors.signingFailed));
    }
  }

  async function loadIntentStatus(intentId: string): Promise<SettlementSubmitResponse> {
    const response = await fetchWithTimeout(`${apiBase}/v1/settlement/intents/${intentId}`, {
      headers: { authorization: `Bearer ${accessToken.trim()}` },
    }, REQUEST_TIMEOUT_MS.default);
    const payload: unknown = await response.json().catch(() => null);
    if (!response.ok) throw operatorResponseError(response, payload, messages.errors.statusRequestFailed(response.status));
    return parseSettlementStatusResponse(payload);
  }

  async function refreshStatus() {
    if (!settlement?.intent_id) return;
    setError(null);
    try {
      const refreshed = await loadIntentStatus(settlement.intent_id);
      setSettlement(refreshed);
      setFlow(settlementIntentAllowsRetry(refreshed.status) ? "ready" : "complete");
    } catch (cause) {
      setError(operatorErrorMessage(cause, messages.errors.statusRefreshFailed));
    }
  }

  async function reconcile() {
    if (!settlement?.intent_id) return;
    setError(null);
    try {
      const response = await fetchWithTimeout(`${apiBase}/v1/settlement/intents/${settlement.intent_id}/reconcile`, {
        method: "POST",
        headers: { authorization: `Bearer ${accessToken.trim()}` },
      }, REQUEST_TIMEOUT_MS.default);
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw operatorResponseError(response, payload, messages.errors.reconciliationRequestFailed(response.status));
      const reconciled = parseSettlementStatusResponse(payload);
      setSettlement(reconciled);
      setFlow(settlementIntentAllowsRetry(reconciled.status) ? "ready" : "complete");
    } catch (cause) {
      setError(operatorErrorMessage(cause, messages.errors.reconciliationFailed));
    }
  }

  const isBusy = flow === "connecting" || flow === "compiling" || flow === "signing" || flow === "submitting";
  const currency = calculation?.result.currency ?? "USD";
  const selectedRoute = routes.find((route) => route.id === routeId) ?? null;
  const selectedPlan = splitPlans.find((plan) => plan.id === splitPlanId) ?? null;
  const splitPlanCompatible = Boolean(selectedPlan && selectedRoute
    && selectedPlan.merchant_account === selectedRoute.merchant_account
    && selectedPlan.mint === selectedRoute.mint);
  const splitPlanDigest = selectedPlan?.plan_digest.map((byte) => byte.toString(16).padStart(2, "0")).join("") ?? "";
  const quoteExpired = settlementQuoteExpired(calculation?.expires_at_unix, nowUnix);
  return (
    <section className="checkout-layout" aria-labelledby="checkout-title">
      <article className="panel">
        <div className="panel-heading"><div><p className="eyebrow">{messages.review.eyebrow}</p><h2 id="checkout-title">{messages.review.title}</h2></div><span className="status-pill">{messages.review.status}</span></div>
        <p className="panel-copy">{messages.review.description}</p>
        <div className="checkout-step"><span className="step-number">1</span><div><strong>{walletAddress ? messages.review.walletConnected : messages.review.walletStep}</strong><p>{walletAddress ? walletAddress.toBase58() : messages.review.walletMissing}</p></div><WalletPicker locale={locale} disabled={isBusy} connectedAddress={walletAddress?.toBase58()} onConnected={setWalletAddress} /></div>
        <form className="form-grid checkout-form" onSubmit={compile}>
          <label className="full-field"><span>{messages.route.paymentRoute}</span><select value={routeId} onChange={(event) => setRouteId(event.target.value)} disabled={routeState !== "ready" || isBusy} required><option value="">{routeState === "loading" ? messages.route.loading : routeState === "error" ? messages.route.unavailable : routes.length ? messages.route.select : messages.route.none}</option>{routes.map((route) => <option value={route.id} key={route.id}>{route.route_key} · {messages.route.version(route.version)}</option>)}</select></label>
          {routeError && <p className="inline-error full-field" role="alert">{routeError}</p>}
          {selectedRoute && <div className="route-evidence full-field" aria-live="polite"><strong>{messages.route.checkedTitle}</strong><small>{messages.route.checkedCopy}</small><details className="technical-details" aria-label={messages.route.technicalLabel}><summary>{messages.route.technicalDetails}</summary><small>Merchant account {selectedRoute.merchant_account} · asset mint {selectedRoute.mint} · {assetScaleMessages.decimalPlaces(selectedRoute.mint_decimals)} · tax account {selectedRoute.tax_vault} · destination account {selectedRoute.net_destination} · token program {selectedRoute.token_program}</small><small>{messages.route.payerSource}</small></details></div>}
          <label className="full-field"><span>{messages.split.sharing}</span><select value={settlementMode} onChange={(event) => setSettlementMode(event.target.value as "single" | "split")} disabled={isBusy}>
            <option value="single">{messages.split.oneDestination}</option>
            <option value="split">{messages.split.recipientPlan}</option>
          </select></label>
          {settlementMode === "split" && <>
            <label className="full-field"><span>{messages.split.approvedPlan}</span><select value={splitPlanId} onChange={(event) => setSplitPlanId(event.target.value)} disabled={splitPlanState !== "ready" || isBusy} required>
              <option value="">{splitPlanState === "loading" ? messages.split.loading : splitPlanState === "error" ? messages.split.unavailable : splitPlans.length ? messages.split.select : messages.split.none}</option>
              {splitPlans.map((plan) => <option value={plan.id} key={plan.id}>{plan.plan_key} · {messages.split.version(plan.version)} · {messages.split.recipients(plan.recipients.length)}</option>)}
            </select></label>
            {splitPlanError && <div className="inline-error full-field" role="alert">{splitPlanError} <button type="button" className="inline-retry" onClick={() => setSplitPlanReload((value) => value + 1)} disabled={isBusy}>{messages.split.retry}</button></div>}
            {selectedPlan && <div className={`route-evidence full-field${splitPlanCompatible ? "" : " route-evidence-warning"}`} aria-live="polite">
              <strong>{splitPlanCompatible ? messages.split.matches : messages.split.mismatch}</strong>
              <small>{messages.split.version(selectedPlan.version)} · {messages.split.recipients(selectedPlan.recipients.length)}</small>
              <details className="technical-details" aria-label={messages.split.details}><summary>{messages.split.details}</summary>{selectedPlan.recipients.map((recipient, index) => <small key={recipient.token_account}><span aria-hidden="true">{index + 1}.</span> {recipient.token_account} · {recipient.basis_points} {messages.split.basisPoints}</small>)}</details>
              {!splitPlanCompatible && <small>{messages.split.chooseRoute}</small>}
            </div>}
          </>}
          <div className="route-evidence full-field"><strong>{messages.safeguards.title}</strong><small>{messages.safeguards.copy}</small></div>
          <button type="submit" disabled={isBusy || quoteExpired || !serviceOnline || !calculation || !selectedRoute || (settlementMode === "split" && !splitPlanCompatible)}>{flow === "compiling" ? messages.review.preparing : messages.review.prepare}</button>
        </form>
        {error && <p className="inline-error" role="alert">{error}</p>}
      </article>

      <article className="panel checkout-review" aria-live="polite">
        <div className="panel-heading"><div><p className="eyebrow">{messages.review.beforeSignEyebrow}</p><h2>{messages.review.beforeSignTitle}</h2></div>{artifact && <span className="status-pill">{messages.review.readyStatus}</span>}</div>
        {!artifact || !calculation || !selectedRoute ? <>
          <div className="empty-state"><span className="empty-icon">✓</span><strong>{messages.review.emptyTitle}</strong><p>{messages.review.emptyCopy}</p></div>
          {calculation?.expires_at_unix && <p className="inline-warning" role={quoteExpired ? "alert" : undefined}>{messages.review.quoteExpires} {formatEpoch(calculation.expires_at_unix, locale, timeZone)} ({quoteExpired ? messages.review.expired : messages.review.serverEnforced}).</p>}
        </> : <>
          <div className="total-card"><span>{messages.review.total}</span><strong>{formatAtomic(calculation.result.gross_atomic, currency, selectedRoute.mint_decimals)}</strong><small>{messages.review.taxWithheld}: {formatAtomic(calculation.result.total_tax_atomic, currency, selectedRoute.mint_decimals)} · {messages.review.amountSent}: {formatAtomic(calculation.result.total_net_atomic, currency, selectedRoute.mint_decimals)}</small></div>
          <TaxTreatmentOutcome outcome={calculation.result.treatment_outcome} currency={currency} atomicDecimals={selectedRoute.mint_decimals} locale={locale} formatAtomic={formatAtomic} />
          <dl className="summary-list"><div><dt>{messages.route.paymentRoute}</dt><dd>{selectedRoute.route_key} · {messages.route.version(selectedRoute.version)}</dd></div><div><dt>{messages.split.sharing}</dt><dd>{"split_plan" in artifact ? `${messages.split.approvedPlan} · ${messages.split.version(selectedPlan?.version ?? 0)}` : messages.split.oneDestination}</dd></div><div><dt>{messages.review.paymentAsset}</dt><dd>{selectedRoute.mint}</dd></div><div><dt>{messages.review.taxRule}</dt><dd>{calculation.result.rule_set_id} · {messages.route.version(calculation.result.rule_version)}</dd></div><div><dt>{messages.review.taxStatus}</dt><dd>{messages.review.calculatedNotAttested}</dd></div><div><dt>{messages.review.paymentReference}</dt><dd><code>{nonce}</code></dd></div></dl>
          <details className="technical-details" aria-label={messages.technical.details}><summary>{messages.technical.details}</summary><dl className="summary-list"><div><dt>{assetScaleMessages.label}</dt><dd>{assetScaleMessages.decimalPlaces(selectedRoute.mint_decimals)}</dd></div><div><dt>{messages.technical.programId}</dt><dd>{artifact.program_id}</dd></div><div><dt>{messages.technical.policyAuthority}</dt><dd>{artifact.policy_attestation}</dd></div><div><dt>{messages.technical.computePolicy}</dt><dd>{messages.route.version(artifact.compute_budget_policy.policy_version)} · {messages.technical.computeUnits(artifact.compute_budget_policy.compute_unit_limit.toLocaleString(locale))}</dd></div><div><dt>{messages.technical.priorityFee}</dt><dd>{messages.technical.microLamports(artifact.compute_budget_policy.micro_lamports_per_compute_unit.toLocaleString(locale))}</dd></div><div><dt>{messages.technical.accounts}</dt><dd>{messages.technical.exactAccountEntries(artifact.accounts.length)}</dd></div><div><dt>{messages.technical.paymentRequest}</dt><dd>{artifact.intent_id}</dd></div><div><dt>{messages.technical.ruleReference}</dt><dd><code>{calculation.result.rule_digest.map((byte) => byte.toString(16).padStart(2, "0")).join("")}</code></dd></div></dl></details>
          {"split_plan" in artifact && selectedPlan && <div className="split-review" aria-label={messages.split.distributionReview}><strong>{messages.split.sharesBound}</strong><small>{selectedPlan.recipients.length} {messages.split.sharesCopy}</small><details className="technical-details"><summary>{messages.split.recipientDetails}</summary>{selectedPlan.recipients.map((recipient) => <div className="split-review-row" key={recipient.token_account}><span>{recipient.token_account}</span><strong>{recipient.basis_points} {messages.split.basisPoints}</strong></div>)}<small>{messages.split.planReference} {splitPlanDigest}.</small></details></div>}
          {calculation.expires_at_unix && <p className="inline-warning" role={quoteExpired ? "alert" : undefined}>{messages.review.quoteExpires} {formatEpoch(calculation.expires_at_unix, locale, timeZone)} ({quoteExpired ? messages.review.expired : messages.review.serverEnforced}).</p>}
          <p className="inline-warning">{messages.review.walletBoundary}</p>
          <button type="button" onClick={signAndSubmit} disabled={isBusy || quoteExpired || flow !== "ready"}>{flow === "signing" ? messages.review.signing : flow === "submitting" ? messages.review.submitting : messages.review.approve}</button>
          {settlement && <div className="settlement-status"><strong>{messages.review.paymentStatus(settlement.status)}</strong><small>{settlement.signature ? messages.review.networkReference(settlement.signature) : messages.review.noNetworkReference}</small>{settlement.status === "submitting" && <small role="status">{messages.review.submittingNotice}</small>}{settlement.status === "unknown" && <small role="status">{messages.review.unknownNotice}</small>}<div className="status-actions"><button type="button" onClick={refreshStatus} disabled={isBusy}>{messages.review.refreshStatus}</button><button type="button" onClick={reconcile} disabled={isBusy || !settlement.signature}>{messages.review.reconcile}</button></div></div>}
        </>}
      </article>
    </section>
  );
}
