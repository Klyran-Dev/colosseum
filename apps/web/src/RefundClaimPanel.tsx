import { FormEvent, useState } from "react";
import { PublicKey } from "@solana/web3.js";
import { fetchWithTimeout, REQUEST_TIMEOUT_MS, responseError } from "./api";
import { createClientUuid } from "./clientIdentity";
import {
  RefundClaimResponse,
  parseRefundClaimResponse,
  parseRefundClaimSubmissionResponse,
  refundClaimHasAllSignatures,
  signRefundClaimRole,
} from "./refundClaims";
import { formatInteger } from "./formatters";
import WalletPicker from "./WalletPicker";

type RefundClaimPanelProps = {
  apiBase: string;
  serviceOnline: boolean;
  accessToken: string;
  locale: string;
};

function statusLabel(status: string): string {
  return status.replaceAll("_", " ");
}

function amountValue(value: string | null, locale: string): string {
  return value === null ? "Not present" : formatInteger(value, locale);
}

function digestPreview(bytes: number[]): string {
  return `${bytes.slice(0, 8).map((byte) => byte.toString(16).padStart(2, "0")).join("")}…`;
}

export default function RefundClaimPanel({ apiBase, serviceOnline, accessToken, locale }: RefundClaimPanelProps) {
  const [adjustmentId, setAdjustmentId] = useState("");
  const [lookupId, setLookupId] = useState("");
  const [idempotencyKey, setIdempotencyKey] = useState(() => createClientUuid());
  const [claim, setClaim] = useState<RefundClaimResponse | null>(null);
  const [signedBytes, setSignedBytes] = useState<string | null>(null);
  const [recoveryBytes, setRecoveryBytes] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [walletAddress, setWalletAddress] = useState<PublicKey | null>(null);

  function requireToken(): string {
    const token = accessToken.trim();
    if (!token) throw new Error("Sign in with an account allowed to prepare refund requests. The credential is held in memory only.");
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
      if (!adjustmentId.trim()) throw new Error("An approved correction reference is required; financial values are loaded by the service.");
      const response = await fetchWithTimeout(`${apiBase.replace(/\/$/, "")}/v1/settlement/refund-claims`, {
        method: "POST",
        headers: { "content-type": "application/json", authorization: `Bearer ${token}` },
        body: JSON.stringify({ idempotency_key: idempotencyKey.trim(), adjustment_id: adjustmentId.trim(), confirm: true }),
      });
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw new Error(responseError(response, payload, `Refund claim preparation failed (${response.status})`));
      const parsed = parseRefundClaimResponse(payload);
      setClaim(parsed);
      setSignedBytes(null);
      setRecoveryBytes("");
      setLookupId(parsed.intent_id);
      setNotice("Your refund request is ready for review. Check the amounts, recipient, approval roles, and expiry before signing. No funds moved.");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Refund claim preparation failed.");
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
      if (!lookupId.trim()) throw new Error("A refund request reference is required.");
      const response = await fetchWithTimeout(`${apiBase.replace(/\/$/, "")}/v1/settlement/refund-claims/${lookupId.trim()}`, { headers: { authorization: `Bearer ${token}` } });
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw new Error(responseError(response, payload, `Refund claim recovery failed (${response.status})`));
      setClaim(parseRefundClaimResponse(payload));
      setSignedBytes(null);
      setRecoveryBytes("");
      setNotice("The saved refund request was loaded. For safety, the service does not return signed wallet data; paste the unchanged wallet file to continue.");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Refund claim recovery failed.");
    } finally {
      setBusy(false);
    }
  }

  async function sign(role: "proposer" | "refund_funder") {
    setBusy(true);
    clearMessages();
    try {
      if (!claim) throw new Error("Prepare or recover a refund request before signing.");
      if (claim.status !== "prepared_unsubmitted" && claim.status !== "signed") throw new Error("This claim is not in a wallet-signable state. Reconcile the durable state before taking another action.");
      const serialized = signedBytes ?? (recoveryBytes.trim() || null);
      if (role === "refund_funder" && !serialized) throw new Error("The proposer signature is required before the refund-funder wallet can sign.");
      const expected = new PublicKey(role === "proposer" ? claim.artifact.proposer : claim.artifact.refund_funder);
      const next = await signRefundClaimRole(claim.artifact.instruction, role, expected, import.meta.env.VITE_SOLANA_RPC_URL ?? "", serialized);
      setSignedBytes(next);
      setRecoveryBytes("");
      setNotice(refundClaimHasAllSignatures(next)
        ? "Both wallet signatures are present in the local transaction artifact. Submit these unchanged bytes to the API; a signature is not a refund confirmation."
        : role === "proposer"
          ? "Proposer signature collected. Switch to the configured refund-funder wallet and collect its signature before submission."
          : "Wallet signature collected. The API still must verify and submit the complete transaction.");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Refund claim wallet signing failed.");
    } finally {
      setBusy(false);
    }
  }

  async function submit() {
    setBusy(true);
    clearMessages();
    try {
      const token = requireToken();
      if (!claim) throw new Error("Prepare or recover a refund request before submission.");
      const bytes = signedBytes ?? recoveryBytes.trim();
      if (!bytes || !refundClaimHasAllSignatures(bytes)) throw new Error("Both required wallet signatures must be present before submission.");
      const response = await fetchWithTimeout(`${apiBase.replace(/\/$/, "")}/v1/settlement/refund-claims/submit`, {
        method: "POST",
        headers: { "content-type": "application/json", authorization: `Bearer ${token}` },
        body: JSON.stringify({ intent_id: claim.intent_id, signed_transaction_base64: bytes }),
      }, REQUEST_TIMEOUT_MS.settlementSubmit);
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw new Error(responseError(response, payload, `Refund claim submission failed (${response.status})`));
      const result = parseRefundClaimSubmissionResponse(payload);
      setClaim({ ...claim, status: result.status, signature: result.signature });
      setNotice(result.status === "unknown"
        ? "The provider outcome is unknown. Do not sign or submit different bytes; reconcile the durable claim against finalized chain evidence."
        : "The provider accepted the unchanged transaction bytes. This is transport evidence only; the claim is not a completed refund until finalized event and balance proof are recorded.");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Refund claim submission failed.");
    } finally {
      setBusy(false);
    }
  }

  async function reconcile() {
    setBusy(true);
    clearMessages();
    try {
      const token = requireToken();
      if (!claim) throw new Error("Prepare or recover a refund request before reconciliation.");
      const response = await fetchWithTimeout(`${apiBase.replace(/\/$/, "")}/v1/settlement/refund-claims/${claim.intent_id}/reconcile`, {
        method: "POST",
        headers: { authorization: `Bearer ${token}` },
      }, REQUEST_TIMEOUT_MS.settlementSubmit);
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw new Error(responseError(response, payload, `Refund claim reconciliation failed (${response.status})`));
      setClaim(parseRefundClaimResponse(payload));
      setNotice("Finalized claim event evidence matched the durable artifact. Execute preparation is now eligible for the separately authorized approver workflow.");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Refund claim reconciliation failed.");
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="adjustment-layout" aria-label="Refund claim wallet workspace">
      <article className="panel">
        <div className="panel-heading"><div><p className="eyebrow">Refund request / two-person approval</p><h2>Prepare a refund request</h2></div><span className="status-pill">You keep custody</span></div>
        <p className="panel-copy">The service loads the approved payment details and prepares the refund for review. You can review the result, but cannot change its financial or recipient details in the browser.</p>
        <form className="form-grid" onSubmit={prepare}>
          <label className="full-field"><span>Approved correction reference</span><input value={adjustmentId} onChange={(event) => setAdjustmentId(event.target.value)} placeholder="Paste the approved correction reference" required /></label>
          <label className="full-field"><span>Request reference</span><input value={idempotencyKey} onChange={(event) => setIdempotencyKey(event.target.value)} aria-describedby="refund-request-reference-help" required /><small id="refund-request-reference-help">A unique reference prevents duplicate requests when you retry.</small></label>
          <button type="submit" disabled={busy || !serviceOnline || !accessToken.trim()}>{busy ? "Preparing…" : "Prepare refund request"}</button>
        </form>
        <form className="form-grid" onSubmit={load}>
          <label className="full-field"><span>Refund request reference</span><input value={lookupId} onChange={(event) => setLookupId(event.target.value)} placeholder="Paste a saved request reference" /></label>
          <button type="submit" disabled={busy || !serviceOnline || !accessToken.trim()}>Load saved request</button>
        </form>
        {!serviceOnline && <p className="inline-warning" role="alert">The service is unavailable. No client-side fallback or local settlement state is used.</p>}
        {error && <p className="inline-error" role="alert">{error}</p>}
        {notice && <p className="inline-success" role="status">{notice}</p>}
      </article>

      <article className="panel" aria-live="polite">
        <div className="panel-heading"><div><p className="eyebrow">Immutable artifact review</p><h2>Wallet authorization</h2></div>{claim && <span className="status-pill">{statusLabel(claim.status)}</span>}</div>
        <WalletPicker
          disabled={busy}
          connectEnabled={Boolean(claim && (claim.status === "prepared_unsubmitted" || claim.status === "signed"))}
          connectedAddress={walletAddress?.toBase58()}
          onConnected={setWalletAddress}
        />
        {!claim ? <div className="empty-state"><span className="empty-icon">↔</span><strong>No refund request loaded</strong><p>Prepare or recover a request to review its payment details and approval status.</p></div> : <>
          <dl className="summary-list">
            <div><dt>Gross refund</dt><dd>{amountValue(claim.artifact.gross_atomic, locale)}</dd></div>
            <div><dt>Tax included</dt><dd>{amountValue(claim.artifact.tax_atomic, locale)}</dd></div>
            <div><dt>Refund after tax</dt><dd>{amountValue(claim.artifact.net_atomic, locale)}</dd></div>
            <div><dt>Asset</dt><dd><code>{claim.artifact.mint}</code></dd></div>
            <div><dt>Destination</dt><dd><code>{claim.artifact.refund_destination}</code></dd></div>
            <div><dt>On-chain refund record</dt><dd><code>{claim.claim}</code></dd></div>
            <div><dt>Refund account</dt><dd><code>{claim.artifact.refund_funder}</code></dd></div>
            <div><dt>Expiry</dt><dd>{claim.artifact.expires_at_unix ? new Date(claim.artifact.expires_at_unix * 1000).toISOString() : "Not present"}</dd></div>
            <div><dt>Submission reference</dt><dd>{claim.signature ? <code>{claim.signature}</code> : "None recorded"}</dd></div>
          </dl>
          {claim.status === "finalized" && claim.finalized_evidence && <div className="settlement-status" aria-live="polite">
            <strong role="status">Finalized claim event evidence matched the durable artifact.</strong>
            <small>The event proof is bound to the recorded signature, finalized slot, claim PDA, amounts, and immutable evidence digest. Finality is movement evidence, not a legal or compliance determination.</small>
            <details>
              <summary>Finalized event proof</summary>
              <dl className="summary-list">
                <div><dt>Event</dt><dd>{claim.finalized_evidence.event_type} · {claim.finalized_evidence.event_version}</dd></div>
                <div><dt>Finalized slot</dt><dd>{claim.finalized_evidence.slot}</dd></div>
                <div><dt>Event index</dt><dd>{claim.finalized_evidence.event_index}</dd></div>
                <div><dt>Event signature</dt><dd><code>{claim.finalized_evidence.signature}</code></dd></div>
                <div><dt>Gross / tax / refund</dt><dd>{claim.finalized_evidence.gross_magnitude} / {claim.finalized_evidence.tax_magnitude} / {claim.finalized_evidence.net_magnitude}</dd></div>
                <div><dt>Evidence digest</dt><dd><code>{digestPreview(claim.finalized_evidence.evidence_digest)}</code></dd></div>
              </dl>
            </details>
          </div>}
          {(claim.status === "prepared_unsubmitted" || claim.status === "signed") && <div className="evidence-actions">
            <p className="evidence-note">Two different people must approve this refund: the request owner first, then the configured refund account owner. The service checks that the approved payment details were not changed.</p>
            <div className="button-row"><button type="button" onClick={() => sign("proposer")} disabled={busy}>Sign as proposer</button><button type="button" onClick={() => sign("refund_funder")} disabled={busy || (!signedBytes && !recoveryBytes.trim())}>Sign as refund funder</button></div>
            <label className="full-field"><span>Recovery data (advanced)</span><textarea value={recoveryBytes} onChange={(event) => setRecoveryBytes(event.target.value)} placeholder="Paste the unchanged wallet file after a browser reload." aria-describedby="refund-recovery-help" rows={3} autoComplete="off" spellCheck={false} /><small id="refund-recovery-help">This temporary data stays in memory and is sent unchanged for verification.</small></label>
            <button type="button" onClick={submit} disabled={busy || (!signedBytes && !recoveryBytes.trim())}>{busy ? "Sending…" : "Send approved refund"}</button>
          </div>}
          {(claim.status === "unknown" || claim.status === "submitted" || claim.status === "processed" || claim.status === "confirmed") && <div className="evidence-actions"><p className="inline-warning">This state requires durable chain reconciliation. A provider response or wallet signature is not proof of a completed refund.</p><button type="button" onClick={reconcile} disabled={busy || !serviceOnline || !accessToken.trim()}>Reconcile finalized claim evidence</button></div>}
          {claim.last_error_code && <p className="inline-error" role="alert">Last durable error: {claim.last_error_code}</p>}
        </>}
      </article>
    </section>
  );
}
