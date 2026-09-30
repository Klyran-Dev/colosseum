import { FormEvent, useEffect, useState } from "react";
import { PublicKey } from "@solana/web3.js";
import { fetchWithTimeout, REQUEST_TIMEOUT_MS, responseError } from "./api";
import { createClientUuid } from "./clientIdentity";
import { DEFAULT_LOCALE, DEFAULT_TIME_ZONE, formatEpoch } from "./formatters";
import {
  CompiledDocumentSettlementResponse,
  SettlementRoute,
  SettlementSubmitResponse,
  parseCompiledSettlementArtifact,
  parsePolicyDigest,
  parseSettlementRoutes,
  parseSettlementStatusResponse,
  signAndEncodeSettlement,
  settlementIntentAllowsRetry,
  settlementQuoteExpired,
} from "./checkout";
import WalletPicker from "./WalletPicker";
import {
  parseDocumentAttestationJson,
  parseDocumentAttestationVerification,
  parseDocumentScopeLineIds,
  DocumentAttestationVerification,
} from "./documentCheckout";

type Props = {
  apiBase: string;
  serviceOnline: boolean;
  accessToken: string;
  initialCalculationId?: string;
  locale?: string;
  timeZone?: string;
};

type Flow = "idle" | "verifying" | "compiling" | "ready" | "signing" | "submitting" | "complete";

function positiveSafeInteger(value: string, field: string): number {
  const parsed = BigInt(value);
  if (parsed <= 0n || parsed > BigInt(Number.MAX_SAFE_INTEGER)) throw new Error(`${field} must be a positive safe integer.`);
  return Number(parsed);
}

function shortDigest(value: number[]): string {
  return value.map((byte) => byte.toString(16).padStart(2, "0")).join("").slice(0, 16) + "…";
}

