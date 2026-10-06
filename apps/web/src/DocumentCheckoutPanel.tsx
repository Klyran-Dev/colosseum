import { FormEvent, useEffect, useState } from "react";
import { PublicKey } from "@solana/web3.js";
import { fetchWithTimeout, operatorResponseError, REQUEST_TIMEOUT_MS } from "./api";
import { createClientNonce, createClientUuid } from "./clientIdentity";
import { DEFAULT_LOCALE, DEFAULT_TIME_ZONE, formatEpoch } from "./formatters";
import {
  CompiledDocumentSettlementResponse,
  SettlementRoute,
  SettlementSubmitResponse,
  parseCompiledSettlementArtifact,
  parseSettlementRoutes,
  parseSettlementStatusResponse,
  signAndEncodeSettlement,
  settlementIntentAllowsRetry,
  settlementQuoteExpired,
} from "./checkout";
import WalletPicker from "./WalletPicker";
import ApplicabilityVerificationCard from "./ApplicabilityVerificationCard";
import { documentCheckoutMessagesFor } from "./documentCheckoutMessages";
import { operatorErrorMessage } from "./operatorError";
import {
  buildDocumentAttestationStatusUrl,
  parseDocumentAttestationStatus,
  parseDocumentScopeLineIds,
  DocumentAttestationStatus,
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

function positiveCanonicalU64(value: string, field: string, invalidMessage?: string): string {
  const parsed = BigInt(value);
  if (parsed <= 0n || parsed > 18_446_744_073_709_551_615n) throw new Error(invalidMessage ?? `${field} must be a positive unsigned 64-bit integer.`);
  return parsed.toString();
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
  const messages = documentCheckoutMessagesFor(locale);
  const [walletAddress, setWalletAddress] = useState<PublicKey | null>(null);
  const [routes, setRoutes] = useState<SettlementRoute[]>([]);
  const [routeId, setRouteId] = useState("");
  const [routeState, setRouteState] = useState<"idle" | "loading" | "ready" | "error">("idle");
  const [routeError, setRouteError] = useState<string | null>(null);
  const [calculationId, setCalculationId] = useState("");
  const [scopeKind, setScopeKind] = useState<"full_document" | "line_subset">("full_document");
  const [lineIds, setLineIds] = useState("");
  const [idempotencyKey] = useState(() => createClientUuid());
  const [nonce, setNonce] = useState(() => createClientNonce());
  const [attestation, setAttestation] = useState<DocumentAttestationStatus | null>(null);
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
    setNonce(createClientNonce());
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
      if (!response.ok) throw operatorResponseError(response, payload, messages.errors.routeRequestFailed(response.status));
      const loaded = parseSettlementRoutes(payload);
      setRoutes(loaded);
      setRouteId((current) => loaded.some((route) => route.id === current) ? current : loaded[0]?.id ?? "");
      setRouteState("ready");
    }).catch((cause: unknown) => {
      if (cause instanceof DOMException && cause.name === "AbortError") return;
      setRouteState("error");
      setRouteError(operatorErrorMessage(cause, messages.evidence.routesUnavailable));
    });
    return () => controller.abort();
  }, [accessToken, apiBase, locale, serviceOnline]);

  function resetPreparedState() {
    setArtifact(null);
    setSettlement(null);
    setFlow("idle");
  }

  async function checkInvoiceEvidence(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);
    resetPreparedState();
    if (!serviceOnline) return setError(messages.errors.serviceUnavailable);
    if (!accessToken.trim()) return setError(messages.errors.authorizationRequired);
    if (!calculationId.trim() || !routeId) return setError(messages.errors.inputRequired);
    try {
      setFlow("verifying");
      const response = await fetchWithTimeout(buildDocumentAttestationStatusUrl(apiBase, calculationId.trim(), routeId), {
        headers: { authorization: `Bearer ${accessToken.trim()}` },
      }, REQUEST_TIMEOUT_MS.default);
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw operatorResponseError(response, payload, messages.errors.evidenceCheckFailed(response.status));
      setAttestation(parseDocumentAttestationStatus(payload));
      setFlow("idle");
    } catch (cause) {
      setFlow("idle");
      setError(operatorErrorMessage(cause, messages.errors.evidenceCheckUnavailable));
    }
  }

  async function compile(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);
    if (!attestation) return setError(messages.errors.invoiceEvidenceRequired);
    if (!walletAddress || !routeId) return setError(messages.errors.walletAndRouteRequired);
    if (settlementQuoteExpired(attestation.expires_at_unix, nowUnix)) return setError(messages.errors.attestationExpired);
    let scope: { kind: "full_document" } | { kind: "line_subset"; line_ids: string[] };
    try {
      positiveCanonicalU64(nonce, "Payment reference number", messages.errors.invalidFields);
      scope = scopeKind === "full_document" ? { kind: "full_document" } : { kind: "line_subset", line_ids: parseDocumentScopeLineIds(lineIds) };
      if (!idempotencyKey.trim()) throw new Error(messages.errors.idempotencyRequired);
    } catch (cause) {
      return setError(operatorErrorMessage(cause, messages.errors.invalidFields));
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
          nonce: positiveCanonicalU64(nonce, "Payment reference number", messages.errors.invalidFields),
          scope,
        }),
      }, REQUEST_TIMEOUT_MS.default);
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw operatorResponseError(response, payload, messages.errors.compileRequestFailed(response.status));
      const parsed = parseCompiledSettlementArtifact(payload);
      if (!("authorization" in parsed) || !parsed.intent_id) throw new Error(messages.errors.intentRequired);
      setArtifact(parsed as CompiledDocumentSettlementResponse);
      setFlow("ready");
    } catch (cause) {
      setFlow("idle");
      setError(operatorErrorMessage(cause, messages.errors.compileFailed));
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

  async function signAndSubmit() {
    if (!artifact?.intent_id || !walletAddress) return setError(messages.errors.prepareRequired);
    if (settlementQuoteExpired(attestation?.expires_at_unix, nowUnix)) return setError(messages.errors.signatureExpired);
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
      if (!response.ok) throw operatorResponseError(response, payload, messages.errors.submissionRequestFailed(response.status));
      setSettlement(parseSettlementStatusResponse(payload));
      setFlow("complete");
    } catch (cause) {
      if (submitAttempted && artifact.intent_id) {
        try {
          const recovered = await loadIntentStatus(artifact.intent_id);
          setSettlement(recovered);
          setFlow(settlementIntentAllowsRetry(recovered.status) ? "ready" : "complete");
          setError(messages.errors.intentRecovery(recovered.status));
          return;
        } catch {
          setFlow("complete");
          setError(messages.errors.submissionUnknown);
          return;
        }
      }
      setFlow("ready");
      setError(operatorErrorMessage(cause, messages.errors.signingFailed));
    }
  }

  async function refreshStatus() {
    if (!settlement?.intent_id) return;
    try {
      const refreshed = await loadIntentStatus(settlement.intent_id);
      setSettlement(refreshed);
      setFlow(settlementIntentAllowsRetry(refreshed.status) ? "ready" : "complete");
    } catch (cause) {
      setError(operatorErrorMessage(cause, messages.errors.statusRefreshFailed));
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
      if (!response.ok) throw operatorResponseError(response, payload, messages.errors.reconciliationRequestFailed(response.status));
      const reconciled = parseSettlementStatusResponse(payload);
      setSettlement(reconciled);
      setFlow(settlementIntentAllowsRetry(reconciled.status) ? "ready" : "complete");
    } catch (cause) {
      setError(operatorErrorMessage(cause, messages.errors.reconciliationFailed));
    }
  }

  const isBusy = ["verifying", "compiling", "signing", "submitting"].includes(flow);
  const selectedRoute = routes.find((route) => route.id === routeId) ?? null;
  const expired = settlementQuoteExpired(attestation?.expires_at_unix, nowUnix);
  const indexerStreamKey = import.meta.env.VITE_INDEXER_STREAM_KEY ?? "";
  return <section className="checkout-layout" aria-labelledby="document-checkout-title">
    <article className="panel">
      <div className="panel-heading"><div><p className="eyebrow">{messages.evidence.eyebrow}</p><h2 id="document-checkout-title">{messages.evidence.title}</h2></div><span className="status-pill">{messages.evidence.status}</span></div>
      <p className="panel-copy">{messages.evidence.description}</p>
      <div className="checkout-step"><span className="step-number">1</span><div><strong>{walletAddress ? messages.evidence.walletConnected : messages.evidence.walletStep}</strong><p>{walletAddress ? walletAddress.toBase58() : messages.evidence.walletMissing}</p></div><WalletPicker locale={locale} disabled={isBusy} connectedAddress={walletAddress?.toBase58()} onConnected={setWalletAddress} /></div>
      <form className="form-grid checkout-form" onSubmit={checkInvoiceEvidence}>
        <label className="full-field"><span>{messages.evidence.invoiceReference}</span><input value={calculationId} onChange={(event) => { setCalculationId(event.target.value); setAttestation(null); resetPreparedState(); }} autoComplete="off" required /></label>
        <label className="full-field"><span>{messages.evidence.paymentRoute}</span><select value={routeId} onChange={(event) => { setRouteId(event.target.value); setAttestation(null); resetPreparedState(); }} disabled={routeState !== "ready" || isBusy} required><option value="">{routeState === "loading" ? messages.evidence.loadingRoutes : routeState === "error" ? messages.evidence.routesUnavailable : routes.length ? messages.evidence.selectRoute : messages.evidence.noRoutes}</option>{routes.map((route) => <option value={route.id} key={route.id}>{route.route_key} · v{route.version} · {route.mint}</option>)}</select></label>
        {routeError && <p className="inline-error full-field" role="alert">{routeError}</p>}
        {selectedRoute && <div className="route-evidence full-field" aria-live="polite"><strong>{messages.evidence.verifiedRoute}</strong><small>{messages.evidence.paymentAsset} {selectedRoute.mint}</small><small>{messages.evidence.recipientProtection}</small></div>}
        <div className="route-evidence full-field" aria-live="polite"><strong>{attestation ? messages.evidence.checkedTitle : messages.evidence.pendingTitle}</strong><small>{attestation ? messages.evidence.checkedCopy : messages.evidence.pendingCopy}</small>{attestation && <details className="technical-details"><summary>{messages.evidence.recordDetails}</summary><small>{messages.evidence.checkedAt} {formatEpoch(attestation.checked_at_unix, locale, timeZone)} · {messages.evidence.expires} {formatEpoch(attestation.expires_at_unix, locale, timeZone)}</small><small>{messages.evidence.evidenceFingerprint} <code>{shortDigest(attestation.payload_digest)}</code> · {messages.evidence.issuerKey} <code>{shortDigest(attestation.issuer_public_key)}</code></small></details>}</div>
        <button type="submit" disabled={isBusy || !serviceOnline || !accessToken.trim()}>{flow === "verifying" ? messages.evidence.checking : attestation ? messages.evidence.checked : messages.evidence.check}</button>
      </form>
      {attestation && <div className="route-evidence" role="status"><strong>{messages.evidence.verifiedNotSent}</strong><small>{messages.evidence.evidenceRecord} <code>{shortDigest(attestation.payload_digest)}</code> · {messages.evidence.signingAuthority} <code>{shortDigest(attestation.issuer_public_key)}</code></small><small>{messages.evidence.expires} {formatEpoch(attestation.expires_at_unix, locale, timeZone)} · {messages.status.expiresChecked(expired)}</small></div>}
    </article>

    <article className="panel">
      <div className="panel-heading"><div><p className="eyebrow">{messages.review.eyebrow}</p><h2>{messages.review.title}</h2></div><span className="status-pill">{messages.review.status}</span></div>
      <form className="form-grid checkout-form" onSubmit={compile}>
        <label><span>{messages.review.paymentScope}</span><select value={scopeKind} onChange={(event) => { setScopeKind(event.target.value as typeof scopeKind); resetPreparedState(); }} disabled={isBusy}><option value="full_document">{messages.review.fullDocument}</option><option value="line_subset">{messages.review.lineSubset}</option></select></label>
        {scopeKind === "line_subset" && <label className="full-field"><span>{messages.review.invoiceLines}</span><input value={lineIds} onChange={(event) => { setLineIds(event.target.value); resetPreparedState(); }} placeholder="line-a,line-c" disabled={isBusy} required /><small className="field-help">{messages.review.lineHelp}</small></label>}
        <div className="route-evidence full-field"><strong>{messages.review.safeguardsTitle}</strong><small>{messages.review.safeguardsCopy}</small></div>
        <button type="submit" disabled={isBusy || !attestation || expired || !walletAddress || !serviceOnline}>{flow === "compiling" ? messages.review.preparing : messages.review.prepare}</button>
      </form>
      {artifact && <div className="settlement-status" aria-live="polite"><strong>{messages.review.readyTitle}</strong><p className="inline-success">{messages.review.readyCopy}</p><dl className="summary-list"><div><dt>{messages.review.amountBeforeTax}</dt><dd>{artifact.authorization.gross_atomic}</dd></div><div><dt>{messages.review.taxWithheld}</dt><dd>{artifact.authorization.tax_atomic}</dd></div><div><dt>{messages.review.amountSent}</dt><dd>{artifact.authorization.net_atomic}</dd></div><div><dt>{messages.review.paymentAsset}</dt><dd><code>{selectedRoute?.mint ?? messages.review.selectedByRoute}</code></dd></div><div><dt>{messages.review.paymentDetails}</dt><dd><code>{shortDigest(artifact.payment_scope_digest)}</code></dd></div></dl><small>{messages.review.verifiedInvoiceEvidence} <code>{shortDigest(artifact.document_attestation_digest)}</code> · {messages.review.paymentReference} {artifact.intent_id}</small><button type="button" onClick={signAndSubmit} disabled={isBusy || flow !== "ready" || expired}>{flow === "signing" ? messages.review.waitingWallet : flow === "submitting" ? messages.review.sendingPayment : messages.review.approve}</button>{settlement && <div className="settlement-status"><strong>{messages.review.paymentStatus}: {settlement.status}</strong><small>{settlement.signature ? messages.status.transactionReference(settlement.signature) : messages.review.noChainTransaction}</small><div className="status-actions"><button type="button" onClick={refreshStatus} disabled={isBusy}>{messages.review.refreshStatus}</button><button type="button" onClick={reconcile} disabled={isBusy || !settlement.signature}>{messages.review.checkRecord}</button></div></div>}</div>}
      {!artifact && <div className="empty-state"><strong>{messages.review.emptyTitle}</strong><p>{messages.review.emptyCopy}</p></div>}
      <ApplicabilityVerificationCard
        apiBase={apiBase}
        accessToken={accessToken}
        serviceOnline={serviceOnline}
        intentId={settlement?.intent_id ?? artifact?.intent_id ?? ""}
        streamKey={indexerStreamKey}
        locale={locale}
        refreshKey={settlement?.status ?? ""}
      />
      {error && <p className="inline-error" role="alert">{error}</p>}
      <p className="inline-warning">{messages.status.walletBoundary} {messages.status.legalBoundary}</p>
    </article>
  </section>;
}
