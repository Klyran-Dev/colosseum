import { FormEvent, useEffect, useMemo, useState } from "react";
import { fetchWithTimeout, operatorResponseError, REQUEST_TIMEOUT_MS } from "./api";
import { parseSettlementRoutes, type SettlementRoute } from "./checkout";
import { DEFAULT_LOCALE, DEFAULT_TIME_ZONE, formatEpoch } from "./formatters";
import { createClientUuid } from "./clientIdentity";
import { vaultReleaseMessagesFor } from "./vaultReleaseMessages";
import {
  VaultReleaseClaim,
  VaultReleaseStatus,
  parseVaultReleaseClaim,
  parseVaultReleaseTransitionResponse,
} from "./vaultRelease";
import { operatorErrorMessage, operatorFacingError } from "./operatorError";

type VaultReleasePanelProps = {
  apiBase: string;
  serviceOnline: boolean;
  accessToken: string;
  locale?: string;
  timeZone?: string;
};

type ReleaseStatus = Exclude<VaultReleaseStatus, "proposed">;

function defaultWindow(): { executeAfter: string; expiresAt: string } {
  const now = Math.floor(Date.now() / 1000);
  return { executeAfter: String(now + 60), expiresAt: String(now + 3_600) };
}

function digestPreview(value: number[]): string {
  return value.map((byte) => byte.toString(16).padStart(2, "0")).join("").slice(0, 16) + "…";
}

function parseOperatorClaim(payload: unknown, invalidMessage: string): VaultReleaseClaim {
  try {
    return parseVaultReleaseClaim(payload);
  } catch {
    throw operatorFacingError(invalidMessage);
  }
}

function parseOperatorTransition(payload: unknown, invalidMessage: string): ReturnType<typeof parseVaultReleaseTransitionResponse> {
  try {
    return parseVaultReleaseTransitionResponse(payload);
  } catch {
    throw operatorFacingError(invalidMessage);
  }
}