export default function DocumentCheckoutPanel({
  apiBase,
  serviceOnline,
  accessToken,
  initialCalculationId = "",
  locale = DEFAULT_LOCALE,
  timeZone = DEFAULT_TIME_ZONE,
}: Props) {
  const [walletAddress, setWalletAddress] = useState<PublicKey | null>(null);
  const [routes, setRoutes] = useState<SettlementRoute[]>([]);
  const [routeId, setRouteId] = useState("");
  const [routeState, setRouteState] = useState<"idle" | "loading" | "ready" | "error">("idle");
  const [routeError, setRouteError] = useState<string | null>(null);
  const [calculationId, setCalculationId] = useState("");
  const [attestationJson, setAttestationJson] = useState("");
  const [policyDigest, setPolicyDigest] = useState("");
  const [scopeKind, setScopeKind] = useState<"full_document" | "line_subset">("full_document");
  const [lineIds, setLineIds] = useState("");
  const [idempotencyKey, setIdempotencyKey] = useState(() => createClientUuid());
  const [nonce, setNonce] = useState("");
  const [policySequence, setPolicySequence] = useState("");
  const [attestation, setAttestation] = useState<DocumentAttestationVerification | null>(null);
  const [artifact, setArtifact] = useState<CompiledDocumentSettlementResponse | null>(null);
  const [settlement, setSettlement] = useState<SettlementSubmitResponse | null>(null);
  const [flow, setFlow] = useState<Flow>("idle");
  const [error, setError] = useState<string | null>(null);
  const [nowUnix, setNowUnix] = useState(() => Math.floor(Date.now() / 1000));

  useEffect(() => {
    const timer = window.setInterval(() => setNowUnix(Math.floor(Date.now() / 1000)), 1_000);
    return () => window.clearInterval(timer);
  }, []);

  useEffect(() => {
    if (initialCalculationId) setCalculationId(initialCalculationId);
  }, [initialCalculationId]);

  useEffect(() => {
    if (!serviceOnline || !accessToken.trim()) {
      setRoutes([]);
      setRouteId("");
      setRouteState("idle");
      return;
    }
    const controller = new AbortController();
    setRouteState("loading");
    setRouteError(null);
    fetchWithTimeout(`${apiBase}/v1/settlement/routes`, {
      headers: { authorization: `Bearer ${accessToken.trim()}` },
      signal: controller.signal,
    }, REQUEST_TIMEOUT_MS.default).then(async (response) => {
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw new Error(responseError(response, payload, `Route loading failed (${response.status}).`));
      const loaded = parseSettlementRoutes(payload);
      setRoutes(loaded);
      setRouteId((current) => loaded.some((route) => route.id === current) ? current : loaded[0]?.id ?? "");
      setRouteState("ready");
    }).catch((cause: unknown) => {
      if (cause instanceof DOMException && cause.name === "AbortError") return;
      setRouteState("error");
      setRouteError(cause instanceof Error ? cause.message : "Configured settlement routes could not be loaded.");
    });
    return () => controller.abort();
  }, [accessToken, apiBase, serviceOnline]);

  function resetPreparedState() {
    setArtifact(null);
    setSettlement(null);
    setFlow("idle");
  }

  async function verifyAttestation(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);
    resetPreparedState();
    if (!serviceOnline) return setError("The API is unavailable; no browser-side attestation verification is used.");
    if (!accessToken.trim()) return setError("An access token with tax:attestation:verify and tax:settlement:compile scopes is required.");
    if (!calculationId.trim() || !routeId) return setError("Enter the document calculation ID and select an active route.");
    let signedAttestation: Record<string, unknown>;
    try {
      signedAttestation = parseDocumentAttestationJson(attestationJson);
    } catch (cause) {
      return setError(cause instanceof Error ? cause.message : "The signed document attestation is invalid.");
    }
    setFlow("verifying");
    try {
      const response = await fetchWithTimeout(`${apiBase}/v1/tax/document-quotes/${encodeURIComponent(calculationId.trim())}/attestation/verify`, {
        method: "POST",
        headers: { "content-type": "application/json", authorization: `Bearer ${accessToken.trim()}` },
        body: JSON.stringify({ route_id: routeId, attestation: signedAttestation }),
      }, REQUEST_TIMEOUT_MS.default);
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw new Error(responseError(response, payload, `Document attestation verification failed (${response.status}).`));
      setAttestation(parseDocumentAttestationVerification(payload));
      setFlow("idle");
    } catch (cause) {
      setFlow("idle");
      setError(cause instanceof Error ? cause.message : "Document attestation verification failed.");
    }
  }

  async function compile(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);
    if (!attestation) return setError("Verify the server-owned document attestation before compilation.");
    if (!walletAddress || !routeId) return setError("Connect the payer wallet and select an active route.");
    if (settlementQuoteExpired(attestation.expires_at_unix, nowUnix)) return setError("The document attestation has expired; verify a new signed attestation.");
    let policyBytes: number[];
    let scope: { kind: "full_document" } | { kind: "line_subset"; line_ids: string[] };
    try {
      policyBytes = parsePolicyDigest(policyDigest);
      positiveSafeInteger(nonce, "Client nonce");
      positiveSafeInteger(policySequence, "Policy sequence");
      scope = scopeKind === "full_document" ? { kind: "full_document" } : { kind: "line_subset", line_ids: parseDocumentScopeLineIds(lineIds) };
      if (!idempotencyKey.trim()) throw new Error("An idempotency key is required for durable document compilation.");
    } catch (cause) {
      return setError(cause instanceof Error ? cause.message : "Document settlement fields are invalid.");
    }
    setFlow("compiling");
    try {
      const response = await fetchWithTimeout(`${apiBase}/v1/settlement/document/compile-from-quote`, {
        method: "POST",
        headers: { "content-type": "application/json", authorization: `Bearer ${accessToken.trim()}` },
        body: JSON.stringify({
          idempotency_key: idempotencyKey.trim(),
          calculation_id: attestation.calculation_id,
          route_id: routeId,
          payer: walletAddress.toBase58(),
          nonce: positiveSafeInteger(nonce, "Client nonce"),
          policy_sequence: positiveSafeInteger(policySequence, "Policy sequence"),
          policy_digest: policyBytes,
          scope,
        }),
      }, REQUEST_TIMEOUT_MS.default);
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw new Error(responseError(response, payload, `Document settlement compilation failed (${response.status}).`));
      const parsed = parseCompiledSettlementArtifact(payload);
      if (!("authorization" in parsed) || !parsed.intent_id) throw new Error("The API did not return a durable document intent; signing is disabled.");
      setArtifact(parsed as CompiledDocumentSettlementResponse);
      setFlow("ready");
    } catch (cause) {
      setFlow("idle");
      setError(cause instanceof Error ? cause.message : "Document settlement compilation failed.");
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

  async function signAndSubmit() {
    if (!artifact?.intent_id || !walletAddress) return setError("Compile a durable document intent and connect the payer wallet first.");
    if (settlementQuoteExpired(attestation?.expires_at_unix, nowUnix)) return setError("The document attestation expired; no wallet signature was requested.");
    setError(null);
    let submitAttempted = false;
    setFlow("signing");
    try {
      const signed = await signAndEncodeSettlement(artifact, walletAddress, import.meta.env.VITE_SOLANA_RPC_URL ?? "");
      setFlow("submitting");
      submitAttempted = true;
      const response = await fetchWithTimeout(`${apiBase}/v1/settlement/submit`, {
        method: "POST",
        headers: { "content-type": "application/json", authorization: `Bearer ${accessToken.trim()}` },
        body: JSON.stringify({ intent_id: artifact.intent_id, signed_transaction_base64: signed, instruction: artifact }),
      }, REQUEST_TIMEOUT_MS.settlementSubmit);
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw new Error(responseError(response, payload, `Document settlement submission failed (${response.status}).`));
      setSettlement(parseSettlementStatusResponse(payload));
      setFlow("complete");
    } catch (cause) {
      if (submitAttempted && artifact.intent_id) {
        try {
          const recovered = await loadIntentStatus(artifact.intent_id);
          setSettlement(recovered);
          setFlow(settlementIntentAllowsRetry(recovered.status) ? "ready" : "complete");
          setError(`The submission outcome is durable as ${recovered.status}; do not sign again until the server permits retry.`);
          return;
        } catch {
          setFlow("complete");
          setError("The submission outcome is unknown. Do not sign again; recover this intent through the authenticated status path.");
          return;
        }
      }
      setFlow("ready");
      setError(cause instanceof Error ? cause.message : "Document wallet signing or submission failed.");
    }
  }

  async function refreshStatus() {
    if (!settlement?.intent_id) return;
    try {
      const refreshed = await loadIntentStatus(settlement.intent_id);
      setSettlement(refreshed);
      setFlow(settlementIntentAllowsRetry(refreshed.status) ? "ready" : "complete");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Status refresh failed.");
    }
  }

  async function reconcile() {
    if (!settlement?.intent_id || !settlement.signature) return;
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

  const isBusy = ["verifying", "compiling", "signing", "submitting"].includes(flow);
  const selectedRoute = routes.find((route) => route.id === routeId) ?? null;
  const expired = settlementQuoteExpired(attestation?.expires_at_unix, nowUnix);

  return <section className="checkout-layout" aria-labelledby="document-checkout-title">
    <article className="panel">
      <div className="panel-heading"><div><p className="eyebrow">Document-bound non-custodial checkout</p><h2 id="document-checkout-title">Authorize a documented payment</h2></div><span className="status-pill">Attestation required</span></div>
      <p className="panel-copy">This workspace keeps document evidence separate from aggregate checkout. The API owns document totals, line scope, route accounts, and the immutable settlement intent; the browser only presents and signs the returned artifact.</p>
      <div className="checkout-step"><span className="step-number">1</span><div><strong>Payer wallet</strong><p>{walletAddress ? walletAddress.toBase58() : "No wallet connected"}</p></div><WalletPicker disabled={isBusy} connectedAddress={walletAddress?.toBase58()} onConnected={setWalletAddress} /></div>
      <form className="form-grid checkout-form" onSubmit={verifyAttestation}>
        <label className="full-field"><span>Document calculation ID (UUIDv7)</span><input value={calculationId} onChange={(event) => { setCalculationId(event.target.value); setAttestation(null); resetPreparedState(); }} autoComplete="off" required /></label>
        <label className="full-field"><span>Active settlement route</span><select value={routeId} onChange={(event) => { setRouteId(event.target.value); setAttestation(null); resetPreparedState(); }} disabled={routeState !== "ready" || isBusy} required><option value="">{routeState === "loading" ? "Loading active routes…" : routeState === "error" ? "Routes unavailable" : routes.length ? "Select a route" : "No active routes"}</option>{routes.map((route) => <option value={route.id} key={route.id}>{route.route_key} · v{route.version} · {route.mint}</option>)}</select></label>
        {routeError && <p className="inline-error full-field" role="alert">{routeError}</p>}
        {selectedRoute && <div className="route-evidence full-field" aria-live="polite"><strong>Server-owned route selected</strong><small>Mint {selectedRoute.mint} · token program {selectedRoute.token_program}</small><small>Merchant-side accounts are not accepted from this form.</small></div>}
        <label className="full-field"><span>Issuer-signed document attestation (JSON)</span><textarea value={attestationJson} onChange={(event) => { setAttestationJson(event.target.value); setAttestation(null); resetPreparedState(); }} rows={8} autoComplete="off" spellCheck={false} required /><small className="field-help">The browser treats this as untrusted until the server verifies its signature, document binding, route mint, validity, facts digest, and amount conservation.</small></label>
        <button type="submit" disabled={isBusy || !serviceOnline || !accessToken.trim()}>{flow === "verifying" ? "Verifying issuer evidence…" : attestation ? "Attestation verified" : "Verify document attestation"}</button>
      </form>
      {attestation && <div className="route-evidence" role="status"><strong>Document attested, not settled</strong><small>Payload digest <code>{shortDigest(attestation.payload_digest)}</code> · issuer key <code>{shortDigest(attestation.issuer_public_key)}</code></small><small>Expires {formatEpoch(attestation.expires_at_unix, locale, timeZone)}{expired ? " · expired; verify a new attestation" : " · server-enforced"}</small></div>}
    </article>

    <article className="panel">
      <div className="panel-heading"><div><p className="eyebrow">Scoped settlement intent</p><h2>Compile, review, then sign</h2></div><span className="status-pill">No client arithmetic</span></div>
      <form className="form-grid checkout-form" onSubmit={compile}>
        <label><span>Payment scope</span><select value={scopeKind} onChange={(event) => { setScopeKind(event.target.value as typeof scopeKind); resetPreparedState(); }} disabled={isBusy}><option value="full_document">Full document</option><option value="line_subset">Complete line subset</option></select></label>
        {scopeKind === "line_subset" && <label className="full-field"><span>Complete line IDs (comma-separated)</span><input value={lineIds} onChange={(event) => { setLineIds(event.target.value); resetPreparedState(); }} placeholder="line-a,line-c" disabled={isBusy} required /><small className="field-help">Partial quantities and arbitrary proration are rejected. Request a new quote for a partial line.</small></label>}
        <label><span>Client nonce</span><input inputMode="numeric" value={nonce} onChange={(event) => { setNonce(event.target.value); resetPreparedState(); }} placeholder="Positive integer" disabled={isBusy} required /></label>
        <label><span>Policy sequence</span><input inputMode="numeric" value={policySequence} onChange={(event) => { setPolicySequence(event.target.value); resetPreparedState(); }} placeholder="Positive integer" disabled={isBusy} required /></label>
        <label className="full-field"><span>Policy digest (hex)</span><input value={policyDigest} onChange={(event) => { setPolicyDigest(event.target.value); resetPreparedState(); }} placeholder="64 hexadecimal characters" autoComplete="off" spellCheck={false} disabled={isBusy} required /></label>
        <label className="full-field"><span>Idempotency key (memory-only)</span><input value={idempotencyKey} onChange={(event) => setIdempotencyKey(event.target.value)} autoComplete="off" disabled={isBusy} required /></label>
        <button type="submit" disabled={isBusy || !attestation || expired || !walletAddress || !serviceOnline}>{flow === "compiling" ? "Compiling durable document intent…" : "Compile document payment for review"}</button>
      </form>
      {artifact && <div className="settlement-status" aria-live="polite"><strong>Server-derived payment authority</strong><dl className="summary-list"><div><dt>Gross</dt><dd>{artifact.authorization.gross_atomic} atomic units</dd></div><div><dt>Tax withheld</dt><dd>{artifact.authorization.tax_atomic} atomic units</dd></div><div><dt>Net destination</dt><dd>{artifact.authorization.net_atomic} atomic units</dd></div><div><dt>Asset mint</dt><dd><code>{selectedRoute?.mint ?? "Server-bound by route"}</code></dd></div><div><dt>Payment scope digest</dt><dd><code>{shortDigest(artifact.payment_scope_digest)}</code></dd></div></dl><small>Document attestation <code>{shortDigest(artifact.document_attestation_digest)}</code> · intent {artifact.intent_id}</small><button type="button" onClick={signAndSubmit} disabled={isBusy || flow !== "ready" || expired}>{flow === "signing" ? "Waiting for wallet…" : flow === "submitting" ? "Submitting unchanged bytes…" : "Sign and submit"}</button>{settlement && <div className="settlement-status"><strong>Durable status: {settlement.status}</strong><small>{settlement.signature ? `Signature ${settlement.signature}` : "No chain signature is recorded yet."}</small><div className="status-actions"><button type="button" onClick={refreshStatus} disabled={isBusy}>Refresh status</button><button type="button" onClick={reconcile} disabled={isBusy || !settlement.signature}>Reconcile evidence</button></div></div>}</div>}
      {!artifact && <div className="empty-state"><strong>Verified document evidence required</strong><p>Verify the issuer attestation first. Compilation remains unavailable until the server returns a durable, scope-bound document intent.</p></div>}
      {error && <p className="inline-error" role="alert">{error}</p>}
      <p className="inline-warning">A successful signature is not settlement confirmation. Finalized Solana evidence decides whether funds moved. This workflow is withholding authorization only, not filing, reporting, legal compliance, or tax advice.</p>
    </article>
  </section>;
}
