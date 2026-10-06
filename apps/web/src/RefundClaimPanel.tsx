import { FormEvent, useState } from "react";
import { PublicKey } from "@solana/web3.js";
import { fetchWithTimeout, operatorResponseError, REQUEST_TIMEOUT_MS } from "./api";
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
import { refundClaimMessagesFor } from "./refundClaimMessages";
import { operatorErrorMessage } from "./operatorError";

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
  const messages = refundClaimMessagesFor(locale);
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
    if (!token) throw new Error(messages.errors.authorizationRequired);
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
      if (!adjustmentId.trim()) throw new Error(messages.errors.correctionReferenceRequired);
      const response = await fetchWithTimeout(`${apiBase.replace(/\/$/, "")}/v1/settlement/refund-claims`, {
        method: "POST",
        headers: { "content-type": "application/json", authorization: `Bearer ${token}` },
        body: JSON.stringify({ idempotency_key: idempotencyKey.trim(), adjustment_id: adjustmentId.trim(), confirm: true }),
      });
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw operatorResponseError(response, payload, messages.errors.preparationRequestFailed(response.status));
      const parsed = parseRefundClaimResponse(payload);
      setClaim(parsed);
      setSignedBytes(null);
      setRecoveryBytes("");
      setLookupId(parsed.intent_id);
      setNotice(messages.errors.preparationReady);
    } catch (cause) {
      setError(operatorErrorMessage(cause, messages.errors.preparationFailed));
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
      if (!lookupId.trim()) throw new Error(messages.errors.requestReferenceRequired);
      const response = await fetchWithTimeout(`${apiBase.replace(/\/$/, "")}/v1/settlement/refund-claims/${lookupId.trim()}`, { headers: { authorization: `Bearer ${token}` } });
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw operatorResponseError(response, payload, messages.errors.recoveryRequestFailed(response.status));
      setClaim(parseRefundClaimResponse(payload));
      setSignedBytes(null);
      setRecoveryBytes("");
      setNotice(messages.errors.recoveryLoaded);
    } catch (cause) {
      setError(operatorErrorMessage(cause, messages.errors.recoveryFailed));
    } finally {
      setBusy(false);
    }
  }

  async function sign(role: "proposer" | "refund_funder") {
    setBusy(true);
    clearMessages();
    try {
      if (!claim) throw new Error(messages.errors.prepareBeforeSigning);
      if (claim.status !== "prepared_unsubmitted" && claim.status !== "signed") throw new Error(messages.errors.signableStateRequired);
      const serialized = signedBytes ?? (recoveryBytes.trim() || null);
      if (role === "refund_funder" && !serialized) throw new Error(messages.errors.funderPrerequisite);
      const expected = new PublicKey(role === "proposer" ? claim.artifact.proposer : claim.artifact.refund_funder);
      const next = await signRefundClaimRole(claim.artifact.instruction, role, expected, import.meta.env.VITE_SOLANA_RPC_URL ?? "", serialized);
      setSignedBytes(next);
      setRecoveryBytes("");
      setNotice(refundClaimHasAllSignatures(next)
        ? messages.errors.bothSignaturesPresent
        : role === "proposer"
          ? messages.errors.proposerSignaturePresent
          : messages.errors.funderSignaturePresent);
    } catch (cause) {
      setError(operatorErrorMessage(cause, messages.errors.signingFailed));
    } finally {
      setBusy(false);
    }
  }

  async function submit() {
    setBusy(true);
    clearMessages();
    try {
      const token = requireToken();
      if (!claim) throw new Error(messages.errors.prepareBeforeSubmission);
      const bytes = signedBytes ?? recoveryBytes.trim();
      if (!bytes || !refundClaimHasAllSignatures(bytes)) throw new Error(messages.errors.signaturesRequired);
      const response = await fetchWithTimeout(`${apiBase.replace(/\/$/, "")}/v1/settlement/refund-claims/submit`, {
        method: "POST",
        headers: { "content-type": "application/json", authorization: `Bearer ${token}` },
        body: JSON.stringify({ intent_id: claim.intent_id, signed_transaction_base64: bytes }),
      }, REQUEST_TIMEOUT_MS.settlementSubmit);
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw operatorResponseError(response, payload, messages.errors.submissionRequestFailed(response.status));
      const result = parseRefundClaimSubmissionResponse(payload);
      setClaim({ ...claim, status: result.status, signature: result.signature });
      setNotice(result.status === "unknown"
        ? messages.errors.submissionUnknown
        : messages.errors.submissionPending);
    } catch (cause) {
      setError(operatorErrorMessage(cause, messages.errors.submissionFailed));
    } finally {
      setBusy(false);
    }
  }

  async function reconcile() {
    setBusy(true);
    clearMessages();
    try {
      const token = requireToken();
      if (!claim) throw new Error(messages.errors.prepareBeforeReconciliation);
      const response = await fetchWithTimeout(`${apiBase.replace(/\/$/, "")}/v1/settlement/refund-claims/${claim.intent_id}/reconcile`, {
        method: "POST",
        headers: { authorization: `Bearer ${token}` },
      }, REQUEST_TIMEOUT_MS.settlementSubmit);
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw operatorResponseError(response, payload, messages.errors.reconciliationRequestFailed(response.status));
      setClaim(parseRefundClaimResponse(payload));
      setNotice(messages.errors.reconciliationReady);
    } catch (cause) {
      setError(operatorErrorMessage(cause, messages.errors.reconciliationFailed));
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="adjustment-layout" aria-label={messages.proposal.ariaLabel}>
      <article className="panel">
        <div className="panel-heading"><div><p className="eyebrow">{messages.proposal.eyebrow}</p><h2>{messages.proposal.title}</h2></div><span className="status-pill">{messages.proposal.badge}</span></div>
        <p className="panel-copy">{messages.proposal.description}</p>
        <form className="form-grid" onSubmit={prepare}>
          <label className="full-field"><span>{messages.proposal.correctionReference}</span><input aria-label={messages.proposal.correctionReference} value={adjustmentId} onChange={(event) => setAdjustmentId(event.target.value)} placeholder={messages.proposal.correctionPlaceholder} required /></label>
          <label className="full-field"><span>{messages.proposal.requestReference}</span><input value={idempotencyKey} onChange={(event) => setIdempotencyKey(event.target.value)} aria-describedby="refund-request-reference-help" required /><small id="refund-request-reference-help">{messages.proposal.requestHelp}</small></label>
          <button type="submit" disabled={busy || !serviceOnline || !accessToken.trim()}>{busy ? messages.proposal.preparing : messages.proposal.prepare}</button>
        </form>
        <form className="form-grid" onSubmit={load}>
          <label className="full-field"><span>{messages.proposal.refundRequestReference}</span><input aria-label={messages.proposal.refundRequestReference} value={lookupId} onChange={(event) => setLookupId(event.target.value)} placeholder={messages.proposal.savedRequestPlaceholder} /></label>
          <button type="submit" disabled={busy || !serviceOnline || !accessToken.trim()}>{messages.proposal.load}</button>
        </form>
        {!serviceOnline && <p className="inline-warning" role="alert">{messages.proposal.serviceUnavailable}</p>}
        {error && <p className="inline-error" role="alert">{error}</p>}
        {notice && <p className="inline-success" role="status">{notice}</p>}
      </article>

      <article className="panel" aria-label={messages.review.ariaLabel} aria-live="polite">
        <div className="panel-heading"><div><p className="eyebrow">{messages.review.eyebrow}</p><h2>{messages.review.title}</h2></div>{claim && <span className="status-pill">{statusLabel(claim.status)}</span>}</div>
        <WalletPicker
          disabled={busy}
          connectEnabled={Boolean(claim && (claim.status === "prepared_unsubmitted" || claim.status === "signed"))}
          connectedAddress={walletAddress?.toBase58()}
          onConnected={setWalletAddress}
          locale={locale}
        />
        {!claim ? <div className="empty-state"><span className="empty-icon">↔</span><strong>{messages.review.noRequest}</strong><p>{messages.review.noRequestCopy}</p></div> : <>
          <dl className="summary-list">
            <div><dt>{messages.review.grossRefund}</dt><dd>{amountValue(claim.artifact.gross_atomic, locale)}</dd></div>
            <div><dt>{messages.review.taxIncluded}</dt><dd>{amountValue(claim.artifact.tax_atomic, locale)}</dd></div>
            <div><dt>{messages.review.refundAfterTax}</dt><dd>{amountValue(claim.artifact.net_atomic, locale)}</dd></div>
            <div><dt>{messages.review.asset}</dt><dd><code>{claim.artifact.mint}</code></dd></div>
            <div><dt>{messages.review.destination}</dt><dd><code>{claim.artifact.refund_destination}</code></dd></div>
            <div><dt>{messages.review.onChainRecord}</dt><dd><code>{claim.claim}</code></dd></div>
            <div><dt>{messages.review.refundAccount}</dt><dd><code>{claim.artifact.refund_funder}</code></dd></div>
            <div><dt>{messages.review.expiry}</dt><dd>{claim.artifact.expires_at_unix ? new Date(claim.artifact.expires_at_unix * 1000).toISOString() : "Not present"}</dd></div>
            <div><dt>{messages.review.submissionReference}</dt><dd>{claim.signature ? <code>{claim.signature}</code> : messages.review.noneRecorded}</dd></div>
          </dl>
          {claim.status === "finalized" && claim.finalized_evidence && <div className="settlement-status" aria-live="polite">
            <strong role="status">{messages.review.finalizedTitle}</strong>
            <small>{messages.review.finalizedCopy}</small>
            <details>
              <summary>{messages.review.finalizedProof}</summary>
              <dl className="summary-list">
                <div><dt>{messages.review.event}</dt><dd>{claim.finalized_evidence.event_type} · {claim.finalized_evidence.event_version}</dd></div>
                <div><dt>{messages.review.finalizedSlot}</dt><dd>{claim.finalized_evidence.slot}</dd></div>
                <div><dt>{messages.review.eventIndex}</dt><dd>{claim.finalized_evidence.event_index}</dd></div>
                <div><dt>{messages.review.eventSignature}</dt><dd><code>{claim.finalized_evidence.signature}</code></dd></div>
                <div><dt>{messages.review.grossTaxRefund}</dt><dd>{claim.finalized_evidence.gross_magnitude} / {claim.finalized_evidence.tax_magnitude} / {claim.finalized_evidence.net_magnitude}</dd></div>
                <div><dt>{messages.review.evidenceDigest}</dt><dd><code>{digestPreview(claim.finalized_evidence.evidence_digest)}</code></dd></div>
              </dl>
            </details>
          </div>}
          {(claim.status === "prepared_unsubmitted" || claim.status === "signed") && <div className="evidence-actions">
            <p className="evidence-note">{messages.wallet.twoPersonNote}</p>
            <div className="button-row"><button type="button" onClick={() => sign("proposer")} disabled={busy}>{messages.wallet.signProposer}</button><button type="button" onClick={() => sign("refund_funder")} disabled={busy || (!signedBytes && !recoveryBytes.trim())}>{messages.wallet.signFunder}</button></div>
            <label className="full-field"><span>{messages.wallet.recovery}</span><textarea aria-label={messages.wallet.recovery} value={recoveryBytes} onChange={(event) => setRecoveryBytes(event.target.value)} placeholder={messages.wallet.recoveryPlaceholder} aria-describedby="refund-recovery-help" rows={3} autoComplete="off" spellCheck={false} /><small id="refund-recovery-help">{messages.wallet.recoveryHelp}</small></label>
            <button type="button" onClick={submit} disabled={busy || (!signedBytes && !recoveryBytes.trim())}>{busy ? messages.wallet.sending : messages.wallet.send}</button>
          </div>}
          {(claim.status === "unknown" || claim.status === "submitted" || claim.status === "processed" || claim.status === "confirmed") && <div className="evidence-actions"><p className="inline-warning">{messages.evidence.reconcileWarning}</p><button type="button" onClick={reconcile} disabled={busy || !serviceOnline || !accessToken.trim()}>{messages.evidence.reconcile}</button></div>}
          {claim.last_error_code && <p className="inline-error" role="alert">{messages.evidence.lastError} {claim.last_error_code}</p>}
        </>}
      </article>
    </section>
  );
}
