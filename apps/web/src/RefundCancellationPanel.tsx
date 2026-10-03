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
    if (!token) throw new Error("Sign in with an account allowed to cancel refund requests. The credential is held in memory only.");
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
      if (!claimIntentId.trim()) throw new Error("A finalized refund request reference is required; the service loads the account and approval details.");
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
      setNotice("The cancellation is ready for review. It cannot move funds; check the refund request and approval details before signing.");
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
      if (!lookupId.trim()) throw new Error("A cancellation reference is required.");
      const response = await fetchWithTimeout(`${apiBase.replace(/\/$/, "")}/v1/settlement/refund-claim-cancellations/${lookupId.trim()}`, {
        headers: { authorization: `Bearer ${token}` },
      });
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw new Error(responseError(response, payload, `Refund cancellation recovery failed (${response.status})`));
      setCancellation(parseRefundCancellationResponse(payload));
      setSignedBytes(null);
      setRecoveryBytes("");
      setWalletAddress(null);
      setNotice("The saved cancellation was loaded. For safety, the service does not return signed wallet data; paste the unchanged wallet file to continue.");
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
      if (!cancellation) throw new Error("Prepare or recover a cancellation before signing.");
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
      if (!cancellation) throw new Error("Prepare or recover a cancellation before submission.");
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
      if (!cancellation) throw new Error("Prepare or recover a cancellation before reconciliation.");
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
        <div className="panel-heading"><div><p className="eyebrow">Refund request / controlled cancellation</p><h2>Cancel a refund request</h2></div><span className="status-pill">No money moved</span></div>
        <p className="panel-copy">Cancellation stops a confirmed refund request from moving forward. The service loads the account details and binds this decision to the approved governance wallet.</p>
        <form className="form-grid" onSubmit={prepare}>
          <label className="full-field"><span>Finalized refund request reference</span><input value={claimIntentId} onChange={(event) => setClaimIntentId(event.target.value)} placeholder="Paste the confirmed refund request reference" required /></label>
          <label className="full-field"><span>Request reference</span><input value={idempotencyKey} onChange={(event) => setIdempotencyKey(event.target.value)} aria-describedby="cancellation-reference-help" required /><small id="cancellation-reference-help">A unique reference prevents duplicate cancellation requests when you retry.</small></label>
          <button type="submit" disabled={busy || !serviceOnline || !accessToken.trim()}>{busy ? "Preparing…" : "Prepare cancellation"}</button>
        </form>
        <form className="form-grid" onSubmit={load}>
          <label className="full-field"><span>Cancellation reference</span><input value={lookupId} onChange={(event) => setLookupId(event.target.value)} placeholder="Paste a saved cancellation reference" /></label>
          <button type="submit" disabled={busy || !serviceOnline || !accessToken.trim()}>Load saved cancellation</button>
        </form>
        {!serviceOnline && <p className="inline-warning" role="alert">The service is unavailable. No client-side fallback or local cancellation state is used.</p>}
        {error && <p className="inline-error" role="alert">{error}</p>}
        {notice && <p className="inline-success" role="status">{notice}</p>}
      </article>

      <article className="panel" aria-live="polite">
        <div className="panel-heading"><div><p className="eyebrow">Immutable artifact review</p><h2>Governance wallet authorization</h2></div>{cancellation && <span className="status-pill">{statusLabel(cancellation.status)}</span>}</div>
        <WalletPicker disabled={busy} connectEnabled={Boolean(cancellation && (cancellation.status === "prepared_unsubmitted" || cancellation.status === "signed"))} connectedAddress={walletAddress?.toBase58()} onConnected={setWalletAddress} />
        {!cancellation ? <div className="empty-state"><span className="empty-icon">⊘</span><strong>No cancellation loaded</strong><p>Prepare or recover a cancellation to review its approval details before signing.</p></div> : <>
          <dl className="summary-list">
            <div><dt>On-chain refund record</dt><dd><code>{cancellation.claim}</code></dd></div>
            <div><dt>Approval authority</dt><dd><code>{cancellation.artifact.governance_authority}</code></dd></div>
            <div><dt>Merchant</dt><dd><code>{cancellation.artifact.merchant}</code></dd></div>
            <div><dt>Asset</dt><dd><code>{cancellation.artifact.mint}</code></dd></div>
            <div><dt>Adjustment record</dt><dd><code>{cancellation.artifact.adjustment}</code></dd></div>
            <div><dt>Release reference number</dt><dd>{cancellation.artifact.instruction.claim_nonce}</dd></div>
            <div><dt>Ledger registry</dt><dd><code>{cancellation.artifact.instruction.registry}</code></dd></div>
            <div><dt>Instruction evidence</dt><dd><code>{digestPreview(cancellation.instruction_digest)}</code></dd></div>
            <div><dt>Request evidence</dt><dd><code>{digestPreview(cancellation.artifact_digest)}</code></dd></div>
            <div><dt>Submission reference</dt><dd>{cancellation.signature ? <code>{cancellation.signature}</code> : "None recorded"}</dd></div>
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
            <p className="evidence-note">One governance wallet signature is required. The service checks that the approved cancellation details were not changed before submission.</p>
            <div className="button-row"><button type="button" onClick={sign} disabled={busy}>Sign as approval authority</button><button type="button" onClick={submit} disabled={busy || (!signedBytes && !recoveryBytes.trim())}>Send cancellation</button></div>
            <label className="full-field"><span>Recovery data (advanced)</span><textarea value={recoveryBytes} onChange={(event) => setRecoveryBytes(event.target.value)} aria-describedby="cancellation-recovery-help" placeholder="Paste the unchanged wallet file after a browser reload." rows={3} autoComplete="off" spellCheck={false} /><small id="cancellation-recovery-help">This temporary data stays in memory and is sent unchanged for verification.</small></label>
          </div>}
          {(cancellation.status === "unknown" || cancellation.status === "submitted" || cancellation.status === "processed" || cancellation.status === "confirmed") && <div className="evidence-actions"><p className="inline-warning" role="alert">A provider response or wallet signature is not proof of a completed cancellation. The server must recover the exact transaction at finalized commitment and match the typed event.</p><button type="button" onClick={reconcile} disabled={busy || !serviceOnline || !accessToken.trim()}>Reconcile finalized cancellation evidence</button></div>}
          {cancellation.status === "finalized" && !cancellation.finalized_evidence && <p className="inline-warning" role="alert">The durable status is finalized but its typed event proof is unavailable. Treat this as an operational inconsistency and do not infer completion.</p>}
          {cancellation.last_error_code && <p className="inline-error" role="alert">Last durable error: {cancellation.last_error_code}</p>}
        </>}
      </article>
    </section>
  );
}
