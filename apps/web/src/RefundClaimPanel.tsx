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

function atomicAmount(value: string | null, locale: string): string {
  return value === null ? "Not present" : `${formatInteger(value, locale)} atomic units`;
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
    if (!token) throw new Error("An OIDC access token with the refund-claim scope and role is required. It is held in memory only.");
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
      if (!adjustmentId.trim()) throw new Error("An approved adjustment ID is required; financial values are loaded by the API.");
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
      setNotice("The server prepared an immutable claim artifact. Review the exact amounts, accounts, authorities, and expiry before collecting wallet signatures. No funds moved.");
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
      if (!lookupId.trim()) throw new Error("A refund claim intent ID is required.");
      const response = await fetchWithTimeout(`${apiBase.replace(/\/$/, "")}/v1/settlement/refund-claims/${lookupId.trim()}`, { headers: { authorization: `Bearer ${token}` } });
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw new Error(responseError(response, payload, `Refund claim recovery failed (${response.status})`));
      setClaim(parseRefundClaimResponse(payload));
      setSignedBytes(null);
      setRecoveryBytes("");
      setNotice("The durable claim state was loaded. Previously signed bytes are not returned by the API; paste the exact wallet artifact to resume a signed submission.");
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
      if (!claim) throw new Error("Prepare or recover a refund claim before signing.");
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
      if (!claim) throw new Error("Prepare or recover a refund claim before submission.");
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
      if (!claim) throw new Error("Prepare or recover a refund claim before reconciliation.");
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
        <div className="panel-heading"><div><p className="eyebrow">Refund movement / two-signer wallet boundary</p><h2>Prepare a refund claim</h2></div><span className="status-pill">No custody</span></div>
        <p className="panel-copy">The API derives the receipt, amounts, token accounts, claim nonce, and expiry from approved evidence. The browser cannot edit financial or account fields.</p>
        <form className="form-grid" onSubmit={prepare}>
          <label className="full-field"><span>Approved adjustment ID</span><input value={adjustmentId} onChange={(event) => setAdjustmentId(event.target.value)} placeholder="UUIDv7 from the approved adjustment" required /></label>
          <label className="full-field"><span>Idempotency key</span><input value={idempotencyKey} onChange={(event) => setIdempotencyKey(event.target.value)} required /></label>
          <button type="submit" disabled={busy || !serviceOnline || !accessToken.trim()}>{busy ? "Working…" : "Prepare server-owned claim"}</button>
        </form>
        <form className="form-grid" onSubmit={load}>
          <label className="full-field"><span>Recover intent ID</span><input value={lookupId} onChange={(event) => setLookupId(event.target.value)} placeholder="UUIDv7 claim intent" /></label>
          <button type="submit" disabled={busy || !serviceOnline || !accessToken.trim()}>Load durable state</button>
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
        {!claim ? <div className="empty-state"><span className="empty-icon">↔</span><strong>No claim artifact loaded</strong><p>Prepare or recover a claim to inspect its exact server-owned evidence.</p></div> : <>
          <dl className="summary-list">
            <div><dt>Gross refund</dt><dd>{atomicAmount(claim.artifact.gross_atomic, locale)}</dd></div>
            <div><dt>Tax component</dt><dd>{atomicAmount(claim.artifact.tax_atomic, locale)}</dd></div>
            <div><dt>Net refund</dt><dd>{atomicAmount(claim.artifact.net_atomic, locale)}</dd></div>
            <div><dt>Asset mint</dt><dd><code>{claim.artifact.mint}</code></dd></div>
            <div><dt>Destination</dt><dd><code>{claim.artifact.refund_destination}</code></dd></div>
            <div><dt>Claim PDA</dt><dd><code>{claim.claim}</code></dd></div>
            <div><dt>Refund funder</dt><dd><code>{claim.artifact.refund_funder}</code></dd></div>
            <div><dt>Expiry</dt><dd>{claim.artifact.expires_at_unix ? new Date(claim.artifact.expires_at_unix * 1000).toISOString() : "Not present"}</dd></div>
            <div><dt>Transport signature</dt><dd>{claim.signature ? <code>{claim.signature}</code> : "None recorded"}</dd></div>
          </dl>
          {(claim.status === "prepared_unsubmitted" || claim.status === "signed") && <div className="evidence-actions">
            <p className="evidence-note">Two distinct signer roles are required: proposer first, then the configured refund funder. The server verifies the exact bytes and signer topology.</p>
            <div className="button-row"><button type="button" onClick={() => sign("proposer")} disabled={busy}>Sign as proposer</button><button type="button" onClick={() => sign("refund_funder")} disabled={busy || (!signedBytes && !recoveryBytes.trim())}>Sign as refund funder</button></div>
            <label className="full-field"><span>Exact signed bytes for recovery (base64, memory-only)</span><textarea value={recoveryBytes} onChange={(event) => setRecoveryBytes(event.target.value)} placeholder="Paste the unchanged wallet artifact when recovering a signed intent after reload." rows={3} autoComplete="off" spellCheck={false} /></label>
            <button type="button" onClick={submit} disabled={busy || (!signedBytes && !recoveryBytes.trim())}>{busy ? "Submitting…" : "Submit unchanged signed bytes"}</button>
          </div>}
          {(claim.status === "unknown" || claim.status === "submitted" || claim.status === "processed" || claim.status === "confirmed") && <div className="evidence-actions"><p className="inline-warning">This state requires durable chain reconciliation. A provider response or wallet signature is not proof of a completed refund.</p><button type="button" onClick={reconcile} disabled={busy || !serviceOnline || !accessToken.trim()}>Reconcile finalized claim evidence</button></div>}
          {claim.last_error_code && <p className="inline-error" role="alert">Last durable error: {claim.last_error_code}</p>}
        </>}
      </article>
    </section>
  );
}
