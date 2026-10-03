import { FormEvent, useState } from "react";
import { PublicKey } from "@solana/web3.js";
import { fetchWithTimeout, REQUEST_TIMEOUT_MS, responseError } from "./api";
import { createClientUuid } from "./clientIdentity";
import { formatInteger } from "./formatters";
import WalletPicker from "./WalletPicker";
import {
  RefundExecuteResponse,
  parseRefundExecuteResponse,
  parseRefundExecuteSubmissionResponse,
  refundExecuteHasAllSignatures,
  signRefundExecuteRole,
} from "./refundExecutions";

type RefundExecutePanelProps = {
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

export default function RefundExecutePanel({ apiBase, serviceOnline, accessToken, locale }: RefundExecutePanelProps) {
  const [claimIntentId, setClaimIntentId] = useState("");
  const [lookupId, setLookupId] = useState("");
  const [idempotencyKey, setIdempotencyKey] = useState(() => createClientUuid());
  const [execute, setExecute] = useState<RefundExecuteResponse | null>(null);
  const [signedBytes, setSignedBytes] = useState<string | null>(null);
  const [recoveryBytes, setRecoveryBytes] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [walletAddress, setWalletAddress] = useState<PublicKey | null>(null);

  function requireToken(): string {
    const token = accessToken.trim();
    if (!token) throw new Error("Sign in with an account allowed to release an approved refund. The credential is held in memory only.");
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
      if (!claimIntentId.trim()) throw new Error("A finalized refund request reference is required; all financial and recipient details are loaded by the service.");
      const response = await fetchWithTimeout(`${apiBase.replace(/\/$/, "")}/v1/settlement/refund-executions`, {
        method: "POST",
        headers: { "content-type": "application/json", authorization: `Bearer ${token}` },
        body: JSON.stringify({ idempotency_key: idempotencyKey.trim(), claim_intent_id: claimIntentId.trim(), confirm: true }),
      });
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw new Error(responseError(response, payload, `Refund execution preparation failed (${response.status})`));
      const parsed = parseRefundExecuteResponse(payload);
      setExecute(parsed);
      setLookupId(parsed.intent_id);
      setSignedBytes(null);
      setRecoveryBytes("");
      setNotice("The refund release is ready for review. Check the approval roles and recipient details before signing. No funds moved.");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Refund execution preparation failed.");
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
      if (!lookupId.trim()) throw new Error("A refund release reference is required.");
      const response = await fetchWithTimeout(`${apiBase.replace(/\/$/, "")}/v1/settlement/refund-executions/${lookupId.trim()}`, { headers: { authorization: `Bearer ${token}` } });
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw new Error(responseError(response, payload, `Refund execution recovery failed (${response.status})`));
      setExecute(parseRefundExecuteResponse(payload));
      setSignedBytes(null);
      setRecoveryBytes("");
      setNotice("The durable execute state was loaded. Previously signed bytes are not returned by the API; paste the unchanged wallet artifact to resume after reload.");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Refund execution recovery failed.");
    } finally {
      setBusy(false);
    }
  }

  async function sign(role: "approver" | "refund_funder") {
    setBusy(true);
    clearMessages();
    try {
      if (!execute) throw new Error("Prepare or recover a refund release before signing.");
      if (execute.status !== "prepared_unsubmitted" && execute.status !== "signed") throw new Error("This execution is not in a wallet-signable state. Reconcile the durable state before taking another action.");
      const serialized = signedBytes ?? (recoveryBytes.trim() || null);
      if (role === "refund_funder" && !serialized) throw new Error("The approver signature is required before the refund-funder wallet can sign.");
      const expected = new PublicKey(role === "approver" ? execute.artifact.instruction.accounts[0].pubkey : execute.artifact.refund_funder);
      const next = await signRefundExecuteRole(execute.artifact.instruction, role, expected, import.meta.env.VITE_SOLANA_RPC_URL ?? "", serialized);
      setSignedBytes(next);
      setRecoveryBytes("");
      setNotice(refundExecuteHasAllSignatures(next)
        ? "Both execute wallet signatures are present in local memory. Submit these unchanged bytes; a signature is not a finalized refund proof."
        : role === "approver"
          ? "Approver signature collected. Switch to the configured refund-funder wallet and collect its signature."
          : "Wallet signature collected. The API still must verify and submit the complete transaction.");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Refund execution wallet signing failed.");
    } finally {
      setBusy(false);
    }
  }

  async function submit() {
    setBusy(true);
    clearMessages();
    try {
      const token = requireToken();
      if (!execute) throw new Error("Prepare or recover a refund release before submission.");
      const bytes = signedBytes ?? recoveryBytes.trim();
      if (!bytes || !refundExecuteHasAllSignatures(bytes)) throw new Error("Both required wallet signatures must be present before submission.");
      const response = await fetchWithTimeout(`${apiBase.replace(/\/$/, "")}/v1/settlement/refund-executions/submit`, {
        method: "POST",
        headers: { "content-type": "application/json", authorization: `Bearer ${token}` },
        body: JSON.stringify({ intent_id: execute.intent_id, signed_transaction_base64: bytes }),
      }, REQUEST_TIMEOUT_MS.settlementSubmit);
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw new Error(responseError(response, payload, `Refund execution submission failed (${response.status})`));
      const result = parseRefundExecuteSubmissionResponse(payload);
      setExecute({ ...execute, status: result.status, signature: result.signature });
      setNotice(result.status === "unknown"
        ? "The provider outcome is unknown. Do not submit different bytes; reconcile against finalized transaction and balance evidence."
        : "The provider accepted the unchanged execute transaction. The refund remains proof-pending until finalized event and balance evidence are recorded.");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Refund execution submission failed.");
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="adjustment-layout" aria-label="Refund execution wallet workspace">
      <article className="panel">
        <div className="panel-heading"><div><p className="eyebrow">Refund release / final approval</p><h2>Prepare refund release</h2></div><span className="status-pill">Proof required</span></div>
        <p className="panel-copy">A refund can be released only after the original request is confirmed. The service loads the approved payment and recipient details; the browser cannot change them.</p>
        <form className="form-grid" onSubmit={prepare}>
          <label className="full-field"><span>Finalized refund request reference</span><input value={claimIntentId} onChange={(event) => setClaimIntentId(event.target.value)} placeholder="Paste the confirmed refund request reference" required /></label>
          <label className="full-field"><span>Request reference</span><input value={idempotencyKey} onChange={(event) => setIdempotencyKey(event.target.value)} aria-describedby="refund-release-reference-help" required /><small id="refund-release-reference-help">A unique reference prevents duplicate release requests when you retry.</small></label>
          <button type="submit" disabled={busy || !serviceOnline || !accessToken.trim()}>{busy ? "Preparing…" : "Prepare refund release"}</button>
        </form>
        <form className="form-grid" onSubmit={load}>
          <label className="full-field"><span>Refund release reference</span><input value={lookupId} onChange={(event) => setLookupId(event.target.value)} placeholder="Paste a saved release reference" /></label>
          <button type="submit" disabled={busy || !serviceOnline || !accessToken.trim()}>Load saved release</button>
        </form>
        {!serviceOnline && <p className="inline-warning" role="alert">The service is unavailable. No client-side fallback or local settlement state is used.</p>}
        {error && <p className="inline-error" role="alert">{error}</p>}
        {notice && <p className="inline-success" role="status">{notice}</p>}
      </article>

      <article className="panel" aria-live="polite">
        <div className="panel-heading"><div><p className="eyebrow">Immutable artifact review</p><h2>Execution authorization</h2></div>{execute && <span className="status-pill">{statusLabel(execute.status)}</span>}</div>
        <WalletPicker
          disabled={busy}
          connectEnabled={Boolean(execute && (execute.status === "prepared_unsubmitted" || execute.status === "signed"))}
          connectedAddress={walletAddress?.toBase58()}
          onConnected={setWalletAddress}
        />
        {!execute ? <div className="empty-state"><span className="empty-icon">↗</span><strong>No refund release loaded</strong><p>Prepare or recover a release to review its confirmed payment and recipient details.</p></div> : <>
          <dl className="summary-list">
            <div><dt>On-chain refund record</dt><dd><code>{execute.claim}</code></dd></div>
            <div><dt>Confirmed request reference</dt><dd><code>{execute.artifact.claim_signature}</code></dd></div>
            <div><dt>Gross refund</dt><dd>{amountValue(execute.artifact.gross_atomic, locale)}</dd></div>
            <div><dt>Tax included</dt><dd>{amountValue(execute.artifact.tax_atomic, locale)}</dd></div>
            <div><dt>Refund after tax</dt><dd>{amountValue(execute.artifact.net_atomic, locale)}</dd></div>
            <div><dt>Asset</dt><dd><code>{execute.artifact.mint}</code></dd></div>
            <div><dt>Destination</dt><dd><code>{execute.artifact.refund_destination}</code></dd></div>
            <div><dt>Final approver</dt><dd><code>{execute.artifact.instruction.accounts[0].pubkey}</code></dd></div>
            <div><dt>Refund account</dt><dd><code>{execute.artifact.refund_funder}</code></dd></div>
            <div><dt>Submission reference</dt><dd>{execute.signature ? <code>{execute.signature}</code> : "None recorded"}</dd></div>
          </dl>
          {(execute.status === "prepared_unsubmitted" || execute.status === "signed") && <div className="evidence-actions">
            <p className="evidence-note">Two different people must approve this release: the final approver first, then the configured refund account owner. The service checks that the approved details were not changed.</p>
            <div className="button-row"><button type="button" onClick={() => sign("approver")} disabled={busy}>Sign as approver</button><button type="button" onClick={() => sign("refund_funder")} disabled={busy || (!signedBytes && !recoveryBytes.trim())}>Sign as refund funder</button></div>
            <label className="full-field"><span>Recovery data (advanced)</span><textarea value={recoveryBytes} onChange={(event) => setRecoveryBytes(event.target.value)} placeholder="Paste the unchanged wallet file after a browser reload." aria-describedby="refund-release-recovery-help" rows={3} autoComplete="off" spellCheck={false} /><small id="refund-release-recovery-help">This temporary data stays in memory and is sent unchanged for verification.</small></label>
            <button type="button" onClick={submit} disabled={busy || (!signedBytes && !recoveryBytes.trim())}>{busy ? "Sending…" : "Send approved refund"}</button>
          </div>}
          {(execute.status === "unknown" || execute.status === "submitted" || execute.status === "processed" || execute.status === "confirmed") && <p className="inline-warning">Proof pending: provider acknowledgement is not a completed refund. Finalized transaction-bound event and balance conservation evidence are required.</p>}
          {execute.status === "finalized" && <p className="inline-success" role="status">Finalized movement evidence is recorded. Review the linked event and balance proof before treating the refund as complete.</p>}
          {execute.last_error_code && <p className="inline-error" role="alert">Last durable error: {execute.last_error_code}</p>}
        </>}
      </article>
    </section>
  );
}
