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

function atomicAmount(value: string | null, locale: string): string {
  return value === null ? "Not present" : `${formatInteger(value, locale)} atomic units`;
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
    if (!token) throw new Error("An OIDC access token with the refund-execute scope and approver role is required. It is held in memory only.");
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
      if (!claimIntentId.trim()) throw new Error("A finalized claim intent ID is required; all financial and account fields are loaded by the API.");
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
      setNotice("The server prepared an immutable execute artifact from finalized claim evidence. Review the roles and exact topology before signing. No funds moved.");
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
      if (!lookupId.trim()) throw new Error("A refund execute intent ID is required.");
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
      if (!execute) throw new Error("Prepare or recover a refund execution before signing.");
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
      if (!execute) throw new Error("Prepare or recover a refund execution before submission.");
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
        <div className="panel-heading"><div><p className="eyebrow">Refund movement / execution boundary</p><h2>Prepare execution</h2></div><span className="status-pill">Proof gated</span></div>
        <p className="panel-copy">Execution is available only from finalized claim evidence. The API derives the claim topology, token accounts, authorities, and configuration; the browser cannot edit financial or account fields.</p>
        <form className="form-grid" onSubmit={prepare}>
          <label className="full-field"><span>Finalized claim intent ID</span><input value={claimIntentId} onChange={(event) => setClaimIntentId(event.target.value)} placeholder="UUIDv7 finalized claim intent" required /></label>
          <label className="full-field"><span>Idempotency key</span><input value={idempotencyKey} onChange={(event) => setIdempotencyKey(event.target.value)} required /></label>
          <button type="submit" disabled={busy || !serviceOnline || !accessToken.trim()}>{busy ? "Working…" : "Prepare server-owned execution"}</button>
        </form>
        <form className="form-grid" onSubmit={load}>
          <label className="full-field"><span>Recover execute intent ID</span><input value={lookupId} onChange={(event) => setLookupId(event.target.value)} placeholder="UUIDv7 execute intent" /></label>
          <button type="submit" disabled={busy || !serviceOnline || !accessToken.trim()}>Load durable state</button>
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
        {!execute ? <div className="empty-state"><span className="empty-icon">↗</span><strong>No execute artifact loaded</strong><p>Prepare or recover an execute intent to inspect its finalized claim binding.</p></div> : <>
          <dl className="summary-list">
            <div><dt>Claim PDA</dt><dd><code>{execute.claim}</code></dd></div>
            <div><dt>Claim signature</dt><dd><code>{execute.artifact.claim_signature}</code></dd></div>
            <div><dt>Gross refund</dt><dd>{atomicAmount(execute.artifact.gross_atomic, locale)}</dd></div>
            <div><dt>Tax component</dt><dd>{atomicAmount(execute.artifact.tax_atomic, locale)}</dd></div>
            <div><dt>Net refund</dt><dd>{atomicAmount(execute.artifact.net_atomic, locale)}</dd></div>
            <div><dt>Asset mint</dt><dd><code>{execute.artifact.mint}</code></dd></div>
            <div><dt>Destination</dt><dd><code>{execute.artifact.refund_destination}</code></dd></div>
            <div><dt>Approver</dt><dd><code>{execute.artifact.instruction.accounts[0].pubkey}</code></dd></div>
            <div><dt>Refund funder</dt><dd><code>{execute.artifact.refund_funder}</code></dd></div>
            <div><dt>Transport signature</dt><dd>{execute.signature ? <code>{execute.signature}</code> : "None recorded"}</dd></div>
          </dl>
          {(execute.status === "prepared_unsubmitted" || execute.status === "signed") && <div className="evidence-actions">
            <p className="evidence-note">Two distinct signer roles are required: approver first, then the configured refund funder. The API verifies the exact bytes and signer topology.</p>
            <div className="button-row"><button type="button" onClick={() => sign("approver")} disabled={busy}>Sign as approver</button><button type="button" onClick={() => sign("refund_funder")} disabled={busy || (!signedBytes && !recoveryBytes.trim())}>Sign as refund funder</button></div>
            <label className="full-field"><span>Exact signed bytes for recovery (base64, memory-only)</span><textarea value={recoveryBytes} onChange={(event) => setRecoveryBytes(event.target.value)} placeholder="Paste unchanged wallet bytes after a browser reload." rows={3} autoComplete="off" spellCheck={false} /></label>
            <button type="button" onClick={submit} disabled={busy || (!signedBytes && !recoveryBytes.trim())}>{busy ? "Submitting…" : "Submit unchanged signed bytes"}</button>
          </div>}
          {(execute.status === "unknown" || execute.status === "submitted" || execute.status === "processed" || execute.status === "confirmed") && <p className="inline-warning">Proof pending: provider acknowledgement is not a completed refund. Finalized transaction-bound event and balance conservation evidence are required.</p>}
          {execute.status === "finalized" && <p className="inline-success" role="status">Finalized movement evidence is recorded. Review the linked event and balance proof before treating the refund as complete.</p>}
          {execute.last_error_code && <p className="inline-error" role="alert">Last durable error: {execute.last_error_code}</p>}
        </>}
      </article>
    </section>
  );
}
