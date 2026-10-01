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
  parsePolicyDigest,
  parseSettlementRoutes,
  parseSettlementStatusResponse,
  parseSettlementSplitPlans,
  signAndEncodeSettlement,
  settlementQuoteExpired,
  settlementIntentAllowsRetry,
} from "./checkout";
import WalletPicker from "./WalletPicker";

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
  const [nonce, setNonce] = useState("");
  const [policySequence, setPolicySequence] = useState("");
  const [policyDigest, setPolicyDigest] = useState("");
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
    if (!routeId || !nonce || !policySequence) {
      return setError("Select an active settlement route, nonce, and policy sequence.");
    }
    if (settlementMode === "split" && (!splitPlanId || !selectedPlan)) {
      return setError("Select an active governed Split Plan before compiling.");
    }
    if (settlementMode === "split" && selectedPlan && selectedRoute
        && (selectedPlan.merchant_account !== selectedRoute.merchant_account || selectedPlan.mint !== selectedRoute.mint)) {
      return setError("The selected Split Plan is not compatible with the selected settlement route.");
    }
    let policyBytes: number[];
    try {
      policyBytes = parsePolicyDigest(policyDigest);
      const numericValues = [nonce, policySequence].map((value) => BigInt(value));
      if (numericValues.some((value) => value <= 0n || value > BigInt(Number.MAX_SAFE_INTEGER))) {
        throw new Error("Nonce and policy sequence must be positive safe integers for this client.");
      }
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
          nonce: Number(nonce),
          policy_sequence: Number(policySequence),
          policy_digest: policyBytes,
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
        <div className="panel-heading"><div><p className="eyebrow">Non-custodial checkout</p><h2 id="checkout-title">Review and authorize settlement</h2></div><span className="status-pill">Wallet required</span></div>
        <p className="panel-copy">The browser serializes the server-compiled settlement and its exact compute-budget policy. Your wallet signs it; the API verifies the complete signed message before any RPC submission.</p>
        <div className="checkout-step"><span className="step-number">1</span><div><strong>Payer wallet</strong><p>{walletAddress ? walletAddress.toBase58() : "No wallet connected"}</p></div><WalletPicker disabled={isBusy} connectedAddress={walletAddress?.toBase58()} onConnected={setWalletAddress} /></div>
        <form className="form-grid checkout-form" onSubmit={compile}>
          <label className="full-field"><span>Configured settlement route</span><select value={routeId} onChange={(event) => setRouteId(event.target.value)} disabled={routeState !== "ready" || isBusy} required><option value="">{routeState === "loading" ? "Loading active routes…" : routeState === "error" ? "Routes unavailable" : routes.length ? "Select a route" : "No active routes"}</option>{routes.map((route) => <option value={route.id} key={route.id}>{route.route_key} · v{route.version} · {route.mint}</option>)}</select></label>
          {routeError && <p className="inline-error full-field" role="alert">{routeError}</p>}
          {selectedRoute && <div className="route-evidence full-field" aria-live="polite"><strong>Tenant configuration resolved by API</strong><small>Merchant {selectedRoute.merchant_account} · mint {selectedRoute.mint} · {selectedRoute.mint_decimals} decimal places · tax vault {selectedRoute.tax_vault} · destination {selectedRoute.net_destination} · {selectedRoute.token_program}</small><small>The payer source account is derived as the canonical associated token account. No account address is accepted from this browser form.</small></div>}
          <label className="full-field"><span>Settlement distribution</span><select value={settlementMode} onChange={(event) => setSettlementMode(event.target.value as "single" | "split")} disabled={isBusy}>
            <option value="single">Single configured destination</option>
            <option value="split">Governed Split Plan</option>
          </select></label>
          {settlementMode === "split" && <>
            <label className="full-field"><span>Active governed Split Plan</span><select value={splitPlanId} onChange={(event) => setSplitPlanId(event.target.value)} disabled={splitPlanState !== "ready" || isBusy} required>
              <option value="">{splitPlanState === "loading" ? "Loading governed plans…" : splitPlanState === "error" ? "Plans unavailable" : splitPlans.length ? "Select a Split Plan" : "No active Split Plans"}</option>
              {splitPlans.map((plan) => <option value={plan.id} key={plan.id}>{plan.plan_key} · v{plan.version} · {plan.recipients.length} recipients</option>)}
            </select></label>
            {splitPlanError && <div className="inline-error full-field" role="alert">{splitPlanError} <button type="button" className="inline-retry" onClick={() => setSplitPlanReload((value) => value + 1)} disabled={isBusy}>Retry</button></div>}
            {selectedPlan && <div className={`route-evidence full-field${splitPlanCompatible ? "" : " route-evidence-warning"}`} aria-live="polite">
              <strong>{splitPlanCompatible ? "Split Plan matches the selected route" : "Split Plan route mismatch"}</strong>
              <small>Plan v{selectedPlan.version} · digest {splitPlanDigest.slice(0, 16)}… · {selectedPlan.recipients.length} bounded recipients</small>
              {selectedPlan.recipients.map((recipient, index) => <small key={recipient.token_account}><span aria-hidden="true">{index + 1}.</span> {recipient.token_account} · {recipient.basis_points} bps</small>)}
              {!splitPlanCompatible && <small>Choose a route with merchant {selectedPlan.merchant_account} and mint {selectedPlan.mint}.</small>}
            </div>}
          </>}
          <label><span>Client nonce</span><input inputMode="numeric" value={nonce} onChange={(event) => setNonce(event.target.value)} placeholder="Positive integer" required /></label>
          <label><span>Policy sequence</span><input inputMode="numeric" value={policySequence} onChange={(event) => setPolicySequence(event.target.value)} placeholder="Positive integer" required /></label>
          <label><span>Policy digest (hex)</span><input value={policyDigest} onChange={(event) => setPolicyDigest(event.target.value)} placeholder="64 hexadecimal characters" autoComplete="off" spellCheck={false} required /></label>
          <button type="submit" disabled={isBusy || quoteExpired || !serviceOnline || !calculation || !selectedRoute || (settlementMode === "split" && !splitPlanCompatible)}>{flow === "compiling" ? "Compiling durable intent…" : "Compile for review"}</button>
        </form>
        {error && <p className="inline-error" role="alert">{error}</p>}
      </article>

      <article className="panel checkout-review" aria-live="polite">
        <div className="panel-heading"><div><p className="eyebrow">Authorization boundary</p><h2>Exact review</h2></div>{artifact && <span className="status-pill">Ready to sign</span>}</div>
        {!artifact || !calculation || !selectedRoute ? <>
          <div className="empty-state"><span className="empty-icon">✓</span><strong>Quote and compiled intent required</strong><p>Generate an authenticated quote, then compile the exact instruction before the wallet control becomes available.</p></div>
          {calculation?.expires_at_unix && <p className="inline-warning" role={quoteExpired ? "alert" : undefined}>Quote expires {formatEpoch(calculation.expires_at_unix, locale, timeZone)} ({quoteExpired ? "expired; create a new quote" : "server-enforced"}).</p>}
        </> : <>
          <div className="total-card"><span>Total wallet authorization</span><strong>{formatAtomic(calculation.result.gross_atomic, currency, selectedRoute.mint_decimals)}</strong><small>Tax withheld: {formatAtomic(calculation.result.total_tax_atomic, currency, selectedRoute.mint_decimals)} · Net: {formatAtomic(calculation.result.total_net_atomic, currency, selectedRoute.mint_decimals)}</small></div>
          <dl className="summary-list"><div><dt>Route</dt><dd>{selectedRoute.route_key} v{selectedRoute.version}</dd></div><div><dt>Distribution</dt><dd>{"split_plan" in artifact ? `Split Plan v${selectedPlan?.version ?? "unknown"}` : "Single destination"}</dd></div><div><dt>Asset mint</dt><dd>{selectedRoute.mint}</dd></div><div><dt>Asset precision</dt><dd>{selectedRoute.mint_decimals} decimal places</dd></div><div><dt>Program</dt><dd>{artifact.program_id}</dd></div><div><dt>Policy sequence</dt><dd>{policySequence}</dd></div><div><dt>Compute policy</dt><dd>v{artifact.compute_budget_policy.policy_version} · {artifact.compute_budget_policy.compute_unit_limit.toLocaleString(locale)} CU</dd></div><div><dt>Priority-fee bid</dt><dd>{artifact.compute_budget_policy.micro_lamports_per_compute_unit.toLocaleString(locale)} μ-lamports/CU</dd></div><div><dt>Accounts</dt><dd>{artifact.accounts.length} exact metas</dd></div><div><dt>Intent</dt><dd>{artifact.intent_id}</dd></div></dl>
          <dl className="summary-list"><div><dt>Published rule</dt><dd>{calculation.result.rule_set_id} v{calculation.result.rule_version}</dd></div><div><dt>Rule digest</dt><dd><code>{calculation.result.rule_digest.map((byte) => byte.toString(16).padStart(2, "0")).join("")}</code></dd></div><div><dt>Policy state</dt><dd>Calculated, not attested</dd></div></dl>
          {"split_plan" in artifact && selectedPlan && <div className="split-review" aria-label="Split Plan signing review"><strong>Recipient distribution is bound to this compiled instruction</strong>{selectedPlan.recipients.map((recipient) => <div className="split-review-row" key={recipient.token_account}><span>{recipient.token_account}</span><strong>{recipient.basis_points} bps</strong></div>)}<small>Plan digest {splitPlanDigest}. The API, not this browser, owns recipient and amount authority.</small></div>}
          {calculation.expires_at_unix && <p className="inline-warning" role={quoteExpired ? "alert" : undefined}>Quote expires {formatEpoch(calculation.expires_at_unix, locale, timeZone)} ({quoteExpired ? "expired; create a new quote" : "server-enforced"}).</p>}
          <p className="inline-warning">Signing authorizes only the reviewed instruction. A wallet signature is not settlement confirmation; the server status below is authoritative.</p>
          <button type="button" onClick={signAndSubmit} disabled={isBusy || quoteExpired || flow !== "ready"}>{flow === "signing" ? "Waiting for wallet…" : flow === "submitting" ? "Submitting unchanged bytes…" : "Sign and submit"}</button>
          {settlement && <div className="settlement-status"><strong>Durable status: {settlement.status}</strong><small>{settlement.signature ? `Signature ${settlement.signature}` : "No chain signature is recorded yet."}</small>{settlement.status === "submitting" && <small role="status">One request owns submission. Do not sign or submit again; refresh or reconcile this intent for chain evidence.</small>}{settlement.status === "unknown" && <small role="status">Provider outcome is ambiguous. Do not authorize another transaction; reconcile this recorded signature.</small>}<div className="status-actions"><button type="button" onClick={refreshStatus} disabled={isBusy}>Refresh status</button><button type="button" onClick={reconcile} disabled={isBusy || !settlement.signature}>Reconcile evidence</button></div></div>}
        </>}
      </article>
    </section>
  );
}
