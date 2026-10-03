import { FormEvent, useEffect, useState } from "react";
import { PublicKey } from "@solana/web3.js";
import { fetchWithTimeout, REQUEST_TIMEOUT_MS, responseError } from "./api";
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
  };
};

type CheckoutPanelProps = {
  apiBase: string;
  serviceOnline: boolean;
  accessToken: string;
  calculation: Calculation | null;
  formatAtomic: (value: string, currency: string, atomicDecimals?: number) => string;
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
        if (!response.ok) throw new Error(responseError(response, payload, `Route loading failed (${response.status}).`));
        const loaded = parseSettlementRoutes(payload);
        setRoutes(loaded);
        setRouteId((current) => loaded.some((route) => route.id === current) ? current : loaded[0]?.id ?? "");
        setRouteState("ready");
      } catch (cause) {
        if (cause instanceof DOMException && cause.name === "AbortError") return;
        setRoutes([]);
        setRouteId("");
        setRouteState("error");
        setRouteError(cause instanceof Error ? cause.message : "Configured settlement routes could not be loaded.");
      }
    }, 300);
    return () => {
      window.clearTimeout(timer);
      controller.abort();
    };
  }, [accessToken, apiBase, serviceOnline]);

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
        if (!response.ok) throw new Error(responseError(response, payload, `Split Plan loading failed (${response.status}).`));
        const loaded = parseSettlementSplitPlans(payload);
        setSplitPlans(loaded);
        setSplitPlanId((current) => loaded.some((plan) => plan.id === current) ? current : loaded[0]?.id ?? "");
        setSplitPlanState("ready");
      } catch (cause) {
        if (cause instanceof DOMException && cause.name === "AbortError") return;
        setSplitPlans([]);
        setSplitPlanId("");
        setSplitPlanState("error");
        setSplitPlanError(cause instanceof Error ? cause.message : "Governed Split Plans could not be loaded.");
      }
    }, 300);
    return () => {
      window.clearTimeout(timer);
      controller.abort();
    };
  }, [accessToken, apiBase, serviceOnline, splitPlanReload]);

  async function compile(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);
    if (!serviceOnline) return setError("The API is unavailable; no client-side fallback is used.");
    if (!accessToken.trim()) return setError("An access token with tax:settlement:compile and tax:settlement:submit scopes is required.");
    if (!calculation || !["calculated_unattested", "attested"].includes(calculation.status)) {
      return setError("Create an authenticated published-rule quote before preparing settlement.");
    }
    if (settlementQuoteExpired(calculation.expires_at_unix, nowUnix)) {
      return setError("This quote has expired. Create a new quote before preparing settlement.");
    }
    if (!walletAddress) return setError("Connect the payer wallet before compiling the signed transaction.");
    if (!routeId) {
      return setError("Select an active settlement route.");
    }
    if (settlementMode === "split" && (!splitPlanId || !selectedPlan)) {
      return setError("Select an active governed Split Plan before compiling.");
    }
    if (settlementMode === "split" && selectedPlan && selectedRoute
        && (selectedPlan.merchant_account !== selectedRoute.merchant_account || selectedPlan.mint !== selectedRoute.mint)) {
      return setError("The selected Split Plan is not compatible with the selected settlement route.");
    }
    let canonicalNonce: string;
    try {
      const numericValue = BigInt(nonce);
      if (numericValue <= 0n || numericValue > 18_446_744_073_709_551_615n) {
        throw new Error("The automatically assigned payment reference is invalid.");
      }
      canonicalNonce = numericValue.toString();
    } catch (cause) {
      return setError(cause instanceof Error ? cause.message : "Settlement fields are invalid.");
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
      if (!response.ok) throw new Error(responseError(response, payload, `Settlement compilation failed (${response.status}).`));
      const compiled: CompiledSettlementArtifact = parseCompiledSettlementArtifact(payload);
      if (!compiled.intent_id) throw new Error("The API did not create a durable settlement intent; signing is disabled.");
      setArtifact(compiled);
      setSettlement(null);
      setFlow("ready");
    } catch (cause) {
      setFlow("idle");
      setError(cause instanceof Error ? cause.message : "Settlement compilation failed.");
    }
  }

  async function signAndSubmit() {
    setError(null);
    if (!artifact || !artifact.intent_id || !walletAddress) return setError("Compile a durable intent and connect the payer wallet first.");
    if (settlementQuoteExpired(calculation?.expires_at_unix, nowUnix)) {
      setFlow("ready");
      return setError("This quote has expired. Create a new quote; no wallet signature was requested.");
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
      if (!response.ok) throw new Error(responseError(response, payload, `Settlement submission failed (${response.status}).`));
      setSettlement(parseSettlementStatusResponse(payload));
      setFlow("complete");
    } catch (cause) {
      if (submitAttempted) {
        try {
          const recovered = await loadIntentStatus(artifact.intent_id);
          setSettlement(recovered);
          setFlow(settlementIntentAllowsRetry(recovered.status) ? "ready" : "complete");
          setError(`The submission request did not complete. The durable intent is ${recovered.status}; do not authorize a second transaction until the server permits retry.`);
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
          setError("The submission outcome is unknown. Do not sign or submit again; recover this durable intent through the authenticated status path.");
          return;
        }
      }
      setFlow("ready");
      setError(cause instanceof Error ? cause.message : "Wallet signing or settlement submission failed.");
    }
  }

  async function loadIntentStatus(intentId: string): Promise<SettlementSubmitResponse> {
    const response = await fetchWithTimeout(`${apiBase}/v1/settlement/intents/${intentId}`, {
      headers: { authorization: `Bearer ${accessToken.trim()}` },
    }, REQUEST_TIMEOUT_MS.default);
    const payload: unknown = await response.json().catch(() => null);
    if (!response.ok) throw new Error(responseError(response, payload, `Status refresh failed (${response.status}).`));
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
      setError(cause instanceof Error ? cause.message : "Status refresh failed.");
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
      if (!response.ok) throw new Error(responseError(response, payload, `Reconciliation failed (${response.status}).`));
      const reconciled = parseSettlementStatusResponse(payload);
      setSettlement(reconciled);
      setFlow(settlementIntentAllowsRetry(reconciled.status) ? "ready" : "complete");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Reconciliation failed.");
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
        <div className="panel-heading"><div><p className="eyebrow">Payment review</p><h2 id="checkout-title">Review payment before signing</h2></div><span className="status-pill">Wallet approval required</span></div>
        <p className="panel-copy">Review the amount, tax, destination, and policy status before your wallet approves the payment. The service checks the payment again before it is sent to the network.</p>
        <div className="checkout-step"><span className="step-number">1</span><div><strong>Wallet to use</strong><p>{walletAddress ? walletAddress.toBase58() : "No wallet connected"}</p></div><WalletPicker disabled={isBusy} connectedAddress={walletAddress?.toBase58()} onConnected={setWalletAddress} /></div>
        <form className="form-grid checkout-form" onSubmit={compile}>
          <label className="full-field"><span>Payment route</span><select value={routeId} onChange={(event) => setRouteId(event.target.value)} disabled={routeState !== "ready" || isBusy} required><option value="">{routeState === "loading" ? "Loading checked routes…" : routeState === "error" ? "Routes unavailable" : routes.length ? "Select a payment route" : "No active routes"}</option>{routes.map((route) => <option value={route.id} key={route.id}>{route.route_key} · version {route.version}</option>)}</select></label>
          {routeError && <p className="inline-error full-field" role="alert">{routeError}</p>}
          {selectedRoute && <div className="route-evidence full-field" aria-live="polite"><strong>Payment route checked</strong><small>This route was selected from your account’s active payment settings and checked before signing.</small><details className="technical-details" aria-label="Technical route details"><summary>Technical route details</summary><small>Merchant account {selectedRoute.merchant_account} · asset mint {selectedRoute.mint} · {selectedRoute.mint_decimals} decimal places · tax account {selectedRoute.tax_vault} · destination account {selectedRoute.net_destination} · token program {selectedRoute.token_program}</small><small>The payer source account is derived as the canonical associated token account. No account address is accepted from this browser form.</small></details></div>}
          <label className="full-field"><span>How payment is shared</span><select value={settlementMode} onChange={(event) => setSettlementMode(event.target.value as "single" | "split")} disabled={isBusy}>
            <option value="single">One approved destination</option>
            <option value="split">Approved recipient plan</option>
          </select></label>
          {settlementMode === "split" && <>
            <label className="full-field"><span>Approved recipient plan</span><select value={splitPlanId} onChange={(event) => setSplitPlanId(event.target.value)} disabled={splitPlanState !== "ready" || isBusy} required>
              <option value="">{splitPlanState === "loading" ? "Loading approved plans…" : splitPlanState === "error" ? "Plans unavailable" : splitPlans.length ? "Select a recipient plan" : "No active recipient plans"}</option>
              {splitPlans.map((plan) => <option value={plan.id} key={plan.id}>{plan.plan_key} · v{plan.version} · {plan.recipients.length} recipients</option>)}
            </select></label>
            {splitPlanError && <div className="inline-error full-field" role="alert">{splitPlanError} <button type="button" className="inline-retry" onClick={() => setSplitPlanReload((value) => value + 1)} disabled={isBusy}>Retry</button></div>}
            {selectedPlan && <div className={`route-evidence full-field${splitPlanCompatible ? "" : " route-evidence-warning"}`} aria-live="polite">
              <strong>{splitPlanCompatible ? "Recipient plan matches the payment route" : "Recipient plan does not match this route"}</strong>
              <small>Version {selectedPlan.version} · {selectedPlan.recipients.length} approved recipients</small>
              <details className="technical-details" aria-label="Technical recipient details"><summary>Recipient account details</summary>{selectedPlan.recipients.map((recipient, index) => <small key={recipient.token_account}><span aria-hidden="true">{index + 1}.</span> {recipient.token_account} · {recipient.basis_points} basis points</small>)}</details>
              {!splitPlanCompatible && <small>Choose a payment route for the plan’s merchant and asset.</small>}
            </div>}
          </>}
          <div className="route-evidence full-field"><strong>Payment safeguards selected automatically</strong><small>The verified quote supplies the policy. A secure payment reference is assigned for safe retry; no protocol values need to be entered.</small></div>
          <button type="submit" disabled={isBusy || quoteExpired || !serviceOnline || !calculation || !selectedRoute || (settlementMode === "split" && !splitPlanCompatible)}>{flow === "compiling" ? "Preparing payment…" : "Prepare payment for review"}</button>
        </form>
        {error && <p className="inline-error" role="alert">{error}</p>}
      </article>

      <article className="panel checkout-review" aria-live="polite">
        <div className="panel-heading"><div><p className="eyebrow">Before you sign</p><h2>Payment review</h2></div>{artifact && <span className="status-pill">Ready for wallet approval</span>}</div>
        {!artifact || !calculation || !selectedRoute ? <>
          <div className="empty-state"><span className="empty-icon">✓</span><strong>Verified quote and payment preparation required</strong><p>Generate a verified quote, then prepare the payment before the wallet approval control becomes available.</p></div>
          {calculation?.expires_at_unix && <p className="inline-warning" role={quoteExpired ? "alert" : undefined}>Quote expires {formatEpoch(calculation.expires_at_unix, locale, timeZone)} ({quoteExpired ? "expired; create a new quote" : "server-enforced"}).</p>}
        </> : <>
          <div className="total-card"><span>Amount your wallet will authorize</span><strong>{formatAtomic(calculation.result.gross_atomic, currency, selectedRoute.mint_decimals)}</strong><small>Tax withheld: {formatAtomic(calculation.result.total_tax_atomic, currency, selectedRoute.mint_decimals)} · Amount sent: {formatAtomic(calculation.result.total_net_atomic, currency, selectedRoute.mint_decimals)}</small></div>
          <dl className="summary-list"><div><dt>Payment route</dt><dd>{selectedRoute.route_key} · version {selectedRoute.version}</dd></div><div><dt>How payment is shared</dt><dd>{"split_plan" in artifact ? `Approved recipient plan · version ${selectedPlan?.version ?? "unknown"}` : "One approved destination"}</dd></div><div><dt>Payment asset</dt><dd>{selectedRoute.mint}</dd></div><div><dt>Tax rule used</dt><dd>{calculation.result.rule_set_id} · version {calculation.result.rule_version}</dd></div><div><dt>Tax status</dt><dd>Calculated; not attested</dd></div><div><dt>Payment reference</dt><dd><code>{nonce}</code></dd></div></dl>
          <details className="technical-details" aria-label="Technical payment details"><summary>Technical payment details</summary><dl className="summary-list"><div><dt>Asset precision</dt><dd>{selectedRoute.mint_decimals} decimal places</dd></div><div><dt>Program ID</dt><dd>{artifact.program_id}</dd></div><div><dt>Policy authority</dt><dd>{artifact.policy_attestation}</dd></div><div><dt>Compute policy</dt><dd>version {artifact.compute_budget_policy.policy_version} · {artifact.compute_budget_policy.compute_unit_limit.toLocaleString(locale)} compute units</dd></div><div><dt>Priority-fee bid</dt><dd>{artifact.compute_budget_policy.micro_lamports_per_compute_unit.toLocaleString(locale)} micro-lamports per compute unit</dd></div><div><dt>Accounts</dt><dd>{artifact.accounts.length} exact account entries</dd></div><div><dt>Payment request</dt><dd>{artifact.intent_id}</dd></div><div><dt>Rule reference</dt><dd><code>{calculation.result.rule_digest.map((byte) => byte.toString(16).padStart(2, "0")).join("")}</code></dd></div></dl></details>
          {"split_plan" in artifact && selectedPlan && <div className="split-review" aria-label="Recipient distribution review"><strong>Recipient shares are bound to this payment</strong><small>{selectedPlan.recipients.length} approved recipients · the service owns recipient and amount authority.</small><details className="technical-details"><summary>Recipient account details</summary>{selectedPlan.recipients.map((recipient) => <div className="split-review-row" key={recipient.token_account}><span>{recipient.token_account}</span><strong>{recipient.basis_points} basis points</strong></div>)}<small>Plan reference {splitPlanDigest}.</small></details></div>}
          {calculation.expires_at_unix && <p className="inline-warning" role={quoteExpired ? "alert" : undefined}>Quote expires {formatEpoch(calculation.expires_at_unix, locale, timeZone)} ({quoteExpired ? "expired; create a new quote" : "server-enforced"}).</p>}
          <p className="inline-warning">Your wallet approves only the payment shown above. Wallet approval is not confirmation that the payment reached the network; use the payment status below.</p>
          <button type="button" onClick={signAndSubmit} disabled={isBusy || quoteExpired || flow !== "ready"}>{flow === "signing" ? "Waiting for wallet…" : flow === "submitting" ? "Sending the reviewed payment…" : "Sign and send payment"}</button>
          {settlement && <div className="settlement-status"><strong>Payment status: {settlement.status}</strong><small>{settlement.signature ? `Network reference ${settlement.signature}` : "No network reference is recorded yet."}</small>{settlement.status === "submitting" && <small role="status">One request is sending this payment. Do not approve it again; refresh or reconcile the recorded payment.</small>}{settlement.status === "unknown" && <small role="status">The network result is not yet clear. Do not approve another payment; reconcile this recorded payment.</small>}<div className="status-actions"><button type="button" onClick={refreshStatus} disabled={isBusy}>Refresh payment status</button><button type="button" onClick={reconcile} disabled={isBusy || !settlement.signature}>Check network record</button></div></div>}
        </>}
      </article>
    </section>
  );
}
