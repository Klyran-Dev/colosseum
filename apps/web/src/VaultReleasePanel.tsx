import { FormEvent, useEffect, useMemo, useState } from "react";
import { fetchWithTimeout, REQUEST_TIMEOUT_MS, responseError } from "./api";
import { parseSettlementRoutes, type SettlementRoute } from "./checkout";
import { DEFAULT_LOCALE, DEFAULT_TIME_ZONE, formatEpoch } from "./formatters";
import { createClientUuid } from "./clientIdentity";
import {
  VaultReleaseClaim,
  VaultReleaseStatus,
  parseVaultReleaseClaim,
  parseVaultReleaseTransitionResponse,
} from "./vaultRelease";

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

export default function VaultReleasePanel({ apiBase, serviceOnline, accessToken, locale = DEFAULT_LOCALE, timeZone = DEFAULT_TIME_ZONE }: VaultReleasePanelProps) {
  const initialWindow = useMemo(defaultWindow, []);
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
        if (!response.ok) throw new Error(responseError(response, payload, `Payment routes could not be loaded (${response.status})`));
        const loaded = parseSettlementRoutes(payload);
        setRoutes(loaded);
        setRouteId((current) => loaded.some((route) => route.id === current) ? current : loaded[0]?.id ?? "");
        if (loaded.length === 0) setRouteError("No active, verified payment routes are available for this account.");
      } catch (cause) {
        if (controller.signal.aborted) return;
        setRoutes([]);
        setRouteId("");
        setRouteError(cause instanceof Error ? cause.message : "Payment routes could not be loaded.");
      } finally {
        if (!controller.signal.aborted) setRouteLoading(false);
      }
    })();
    return () => controller.abort();
  }, [accessToken, apiBase, serviceOnline]);

  function requireToken(): string {
    const token = accessToken.trim();
    if (!token) throw new Error("Sign in with an account allowed to manage treasury releases. The credential is held in memory only.");
    return token;
  }

  function parseJsonObject(value: string, label: string): Record<string, unknown> {
    const parsed: unknown = JSON.parse(value);
    if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) throw new Error(`${label} must be a JSON object.`);
    return parsed as Record<string, unknown>;
  }

  async function loadClaimById(id: string, token: string): Promise<VaultReleaseClaim> {
    const response = await fetchWithTimeout(`${apiBase}/v1/vault/release-claims/${encodeURIComponent(id)}`, {
      headers: { authorization: `Bearer ${token}` },
    });
    const payload: unknown = await response.json().catch(() => null);
    if (!response.ok) throw new Error(responseError(response, payload, `Claim lookup failed (${response.status})`));
    return parseVaultReleaseClaim(payload);
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
        throw new Error("Reference digest must be a JSON array of exactly 32 byte values.");
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
      if (!response.ok) throw new Error(responseError(response, payload, `Claim proposal failed (${response.status})`));
      const created = parseVaultReleaseClaim(payload);
      setClaim(created);
      setClaimId(created.id);
      setNotice(`Release request ${created.id} was saved for approval. No funds moved and no spending authority was delegated.`);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Claim proposal failed");
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
      if (!response.ok) throw new Error(responseError(response, payload, `Claim lookup failed (${response.status})`));
      setClaim(parseVaultReleaseClaim(payload));
      setNotice("The saved release request was loaded. Confirmed ledger evidence remains the final source of truth.");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Claim lookup failed");
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
      const parsedEvidence = parseJsonObject(evidence, "Evidence");
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
      if (!response.ok) throw new Error(responseError(response, payload, `Claim transition failed (${response.status})`));
      const transitioned = parseVaultReleaseTransitionResponse(payload);
      const expectedClaimId = (claim?.id ?? claimId).trim();
      if (transitioned.claim_id !== expectedClaimId) throw new Error("The API returned a transition for a different claim.");
      try {
        const refreshed = await loadClaimById(transitioned.claim_id, token);
        setClaim(refreshed);
        setClaimId(refreshed.id);
        setNotice(`The request moved to ${transitioned.to_status.replaceAll("_", " ")}. The page was refreshed from the service.`);
      } catch (refreshCause) {
        setClaim(null);
        setNotice(`The request moved to ${transitioned.to_status.replaceAll("_", " ")}, but the refreshed request could not be loaded. Reconcile before taking another action.`);
        throw refreshCause;
      }
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Claim transition failed");
    } finally {
      setBusy(false);
    }
  }

  const transitionNeedsChain = transitionStatus === "onchain_created" || transitionStatus === "executed";
  const transitionNeedsError = transitionStatus === "unknown";
  const selectedRoute = routes.find((route) => route.id === routeId);

  return (
    <section className="vault-layout" aria-label="Governed vault release workspace">
      <article className="panel">
        <div className="panel-heading"><div><p className="eyebrow">Treasury / two-person approval</p><h2>Create a release request</h2></div><span className="status-pill">Two-person control</span></div>
        <p className="panel-copy">Create a release request for review. Approval, ledger recording, completion, cancellation, and uncertainty are tracked separately so every decision can be explained. The browser never signs or moves funds.</p>
        <form onSubmit={createClaim} className="form-grid">
          <label className="full-field"><span>Sign-in status</span><textarea value={accessToken ? "Account ready" : ""} readOnly aria-describedby="vault-token-help" rows={2} /><small id="vault-token-help">Your account must be allowed to create treasury release requests. Sign-in data is never saved by this page.</small></label>
          <label><span>Request reference</span><input value={idempotencyKey} onChange={(event) => setIdempotencyKey(event.target.value)} aria-describedby="release-reference-help" required /><small id="release-reference-help">A unique reference prevents duplicate requests when you retry.</small></label>
          <label><span>Release reference number</span><input aria-label="Release reference number" inputMode="numeric" value={nonce} onChange={(event) => setNonce(event.target.value)} aria-describedby="release-number-help" required /><small id="release-number-help">This binds the request to one ledger record.</small></label>
          <label><span>Verified payment route</span><select aria-label="Verified payment route" value={routeId} onChange={(event) => setRouteId(event.target.value)} disabled={routeLoading || routes.length === 0} required><option value="">{routeLoading ? "Loading verified routes…" : "Select a verified route"}</option>{routes.map((route) => <option key={route.id} value={route.id}>{route.route_key} · version {route.version}</option>)}</select><small>Mint, token program, and tax holding account are resolved from this server-approved route.</small></label>
          <label><span>Recipient account</span><input value={destination} onChange={(event) => setDestination(event.target.value)} placeholder="Paste the recipient account" required /></label>
          <label><span>Amount</span><input aria-label="Amount" inputMode="numeric" value={amountAtomic} onChange={(event) => setAmountAtomic(event.target.value)} aria-describedby="release-amount-help" required /><small id="release-amount-help">Enter the exact amount in the asset’s smallest unit. The service checks precision before saving.</small></label>
          <label><span>Earliest release time</span><input inputMode="numeric" value={executeAfter} onChange={(event) => setExecuteAfter(event.target.value)} aria-describedby="release-start-help" required /><small id="release-start-help">Use the UTC timestamp required by the release policy.</small></label>
          <label><span>Release expiry</span><input inputMode="numeric" value={expiresAt} onChange={(event) => setExpiresAt(event.target.value)} aria-describedby="release-expiry-help" required /><small id="release-expiry-help">The request cannot be executed after this time.</small></label>
          <label className="full-field"><span>Evidence reference</span><textarea id="vault-release-evidence-reference" aria-label="Evidence reference" value={referenceDigest} onChange={(event) => setReferenceDigest(event.target.value)} aria-describedby="release-evidence-help" rows={3} required /><small id="release-evidence-help">Paste the exact evidence reference supplied by the approved workflow. A request cannot be created without it.</small></label>
          <button type="submit" disabled={busy || !serviceOnline || !accessToken.trim() || !routeId}>{busy ? "Saving…" : "Create release request"}</button>
        </form>
        {routeError && <p className="inline-warning" role="alert">{routeError} No release request can be created until a verified route is available.</p>}
        {selectedRoute && <div className="vault-route-summary" aria-label="Verified route details"><strong>Route evidence</strong><span>{selectedRoute.mint}</span><span>{selectedRoute.token_program}</span><span>Tax holding account: {selectedRoute.tax_vault}</span></div>}
        {!serviceOnline && <p className="inline-warning" role="alert">The API is unavailable. No client-side fallback or optimistic claim is created.</p>}
      </article>

      <article className="panel">
        <div className="panel-heading"><div><p className="eyebrow">Lifecycle and evidence</p><h2>Review or transition a claim</h2></div>{claim && <span className="status-pill status-pill-warning">{claim.status.replaceAll("_", " ")}</span>}</div>
        <p className="panel-copy">Each next step requires the right approval. Ledger-created and completed states need proof from the network; an uncertain state must include a clear reason.</p>
        <form onSubmit={loadClaim} className="form-grid">
          <label className="full-field"><span>Release request reference</span><input value={claimId} onChange={(event) => setClaimId(event.target.value)} placeholder="Paste a saved release request reference" required /></label>
          <button type="submit" disabled={busy || !serviceOnline || !accessToken.trim()}>Load release request</button>
        </form>
        {claim && <div className="vault-claim-card"><dl className="summary-list"><div><dt>Status</dt><dd>{claim.status.replaceAll("_", " ")}</dd></div><div><dt>Amount</dt><dd>{claim.amount_atomic}</dd></div><div><dt>Release reference number</dt><dd>{claim.nonce}</dd></div><div><dt>Release window</dt><dd>{formatEpoch(claim.execute_after_unix, locale, timeZone)} → {formatEpoch(claim.expires_at_unix, locale, timeZone)}</dd></div><div><dt>Request evidence</dt><dd><code>{digestPreview(claim.request_digest)}</code></dd></div><div><dt>Supporting evidence</dt><dd><code>{digestPreview(claim.reference_digest)}</code></dd></div><div><dt>Created by</dt><dd><code>{claim.proposer_subject}</code></dd></div><div><dt>Approved by</dt><dd><code>{claim.approver_subject ?? "Not approved"}</code></dd></div></dl></div>}
        <form onSubmit={transitionClaim} className="vault-transition-form">
          <label><span>Next step</span><select value={transitionStatus} onChange={(event) => setTransitionStatus(event.target.value as ReleaseStatus)}><option value="approved">Approved</option><option value="onchain_created">Ledger record created</option><option value="executed">Completed</option><option value="cancelled">Cancelled</option><option value="unknown">Needs investigation</option></select></label>
          {transitionNeedsChain && <label><span>On-chain release account {transitionStatus === "onchain_created" ? "(required)" : "(if available)"}</span><input value={claimPda} onChange={(event) => setClaimPda(event.target.value)} required={transitionStatus === "onchain_created"} /></label>}
          {transitionNeedsChain && <label><span>Ledger transaction reference (required)</span><input value={transactionSignature} onChange={(event) => setTransactionSignature(event.target.value)} required /></label>}
          {transitionNeedsChain && <label><span>Confirmed ledger position (required)</span><input inputMode="numeric" value={slot} onChange={(event) => setSlot(event.target.value)} required /></label>}
          {transitionNeedsError && <label><span>Why it stopped (required)</span><input value={lastErrorCode} onChange={(event) => setLastErrorCode(event.target.value)} required /></label>}
          <label className="full-field"><span>Decision evidence</span><textarea value={evidence} onChange={(event) => setEvidence(event.target.value)} aria-describedby="release-transition-evidence-help" rows={3} required /><small id="release-transition-evidence-help">Advanced evidence for the audit record. Use the format supplied by your operations workflow.</small></label>
          <button type="submit" disabled={busy || !serviceOnline || !accessToken.trim() || !(claim?.id ?? claimId).trim()}>Record next step</button>
        </form>
        {notice && <p className="inline-success" role="status">{notice}</p>}
        {error && <p className="inline-error" role="alert">{error}</p>}
        <p className="vault-authority-note"><strong>What this page can do:</strong> record requests and evidence. A separately approved operator must prepare, sign, submit, and reconcile the ledger transaction. Confirmed ledger state—not this page—decides whether funds moved.</p>
      </article>
    </section>
  );
}