export default function VaultReleasePanel({ apiBase, serviceOnline, accessToken, locale = DEFAULT_LOCALE, timeZone = DEFAULT_TIME_ZONE }: VaultReleasePanelProps) {
  const initialWindow = useMemo(defaultWindow, []);
  const messages = vaultReleaseMessagesFor(locale);
  const [idempotencyKey, setIdempotencyKey] = useState<string>(() => createClientUuid());
  const [routes, setRoutes] = useState<SettlementRoute[]>([]);
  const [routeId, setRouteId] = useState("");
  const [routeLoading, setRouteLoading] = useState(false);
  const [routeError, setRouteError] = useState<string | null>(null);
  const [destination, setDestination] = useState("");
  const [nonce, setNonce] = useState("");
  const [amountAtomic, setAmountAtomic] = useState("");
  const [referenceDigest, setReferenceDigest] = useState("");
  const [executeAfter, setExecuteAfter] = useState(initialWindow.executeAfter);
  const [expiresAt, setExpiresAt] = useState(initialWindow.expiresAt);
  const [claimId, setClaimId] = useState("");
  const [claim, setClaim] = useState<VaultReleaseClaim | null>(null);
  const [transitionStatus, setTransitionStatus] = useState<ReleaseStatus>("approved");
  const [claimPda, setClaimPda] = useState("");
  const [transactionSignature, setTransactionSignature] = useState("");
  const [slot, setSlot] = useState("");
  const [lastErrorCode, setLastErrorCode] = useState("");
  const [evidence, setEvidence] = useState('{"incident_reference":""}');
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!serviceOnline || !accessToken.trim()) {
      setRoutes([]);
      setRouteId("");
      setRouteError(null);
      setRouteLoading(false);
      return;
    }
    const controller = new AbortController();
    setRouteLoading(true);
    setRouteError(null);
    void (async () => {
      try {
        const response = await fetchWithTimeout(`${apiBase}/v1/vault/release-routes`, {
          headers: { authorization: `Bearer ${accessToken.trim()}` },
          signal: controller.signal,
        }, REQUEST_TIMEOUT_MS.default);
        const payload: unknown = await response.json().catch(() => null);
        if (!response.ok) throw operatorResponseError(response, payload, messages.errors.routeRequestFailed(response.status));
        const loaded = parseSettlementRoutes(payload);
        setRoutes(loaded);
        setRouteId((current) => loaded.some((route) => route.id === current) ? current : loaded[0]?.id ?? "");
        if (loaded.length === 0) setRouteError(messages.errors.noVerifiedRoutes);
      } catch (cause) {
        if (controller.signal.aborted) return;
        setRoutes([]);
        setRouteId("");
        setRouteError(operatorErrorMessage(cause, messages.errors.routeLoadFailed));
      } finally {
        if (!controller.signal.aborted) setRouteLoading(false);
      }
    })();
    return () => controller.abort();
  }, [accessToken, apiBase, serviceOnline]);

  function requireToken(): string {
    const token = accessToken.trim();
    if (!token) throw operatorFacingError(messages.errors.authorizationRequired);
    return token;
  }

  function parseJsonObject(value: string): Record<string, unknown> {
    const parsed: unknown = JSON.parse(value);
    if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) throw operatorFacingError(messages.errors.evidenceObjectRequired);
    return parsed as Record<string, unknown>;
  }

  async function loadClaimById(id: string, token: string): Promise<VaultReleaseClaim> {
    const response = await fetchWithTimeout(`${apiBase}/v1/vault/release-claims/${encodeURIComponent(id)}`, {
      headers: { authorization: `Bearer ${token}` },
    });
    const payload: unknown = await response.json().catch(() => null);
    if (!response.ok) throw operatorResponseError(response, payload, messages.errors.claimLookupRequestFailed(response.status));
    return parseOperatorClaim(payload, messages.errors.invalidClaim);
  }

  async function createClaim(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);
    setNotice(null);
    setBusy(true);
    try {
      const token = requireToken();
      const parsedReference = JSON.parse(referenceDigest) as unknown;
      if (!Array.isArray(parsedReference) || parsedReference.length !== 32 || parsedReference.some((byte) => !Number.isInteger(byte) || byte < 0 || byte > 255)) {
        throw operatorFacingError(messages.errors.referenceDigestInvalid);
      }
      const response = await fetchWithTimeout(`${apiBase}/v1/vault/release-claims`, {
        method: "POST",
        headers: { "content-type": "application/json", authorization: `Bearer ${token}` },
        body: JSON.stringify({
          idempotency_key: idempotencyKey.trim(),
          route_id: routeId,
          destination: destination.trim(),
          nonce: nonce.trim(),
          amount_atomic: amountAtomic.trim(),
          reference_digest: parsedReference,
          execute_after_unix: Number(executeAfter),
          expires_at_unix: Number(expiresAt),
        }),
      });
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw operatorResponseError(response, payload, messages.errors.claimProposalRequestFailed(response.status));
      const created = parseOperatorClaim(payload, messages.errors.invalidClaim);
      setClaim(created);
      setClaimId(created.id);
      setNotice(`${messages.errors.claimCreated} ${created.id}`);
    } catch (cause) {
      setError(operatorErrorMessage(cause, messages.errors.claimProposalFailed));
    } finally {
      setBusy(false);
    }
  }

  async function loadClaim(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);
    setNotice(null);
    setBusy(true);
    try {
      const token = requireToken();
      const response = await fetchWithTimeout(`${apiBase}/v1/vault/release-claims/${encodeURIComponent(claimId.trim())}`, {
        headers: { authorization: `Bearer ${token}` },
      });
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw operatorResponseError(response, payload, messages.errors.claimLookupRequestFailed(response.status));
      setClaim(parseOperatorClaim(payload, messages.errors.invalidClaim));
      setNotice(messages.errors.claimLoaded);
    } catch (cause) {
      setError(operatorErrorMessage(cause, messages.errors.claimLookupFailed));
    } finally {
      setBusy(false);
    }
  }

  async function transitionClaim(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);
    setNotice(null);
    setBusy(true);
    try {
      const token = requireToken();
      const parsedEvidence = parseJsonObject(evidence);
      const response = await fetchWithTimeout(`${apiBase}/v1/vault/release-claims/${encodeURIComponent((claim?.id ?? claimId).trim())}/transition`, {
        method: "POST",
        headers: { "content-type": "application/json", authorization: `Bearer ${token}` },
        body: JSON.stringify({
          to_status: transitionStatus,
          evidence: parsedEvidence,
          claim_pda: claimPda.trim() || null,
          transaction_signature: transactionSignature.trim() || null,
          slot: slot.trim() ? Number(slot) : null,
          last_error_code: lastErrorCode.trim() || null,
        }),
      });
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw operatorResponseError(response, payload, messages.errors.claimTransitionRequestFailed(response.status));
      const transitioned = parseOperatorTransition(payload, messages.errors.invalidTransition);
      const expectedClaimId = (claim?.id ?? claimId).trim();
      if (transitioned.claim_id !== expectedClaimId) throw operatorFacingError(messages.errors.claimMismatch);
      try {
        const refreshed = await loadClaimById(transitioned.claim_id, token);
        setClaim(refreshed);
        setClaimId(refreshed.id);
        setNotice(messages.errors.transitionSaved);
      } catch (refreshCause) {
        setClaim(null);
        setNotice(messages.errors.transitionSavedButRefreshFailed);
        throw refreshCause;
      }
    } catch (cause) {
      setError(operatorErrorMessage(cause, messages.errors.claimTransitionFailed));
    } finally {
      setBusy(false);
    }
  }

  const transitionNeedsChain = transitionStatus === "onchain_created" || transitionStatus === "executed";
  const transitionNeedsError = transitionStatus === "unknown";
  const selectedRoute = routes.find((route) => route.id === routeId);
  return (
    <section className="vault-layout" aria-label={messages.proposal.ariaLabel}>
      <article className="panel">
        <div className="panel-heading"><div><p className="eyebrow">{messages.proposal.eyebrow}</p><h2>{messages.proposal.title}</h2></div><span className="status-pill">{messages.proposal.control}</span></div>
        <p className="panel-copy">{messages.proposal.description}</p>
        <form onSubmit={createClaim} className="form-grid">
          <label className="full-field"><span>{messages.proposal.signInStatus}</span><textarea value={accessToken ? messages.proposal.accountReady : ""} readOnly aria-describedby="vault-token-help" rows={2} /><small id="vault-token-help">{messages.proposal.accountHelp}</small></label>
          <label><span>{messages.proposal.requestReference}</span><input value={idempotencyKey} onChange={(event) => setIdempotencyKey(event.target.value)} aria-describedby="release-reference-help" required /><small id="release-reference-help">{messages.proposal.requestHelp}</small></label>
          <label><span>{messages.proposal.releaseNumber}</span><input aria-label={messages.proposal.releaseNumber} inputMode="numeric" value={nonce} onChange={(event) => setNonce(event.target.value)} aria-describedby="release-number-help" required /><small id="release-number-help">{messages.proposal.releaseNumberHelp}</small></label>
          <label><span>{messages.proposal.verifiedRoute}</span><select aria-label={messages.proposal.verifiedRoute} value={routeId} onChange={(event) => setRouteId(event.target.value)} disabled={routeLoading || routes.length === 0} required><option value="">{routeLoading ? messages.proposal.loadingRoutes : messages.proposal.selectRoute}</option>{routes.map((route) => <option key={route.id} value={route.id}>{route.route_key} · version {route.version}</option>)}</select><small>{messages.proposal.routeHelp}</small></label>
          <label><span>{messages.proposal.recipient}</span><input aria-label={messages.proposal.recipient} value={destination} onChange={(event) => setDestination(event.target.value)} placeholder={messages.proposal.recipientPlaceholder} required /></label>
          <label><span>{messages.proposal.amount}</span><input aria-label={messages.proposal.amount} inputMode="numeric" value={amountAtomic} onChange={(event) => setAmountAtomic(event.target.value)} aria-describedby="release-amount-help" required /><small id="release-amount-help">{messages.proposal.amountHelp}</small></label>
          <label><span>{messages.proposal.earliest}</span><input aria-label={messages.proposal.earliest} inputMode="numeric" value={executeAfter} onChange={(event) => setExecuteAfter(event.target.value)} aria-describedby="release-start-help" required /><small id="release-start-help">{messages.proposal.earliestHelp}</small></label>
          <label><span>{messages.proposal.expiry}</span><input aria-label={messages.proposal.expiry} inputMode="numeric" value={expiresAt} onChange={(event) => setExpiresAt(event.target.value)} aria-describedby="release-expiry-help" required /><small id="release-expiry-help">{messages.proposal.expiryHelp}</small></label>
          <label className="full-field"><span>{messages.proposal.evidenceReference}</span><textarea id="vault-release-evidence-reference" aria-label={messages.proposal.evidenceReference} value={referenceDigest} onChange={(event) => setReferenceDigest(event.target.value)} aria-describedby="release-evidence-help" rows={3} required /><small id="release-evidence-help">{messages.proposal.evidenceHelp}</small></label>
          <button type="submit" disabled={busy || !serviceOnline || !accessToken.trim() || !routeId}>{busy ? messages.proposal.saving : messages.proposal.create}</button>
        </form>
        {routeError && <p className="inline-warning" role="alert">{routeError} {messages.proposal.routeErrorSuffix}</p>}
        {selectedRoute && <div className="vault-route-summary" aria-label={messages.proposal.routeDetails}><strong>{messages.proposal.routeEvidence}</strong><span>{selectedRoute.mint}</span><span>{selectedRoute.token_program}</span><span>{messages.proposal.taxHoldingAccount} {selectedRoute.tax_vault}</span></div>}
        {!serviceOnline && <p className="inline-warning" role="alert">{messages.proposal.offline}</p>}
      </article>

      <article className="panel" aria-label={messages.lifecycle.ariaLabel}>
        <div className="panel-heading"><div><p className="eyebrow">{messages.lifecycle.eyebrow}</p><h2>{messages.lifecycle.title}</h2></div>{claim && <span className="status-pill status-pill-warning">{claim.status.replaceAll("_", " ")}</span>}</div>
        <p className="panel-copy">{messages.lifecycle.description}</p>
        <form onSubmit={loadClaim} className="form-grid">
          <label className="full-field"><span>{messages.lifecycle.requestReference}</span><input aria-label={messages.lifecycle.requestReference} value={claimId} onChange={(event) => setClaimId(event.target.value)} placeholder={messages.lifecycle.requestPlaceholder} required /></label>
          <button type="submit" disabled={busy || !serviceOnline || !accessToken.trim()}>{messages.lifecycle.load}</button>
        </form>
        {claim && <div className="vault-claim-card"><dl className="summary-list"><div><dt>{messages.lifecycle.status}</dt><dd>{claim.status.replaceAll("_", " ")}</dd></div><div><dt>{messages.lifecycle.amount}</dt><dd>{claim.amount_atomic}</dd></div><div><dt>{messages.lifecycle.releaseNumber}</dt><dd>{claim.nonce}</dd></div><div><dt>{messages.lifecycle.releaseWindow}</dt><dd>{formatEpoch(claim.execute_after_unix, locale, timeZone)} → {formatEpoch(claim.expires_at_unix, locale, timeZone)}</dd></div><div><dt>{messages.lifecycle.requestEvidence}</dt><dd><code>{digestPreview(claim.request_digest)}</code></dd></div><div><dt>{messages.lifecycle.supportingEvidence}</dt><dd><code>{digestPreview(claim.reference_digest)}</code></dd></div><div><dt>{messages.lifecycle.createdBy}</dt><dd><code>{claim.proposer_subject}</code></dd></div><div><dt>{messages.lifecycle.approvedBy}</dt><dd><code>{claim.approver_subject ?? messages.lifecycle.notApproved}</code></dd></div></dl></div>}
        <form onSubmit={transitionClaim} className="vault-transition-form">
          <label><span>{messages.lifecycle.nextStep}</span><select aria-label={messages.lifecycle.nextStep} value={transitionStatus} onChange={(event) => setTransitionStatus(event.target.value as ReleaseStatus)}><option value="approved">{messages.lifecycle.approved}</option><option value="onchain_created">{messages.lifecycle.ledgerCreated}</option><option value="executed">{messages.lifecycle.completed}</option><option value="cancelled">{messages.lifecycle.cancelled}</option><option value="unknown">{messages.lifecycle.needsInvestigation}</option></select></label>
          {transitionNeedsChain && <label><span>{messages.lifecycle.onChainAccount} {transitionStatus === "onchain_created" ? `(${messages.lifecycle.required})` : `(${messages.lifecycle.ifAvailable})`}</span><input aria-label={messages.lifecycle.onChainAccount} value={claimPda} onChange={(event) => setClaimPda(event.target.value)} required={transitionStatus === "onchain_created"} /></label>}
          {transitionNeedsChain && <label><span>{messages.lifecycle.ledgerTransaction}</span><input aria-label={messages.lifecycle.ledgerTransaction} value={transactionSignature} onChange={(event) => setTransactionSignature(event.target.value)} required /></label>}
          {transitionNeedsChain && <label><span>{messages.lifecycle.confirmedPosition}</span><input aria-label={messages.lifecycle.confirmedPosition} inputMode="numeric" value={slot} onChange={(event) => setSlot(event.target.value)} required /></label>}
          {transitionNeedsError && <label><span>{messages.lifecycle.whyStopped}</span><input aria-label={messages.lifecycle.whyStopped} value={lastErrorCode} onChange={(event) => setLastErrorCode(event.target.value)} required /></label>}
          <label className="full-field"><span>{messages.lifecycle.decisionEvidence}</span><textarea aria-label={messages.lifecycle.decisionEvidence} value={evidence} onChange={(event) => setEvidence(event.target.value)} aria-describedby="release-transition-evidence-help" rows={3} required /><small id="release-transition-evidence-help">{messages.lifecycle.decisionEvidenceHelp}</small></label>
          <button type="submit" disabled={busy || !serviceOnline || !accessToken.trim() || !(claim?.id ?? claimId).trim()}>{messages.lifecycle.record}</button>
        </form>
        {notice && <p className="inline-success" role="status">{notice}</p>}
        {error && <p className="inline-error" role="alert">{error}</p>}
        <p className="vault-authority-note"><strong>{messages.authority.title}</strong> {messages.authority.copy}</p>
      </article>
    </section>
  );
}
