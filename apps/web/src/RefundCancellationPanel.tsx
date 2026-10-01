import { FormEvent, useState } from "react";
import { PublicKey } from "@solana/web3.js";
import { fetchWithTimeout, REQUEST_TIMEOUT_MS, responseError } from "./api";
import { createClientUuid } from "./clientIdentity";
import {
  parseRefundCancellationResponse,
  parseRefundCancellationSubmissionResponse,
  refundCancellationHasSignature,
  signRefundCancellation,
  type RefundCancellationResponse,
} from "./refundCancellation";
import WalletPicker from "./WalletPicker";

type RefundCancellationPanelProps = {
  apiBase: string;
  serviceOnline: boolean;
  accessToken: string;
};

function digestPreview(bytes: number[]): string {
  return `${bytes.slice(0, 8).map((byte) => byte.toString(16).padStart(2, "0")).join("")}…`;
}

function statusLabel(status: string): string {
  return status.replaceAll("_", " ");
}

export default function RefundCancellationPanel({ apiBase, serviceOnline, accessToken }: RefundCancellationPanelProps) {
  const [claimIntentId, setClaimIntentId] = useState("");
  const [lookupId, setLookupId] = useState("");
  const [idempotencyKey, setIdempotencyKey] = useState(() => createClientUuid());
  const [cancellation, setCancellation] = useState<RefundCancellationResponse | null>(null);
  const [signedBytes, setSignedBytes] = useState<string | null>(null);
  const [recoveryBytes, setRecoveryBytes] = useState("");
  const [walletAddress, setWalletAddress] = useState<PublicKey | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  function requireToken(): string {
    const token = accessToken.trim();
    if (!token) throw new Error("An OIDC access token with the governed cancellation scope is required. It is held in memory only.");
    return token;
  }

  function clearMessages() {
    setError(null);
    setNotice(null);
  }

  async function prepare(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setBusy(true);
    clearMessages();
    try {
      const token = requireToken();
      if (!claimIntentId.trim()) throw new Error("A finalized refund-claim intent ID is required; the server loads the claim topology and authority evidence.");
      const response = await fetchWithTimeout(`${apiBase.replace(/\/$/, "")}/v1/settlement/refund-claim-cancellations`, {
        method: "POST",
        headers: { "content-type": "application/json", authorization: `Bearer ${token}` },
        body: JSON.stringify({ idempotency_key: idempotencyKey.trim(), claim_intent_id: claimIntentId.trim(), confirm: true }),
      });
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw new Error(responseError(response, payload, `Refund cancellation preparation failed (${response.status})`));
      const parsed = parseRefundCancellationResponse(payload);
      setCancellation(parsed);
      setLookupId(parsed.intent_id);
      setSignedBytes(null);
      setRecoveryBytes("");
      setWalletAddress(null);
      setNotice("The server prepared a governance-only cancellation artifact. It carries no amount or token-account fields and cannot move funds; review the exact account topology before signing.");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Refund cancellation preparation failed.");
    } finally {
      setBusy(false);
    }
  }

  async function load(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setBusy(true);
    clearMessages();
    try {
      const token = requireToken();
      if (!lookupId.trim()) throw new Error("A refund cancellation intent ID is required.");
      const response = await fetchWithTimeout(`${apiBase.replace(/\/$/, "")}/v1/settlement/refund-claim-cancellations/${lookupId.trim()}`, {
        headers: { authorization: `Bearer ${token}` },
      });
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw new Error(responseError(response, payload, `Refund cancellation recovery failed (${response.status})`));
      setCancellation(parseRefundCancellationResponse(payload));
      setSignedBytes(null);
      setRecoveryBytes("");
      setWalletAddress(null);
      setNotice("The durable cancellation intent was loaded. Signed bytes are not returned by the API; paste the unchanged wallet artifact to resume submission.");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Refund cancellation recovery failed.");
    } finally {
      setBusy(false);
    }
  }

  async function sign() {
    setBusy(true);
    clearMessages();
    try {
      if (!cancellation) throw new Error("Prepare or recover a refund cancellation before signing.");
      if (cancellation.status !== "prepared_unsubmitted" && cancellation.status !== "signed") throw new Error("This cancellation is not in a wallet-signable state. Reconcile its durable state before taking another action.");
      const expectedAuthority = new PublicKey(cancellation.artifact.governance_authority);
      const serialized = signedBytes ?? (recoveryBytes.trim() || null);
      const next = await signRefundCancellation(cancellation.artifact.instruction, expectedAuthority, import.meta.env.VITE_SOLANA_RPC_URL ?? "", serialized);
      setSignedBytes(next);
      setRecoveryBytes("");
      setWalletAddress(expectedAuthority);
      setNotice(refundCancellationHasSignature(next)
        ? "The governance wallet signature is present in local memory. Submit these unchanged bytes; a signature is not finalized cancellation evidence."
        : "The wallet did not return a signed transaction artifact.");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Refund cancellation wallet signing failed.");
    } finally {
      setBusy(false);
    }
  }

  async function submit() {
    setBusy(true);
    clearMessages();
    try {
      const token = requireToken();
      if (!cancellation) throw new Error("Prepare or recover a refund cancellation before submission.");
      const bytes = signedBytes ?? recoveryBytes.trim();
      if (!bytes || !refundCancellationHasSignature(bytes)) throw new Error("The governance wallet signature must be present before submission.");
      const response = await fetchWithTimeout(`${apiBase.replace(/\/$/, "")}/v1/settlement/refund-claim-cancellations/submit`, {
        method: "POST",
        headers: { "content-type": "application/json", authorization: `Bearer ${token}` },
        body: JSON.stringify({ intent_id: cancellation.intent_id, signed_transaction_base64: bytes }),
      }, REQUEST_TIMEOUT_MS.settlementSubmit);
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw new Error(responseError(response, payload, `Refund cancellation submission failed (${response.status})`));
      const result = parseRefundCancellationSubmissionResponse(payload);
      setCancellation({ ...cancellation, status: result.status as RefundCancellationResponse["status"], signature: result.signature ?? null });
      setNotice(result.status === "unknown"
        ? "The provider outcome is unknown. Do not sign or submit different bytes; finalized chain evidence is required."
        : "The provider acknowledged the unchanged cancellation transaction. This is transport evidence only; the refund claim remains active until finalized cancellation event evidence is reconciled.");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Refund cancellation submission failed.");
    } finally {
      setBusy(false);
    }
  }

  async function reconcile() {
    setBusy(true);
    clearMessages();
    try {
      const token = requireToken();
      if (!cancellation) throw new Error("Prepare or recover a refund cancellation before reconciliation.");
      const response = await fetchWithTimeout(`${apiBase.replace(/\/$/, "")}/v1/settlement/refund-claim-cancellations/${cancellation.intent_id}/reconcile`, {
        method: "POST",
        headers: { authorization: `Bearer ${token}` },
      }, REQUEST_TIMEOUT_MS.settlementSubmit);
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw new Error(responseError(response, payload, `Refund cancellation reconciliation failed (${response.status})`));
      setCancellation(parseRefundCancellationResponse(payload));
      setNotice("Finalized cancellation event evidence matched the durable artifact, and the original refund claim was converged atomically to cancelled.");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Refund cancellation reconciliation failed.");
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="adjustment-layout" aria-label="Refund claim cancellation workspace">
      <article className="panel">
        <div className="panel-heading"><div><p className="eyebrow">Refund movement / governed cancellation boundary</p><h2>Prepare a claim cancellation</h2></div><span className="status-pill">No custody</span></div>
        <p className="panel-copy">Cancellation is a governance action against a finalized refund-claim creation. The API derives all account identities and binds the artifact to the authenticated governance wallet.</p>
        <form className="form-grid" onSubmit={prepare}>
          <label className="full-field"><span>Finalized refund-claim intent ID</span><input value={claimIntentId} onChange={(event) => setClaimIntentId(event.target.value)} placeholder="UUIDv7 from the finalized claim" required /></label>
          <label className="full-field"><span>Idempotency key</span><input value={idempotencyKey} onChange={(event) => setIdempotencyKey(event.target.value)} required /></label>
          <button type="submit" disabled={busy || !serviceOnline || !accessToken.trim()}>{busy ? "Working…" : "Prepare governed cancellation"}</button>
        </form>
        <form className="form-grid" onSubmit={load}>
          <label className="full-field"><span>Recover cancellation intent ID</span><input value={lookupId} onChange={(event) => setLookupId(event.target.value)} placeholder="UUIDv7 cancellation intent" /></label>
          <button type="submit" disabled={busy || !serviceOnline || !accessToken.trim()}>Load durable state</button>
        </form>
        {!serviceOnline && <p className="inline-warning" role="alert">The service is unavailable. No client-side fallback or local cancellation state is used.</p>}
        {error && <p className="inline-error" role="alert">{error}</p>}
        {notice && <p className="inline-success" role="status">{notice}</p>}
      </article>

      <article className="panel" aria-live="polite">
        <div className="panel-heading"><div><p className="eyebrow">Immutable artifact review</p><h2>Governance wallet authorization</h2></div>{cancellation && <span className="status-pill">{statusLabel(cancellation.status)}</span>}</div>
        <WalletPicker disabled={busy} connectEnabled={Boolean(cancellation && (cancellation.status === "prepared_unsubmitted" || cancellation.status === "signed"))} connectedAddress={walletAddress?.toBase58()} onConnected={setWalletAddress} />
        {!cancellation ? <div className="empty-state"><span className="empty-icon">⊘</span><strong>No cancellation artifact loaded</strong><p>Prepare or recover an intent to inspect its exact governance-only evidence.</p></div> : <>
          <dl className="summary-list">
            <div><dt>Claim PDA</dt><dd><code>{cancellation.claim}</code></dd></div>
            <div><dt>Governance authority</dt><dd><code>{cancellation.artifact.governance_authority}</code></dd></div>
            <div><dt>Merchant</dt><dd><code>{cancellation.artifact.merchant}</code></dd></div>
            <div><dt>Asset mint</dt><dd><code>{cancellation.artifact.mint}</code></dd></div>
            <div><dt>Adjustment PDA</dt><dd><code>{cancellation.artifact.adjustment}</code></dd></div>
            <div><dt>Claim nonce</dt><dd>{cancellation.artifact.instruction.claim_nonce}</dd></div>
            <div><dt>Registry PDA</dt><dd><code>{cancellation.artifact.instruction.registry}</code></dd></div>
            <div><dt>Instruction digest</dt><dd><code>{digestPreview(cancellation.instruction_digest)}</code></dd></div>
            <div><dt>Artifact digest</dt><dd><code>{digestPreview(cancellation.artifact_digest)}</code></dd></div>
            <div><dt>Transport signature</dt><dd>{cancellation.signature ? <code>{cancellation.signature}</code> : "None recorded"}</dd></div>
          </dl>
          <div className="settlement-status">
            <strong>Cancellation moves no funds.</strong>
            <small>The on-chain instruction only marks the refund claim cancelled after governance authorization. This UI does not infer finality from wallet signing or provider acknowledgement.</small>
          </div>
          {cancellation.status === "finalized" && cancellation.finalized_evidence && <div className="settlement-status" aria-live="polite">
            <strong role="status">Finalized cancellation event evidence matched the durable artifact.</strong>
            <small>The claim is now durably cancelled. This is movement-state evidence only; it is not legal or tax advice.</small>
            <details>
              <summary>Finalized event proof</summary>
              <dl className="summary-list">
                <div><dt>Event</dt><dd>{cancellation.finalized_evidence.event_type} · {cancellation.finalized_evidence.event_version}</dd></div>
                <div><dt>Finalized slot</dt><dd>{cancellation.finalized_evidence.slot}</dd></div>
                <div><dt>Event index</dt><dd>{cancellation.finalized_evidence.event_index}</dd></div>
                <div><dt>Event signature</dt><dd><code>{cancellation.finalized_evidence.signature}</code></dd></div>
                <div><dt>Evidence digest</dt><dd><code>{digestPreview(cancellation.finalized_evidence.evidence_digest)}</code></dd></div>
              </dl>
            </details>
          </div>}
          {(cancellation.status === "prepared_unsubmitted" || cancellation.status === "signed") && <div className="evidence-actions">
            <p className="evidence-note">One governance wallet signature is required. The server verifies the exact five-account instruction, fee payer, signed bytes, and finalized route registry before submission.</p>
            <div className="button-row"><button type="button" onClick={sign} disabled={busy}>Sign as governance authority</button><button type="button" onClick={submit} disabled={busy || (!signedBytes && !recoveryBytes.trim())}>Submit unchanged signed bytes</button></div>
            <label className="full-field"><span>Exact signed bytes for recovery (base64, memory-only)</span><textarea value={recoveryBytes} onChange={(event) => setRecoveryBytes(event.target.value)} placeholder="Paste the unchanged wallet artifact when recovering a signed intent after reload." rows={3} autoComplete="off" spellCheck={false} /></label>
          </div>}
          {(cancellation.status === "unknown" || cancellation.status === "submitted" || cancellation.status === "processed" || cancellation.status === "confirmed") && <div className="evidence-actions"><p className="inline-warning" role="alert">A provider response or wallet signature is not proof of a completed cancellation. The server must recover the exact transaction at finalized commitment and match the typed event.</p><button type="button" onClick={reconcile} disabled={busy || !serviceOnline || !accessToken.trim()}>Reconcile finalized cancellation evidence</button></div>}
          {cancellation.status === "finalized" && !cancellation.finalized_evidence && <p className="inline-warning" role="alert">The durable status is finalized but its typed event proof is unavailable. Treat this as an operational inconsistency and do not infer completion.</p>}
          {cancellation.last_error_code && <p className="inline-error" role="alert">Last durable error: {cancellation.last_error_code}</p>}
        </>}
      </article>
    </section>
  );
}
