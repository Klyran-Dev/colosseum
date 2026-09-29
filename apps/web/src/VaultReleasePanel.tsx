import { FormEvent, useMemo, useState } from "react";
import { fetchWithTimeout, REQUEST_TIMEOUT_MS, responseError } from "./api";
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

const DEFAULT_REFERENCE_DIGEST = JSON.stringify(Array.from({ length: 32 }, (_, index) => index + 1));

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
  const [mint, setMint] = useState("");
  const [tokenProgram, setTokenProgram] = useState("");
  const [taxVault, setTaxVault] = useState("");
  const [destination, setDestination] = useState("");
  const [nonce, setNonce] = useState("1");
  const [amountAtomic, setAmountAtomic] = useState("1");
  const [referenceDigest, setReferenceDigest] = useState(DEFAULT_REFERENCE_DIGEST);
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

  function requireToken(): string {
    const token = accessToken.trim();
    if (!token) throw new Error("An OIDC access token with the required vault role and scope is required. It is held in memory only.");
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
          mint: mint.trim(),
          token_program: tokenProgram.trim(),
          tax_vault: taxVault.trim(),
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
      setNotice(`Claim ${created.id} is durably proposed. No funds moved and no chain authority was delegated.`);
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
      setNotice("Claim state loaded from the tenant-scoped API projection. Finalized chain evidence remains authoritative.");
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
        setNotice(`Transition recorded as ${transitioned.to_status}. The displayed claim was refreshed from the tenant-scoped API.`);
      } catch (refreshCause) {
        setClaim(null);
        setNotice(`Transition recorded as ${transitioned.to_status}, but the refreshed claim could not be loaded. Reconcile before taking another lifecycle action.`);
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

  return (
    <section className="vault-layout" aria-label="Governed vault release workspace">
      <article className="panel">
        <div className="panel-heading"><div><p className="eyebrow">Governed treasury operation</p><h2>Propose a vault release claim</h2></div><span className="status-pill">Two-person control</span></div>
        <p className="panel-copy">Create an immutable, tenant-scoped release intent. Proposal, approval, transaction recording, execution, cancellation, and uncertainty are separate auditable states. The browser never signs or moves funds.</p>
        <form onSubmit={createClaim} className="form-grid">
          <label className="full-field"><span>OIDC access token (memory-only)</span><textarea value={accessToken ? "Token loaded in memory" : ""} readOnly aria-describedby="vault-token-help" rows={2} /><small id="vault-token-help">Use a proposer token with <code>tax:vault:propose</code> and <code>vault_release_proposer</code>. The token value is owned by the parent checkout workspace and is never persisted.</small></label>
          <label><span>Idempotency key</span><input value={idempotencyKey} onChange={(event) => setIdempotencyKey(event.target.value)} required /></label>
          <label><span>Claim nonce (u64)</span><input inputMode="numeric" value={nonce} onChange={(event) => setNonce(event.target.value)} required /></label>
          <label><span>Mint public key</span><input value={mint} onChange={(event) => setMint(event.target.value)} placeholder="Base58 mint" required /></label>
          <label><span>Token program public key</span><input value={tokenProgram} onChange={(event) => setTokenProgram(event.target.value)} placeholder="Base58 program" required /></label>
          <label><span>Tax vault public key</span><input value={taxVault} onChange={(event) => setTaxVault(event.target.value)} placeholder="Base58 vault" required /></label>
          <label><span>Destination public key</span><input value={destination} onChange={(event) => setDestination(event.target.value)} placeholder="Base58 destination" required /></label>
          <label><span>Amount (atomic units)</span><input inputMode="numeric" value={amountAtomic} onChange={(event) => setAmountAtomic(event.target.value)} required /></label>
          <label><span>Execute after (Unix seconds)</span><input inputMode="numeric" value={executeAfter} onChange={(event) => setExecuteAfter(event.target.value)} required /></label>
          <label><span>Expires at (Unix seconds)</span><input inputMode="numeric" value={expiresAt} onChange={(event) => setExpiresAt(event.target.value)} required /></label>
          <label className="full-field"><span>Reference digest (JSON array of 32 bytes)</span><textarea value={referenceDigest} onChange={(event) => setReferenceDigest(event.target.value)} rows={3} required /></label>
          <button type="submit" disabled={busy || !serviceOnline || !accessToken.trim()}>{busy ? "Submitting…" : "Persist release proposal"}</button>
        </form>
        {!serviceOnline && <p className="inline-warning" role="alert">The API is unavailable. No client-side fallback or optimistic claim is created.</p>}
      </article>

      <article className="panel">
        <div className="panel-heading"><div><p className="eyebrow">Lifecycle and evidence</p><h2>Review or transition a claim</h2></div>{claim && <span className="status-pill status-pill-warning">{claim.status.replaceAll("_", " ")}</span>}</div>
        <p className="panel-copy">Every transition requires the exact role for that action. Chain-created and executed states require transaction evidence; an unknown state requires an explicit bounded error code.</p>
        <form onSubmit={loadClaim} className="form-grid">
          <label className="full-field"><span>Claim ID (UUIDv7)</span><input value={claimId} onChange={(event) => setClaimId(event.target.value)} placeholder="Load a previously proposed claim" required /></label>
          <button type="submit" disabled={busy || !serviceOnline || !accessToken.trim()}>Load tenant-scoped claim</button>
        </form>
        {claim && <div className="vault-claim-card"><dl className="summary-list"><div><dt>State</dt><dd>{claim.status.replaceAll("_", " ")}</dd></div><div><dt>Amount</dt><dd>{claim.amount_atomic} atomic units</dd></div><div><dt>Nonce</dt><dd>{claim.nonce}</dd></div><div><dt>Window</dt><dd>{formatEpoch(claim.execute_after_unix, locale, timeZone)} → {formatEpoch(claim.expires_at_unix, locale, timeZone)}</dd></div><div><dt>Request digest</dt><dd><code>{digestPreview(claim.request_digest)}</code></dd></div><div><dt>Reference digest</dt><dd><code>{digestPreview(claim.reference_digest)}</code></dd></div><div><dt>Proposer</dt><dd><code>{claim.proposer_subject}</code></dd></div><div><dt>Approver</dt><dd><code>{claim.approver_subject ?? "Not approved"}</code></dd></div></dl></div>}
        <form onSubmit={transitionClaim} className="vault-transition-form">
          <label><span>Next lifecycle state</span><select value={transitionStatus} onChange={(event) => setTransitionStatus(event.target.value as ReleaseStatus)}><option value="approved">Approved</option><option value="onchain_created">On-chain claim created</option><option value="executed">Executed</option><option value="cancelled">Cancelled</option><option value="unknown">Unknown / reconciliation required</option></select></label>
          {transitionNeedsChain && <label><span>Claim PDA {transitionStatus === "onchain_created" ? "(required)" : "(if available)"}</span><input value={claimPda} onChange={(event) => setClaimPda(event.target.value)} required={transitionStatus === "onchain_created"} /></label>}
          {transitionNeedsChain && <label><span>Transaction signature (required)</span><input value={transactionSignature} onChange={(event) => setTransactionSignature(event.target.value)} required /></label>}
          {transitionNeedsChain && <label><span>Observed slot (required)</span><input inputMode="numeric" value={slot} onChange={(event) => setSlot(event.target.value)} required /></label>}
          {transitionNeedsError && <label><span>Last error code (required)</span><input value={lastErrorCode} onChange={(event) => setLastErrorCode(event.target.value)} required /></label>}
          <label className="full-field"><span>Transition evidence (JSON object)</span><textarea value={evidence} onChange={(event) => setEvidence(event.target.value)} rows={3} required /></label>
          <button type="submit" disabled={busy || !serviceOnline || !accessToken.trim() || !(claim?.id ?? claimId).trim()}>Record governed transition</button>
        </form>
        {notice && <p className="inline-success" role="status">{notice}</p>}
        {error && <p className="inline-error" role="alert">{error}</p>}
        <p className="vault-authority-note"><strong>Authority boundary:</strong> this workspace records commands and evidence only. A separate approved operator must compile, sign, submit, and reconcile the Solana transaction. Finalized chain state, not this database projection, decides whether funds moved.</p>
      </article>
    </section>
  );
}
