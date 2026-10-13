import { FormEvent, useState } from "react";
import { PublicKey } from "@solana/web3.js";
import { fetchWithTimeout, operatorResponseError, REQUEST_TIMEOUT_MS } from "./api";
import { createClientUuid } from "./clientIdentity";
import {
  parseRefundCancellationResponse,
  parseRefundCancellationSubmissionResponse,
  refundCancellationHasSignature,
  signRefundCancellation,
  type RefundCancellationResponse,
} from "./refundCancellation";
import WalletPicker from "./WalletPicker";
import { refundCancellationMessagesFor, type RefundCancellationMessages } from "./refundCancellationMessages";
import { operatorErrorMessage } from "./operatorError";
import { shellMessagesFor } from "./shellMessages";

type RefundCancellationPanelProps = {
  apiBase: string;
  serviceOnline: boolean;
  accessToken: string;
  locale: string;
};

function digestPreview(bytes: number[]): string {
  return `${bytes.slice(0, 8).map((byte) => byte.toString(16).padStart(2, "0")).join("")}…`;
}

function statusLabel(status: string, messages: RefundCancellationMessages): string {
  return messages.status.labels[status] ?? messages.status.labels.unknown;
}

export default function RefundCancellationPanel({ apiBase, serviceOnline, accessToken, locale }: RefundCancellationPanelProps) {
  const messages = refundCancellationMessagesFor(locale);
  const shellMessages = shellMessagesFor(locale);
  const workflowLocked = !serviceOnline || !accessToken.trim();
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
    if (!token) throw new Error(messages.errors.auth);
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
      if (!claimIntentId.trim()) throw new Error(messages.errors.finalizedRequest);
      const response = await fetchWithTimeout(`${apiBase.replace(/\/$/, "")}/v1/settlement/refund-claim-cancellations`, {
        method: "POST",
        headers: { "content-type": "application/json", authorization: `Bearer ${token}` },
        body: JSON.stringify({ idempotency_key: idempotencyKey.trim(), claim_intent_id: claimIntentId.trim(), confirm: true }),
      });
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw operatorResponseError(response, payload, messages.errors.httpError(messages.errors.prepare, response.status));
      const parsed = parseRefundCancellationResponse(payload);
      setCancellation(parsed);
      setLookupId(parsed.intent_id);
      setSignedBytes(null);
      setRecoveryBytes("");
      setWalletAddress(null);
      setNotice(messages.notices.prepared);
    } catch (cause) {
      setError(operatorErrorMessage(cause, messages.errors.prepare));
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
      if (!lookupId.trim()) throw new Error(messages.errors.cancellationReference);
      const response = await fetchWithTimeout(`${apiBase.replace(/\/$/, "")}/v1/settlement/refund-claim-cancellations/${lookupId.trim()}`, {
        headers: { authorization: `Bearer ${token}` },
      });
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw operatorResponseError(response, payload, messages.errors.httpError(messages.errors.recovery, response.status));
      setCancellation(parseRefundCancellationResponse(payload));
      setSignedBytes(null);
      setRecoveryBytes("");
      setWalletAddress(null);
      setNotice(messages.notices.loaded);
    } catch (cause) {
      setError(operatorErrorMessage(cause, messages.errors.recovery));
    } finally {
      setBusy(false);
    }
  }

  async function sign() {
    setBusy(true);
    clearMessages();
    try {
      if (!cancellation) throw new Error(messages.errors.prepareBeforeSign);
      if (cancellation.status !== "prepared_unsubmitted" && cancellation.status !== "signed") throw new Error(messages.errors.signableState);
      const expectedAuthority = new PublicKey(cancellation.artifact.governance_authority);
      const serialized = signedBytes ?? (recoveryBytes.trim() || null);
      const next = await signRefundCancellation(cancellation.artifact.instruction, expectedAuthority, import.meta.env.VITE_SOLANA_RPC_URL ?? "", serialized);
      setSignedBytes(next);
      setRecoveryBytes("");
      setWalletAddress(expectedAuthority);
      setNotice(refundCancellationHasSignature(next)
        ? messages.notices.signed
        : messages.notices.noSignature);
    } catch (cause) {
      setError(operatorErrorMessage(cause, messages.errors.sign));
    } finally {
      setBusy(false);
    }
  }

  async function submit() {
    setBusy(true);
    clearMessages();
    try {
      const token = requireToken();
      if (!cancellation) throw new Error(messages.errors.prepareBeforeSign);
      const bytes = signedBytes ?? recoveryBytes.trim();
      if (!bytes || !refundCancellationHasSignature(bytes)) throw new Error(messages.errors.signatureRequired);
      const response = await fetchWithTimeout(`${apiBase.replace(/\/$/, "")}/v1/settlement/refund-claim-cancellations/submit`, {
        method: "POST",
        headers: { "content-type": "application/json", authorization: `Bearer ${token}` },
        body: JSON.stringify({ intent_id: cancellation.intent_id, signed_transaction_base64: bytes }),
      }, REQUEST_TIMEOUT_MS.settlementSubmit);
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw operatorResponseError(response, payload, messages.errors.httpError(messages.errors.submit, response.status));
      const result = parseRefundCancellationSubmissionResponse(payload);
      setCancellation({ ...cancellation, status: result.status as RefundCancellationResponse["status"], signature: result.signature ?? null });
      setNotice(result.status === "unknown"
        ? messages.notices.unknownSubmission
        : messages.notices.submitted);
    } catch (cause) {
      setError(operatorErrorMessage(cause, messages.errors.submit));
    } finally {
      setBusy(false);
    }
  }

  async function reconcile() {
    setBusy(true);
    clearMessages();
    try {
      const token = requireToken();
      if (!cancellation) throw new Error(messages.errors.prepareBeforeSign);
      const response = await fetchWithTimeout(`${apiBase.replace(/\/$/, "")}/v1/settlement/refund-claim-cancellations/${cancellation.intent_id}/reconcile`, {
        method: "POST",
        headers: { authorization: `Bearer ${token}` },
      }, REQUEST_TIMEOUT_MS.settlementSubmit);
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw operatorResponseError(response, payload, messages.errors.httpError(messages.errors.reconcile, response.status));
      setCancellation(parseRefundCancellationResponse(payload));
      setNotice(messages.notices.reconciled);
    } catch (cause) {
      setError(operatorErrorMessage(cause, messages.errors.reconcile));
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="adjustment-layout" aria-label={messages.proposal.ariaLabel}>
      <article className="panel">
        <div className="panel-heading"><div><p className="eyebrow">{messages.proposal.eyebrow}</p><h2>{messages.proposal.title}</h2></div><span className="status-pill">{messages.proposal.badge}</span></div>
        <p className="panel-copy">{messages.proposal.description}</p>
        {!accessToken.trim() && <div className="workflow-lock" role="note"><strong>{shellMessages.session.signInToContinue}</strong><p>{shellMessages.session.signInToContinueCopy}</p></div>}
        <fieldset className="workflow-fieldset" disabled={workflowLocked || busy}>
        <legend className="sr-only">{messages.proposal.title}</legend>
        <form className="form-grid" onSubmit={prepare}>
          <label className="full-field"><span>{messages.proposal.finalizedRequest}</span><input value={claimIntentId} onChange={(event) => setClaimIntentId(event.target.value)} placeholder={messages.proposal.finalizedPlaceholder} required /></label>
          <label className="full-field"><span>{messages.proposal.requestReference}</span><input value={idempotencyKey} onChange={(event) => setIdempotencyKey(event.target.value)} aria-describedby="cancellation-reference-help" required /><small id="cancellation-reference-help">{messages.proposal.requestHelp}</small></label>
          <button type="submit" disabled={busy || !serviceOnline || !accessToken.trim()}>{busy ? messages.proposal.preparing : messages.proposal.prepare}</button>
        </form>
        <form className="form-grid" onSubmit={load}>
          <label className="full-field"><span>{messages.proposal.cancellationReference}</span><input value={lookupId} onChange={(event) => setLookupId(event.target.value)} placeholder={messages.proposal.cancellationPlaceholder} /></label>
          <button type="submit" disabled={busy || !serviceOnline || !accessToken.trim()}>{messages.proposal.load}</button>
        </form>
        </fieldset>
        {!serviceOnline && <p className="inline-warning" role="alert">{messages.proposal.serviceUnavailable}</p>}
        {error && <p className="inline-error" role="alert">{error}</p>}
        {notice && <p className="inline-success" role="status">{notice}</p>}
      </article>

      <article className="panel" aria-live="polite" aria-label={messages.review.ariaLabel}>
        <div className="panel-heading"><div><p className="eyebrow">{messages.review.eyebrow}</p><h2>{messages.review.title}</h2></div>{cancellation && <span className="status-pill">{statusLabel(cancellation.status, messages)}</span>}</div>
        <WalletPicker locale={locale} disabled={busy || workflowLocked} connectEnabled={Boolean(cancellation && (cancellation.status === "prepared_unsubmitted" || cancellation.status === "signed"))} connectedAddress={walletAddress?.toBase58()} onConnected={setWalletAddress} />
        {!cancellation ? <div className="empty-state"><span className="empty-icon">⊘</span><strong>{messages.review.noCancellation}</strong><p>{messages.review.noCancellationCopy}</p></div> : <>
          <dl className="summary-list">
            <div><dt>{messages.review.onChainRecord}</dt><dd><code>{cancellation.claim}</code></dd></div>
            <div><dt>{messages.review.approvalAuthority}</dt><dd><code>{cancellation.artifact.governance_authority}</code></dd></div>
            <div><dt>{messages.review.merchant}</dt><dd><code>{cancellation.artifact.merchant}</code></dd></div>
            <div><dt>{messages.review.asset}</dt><dd><code>{cancellation.artifact.mint}</code></dd></div>
            <div><dt>{messages.review.adjustmentRecord}</dt><dd><code>{cancellation.artifact.adjustment}</code></dd></div>
            <div><dt>{messages.review.releaseReferenceNumber}</dt><dd>{cancellation.artifact.instruction.claim_nonce}</dd></div>
            <div><dt>{messages.review.ledgerRegistry}</dt><dd><code>{cancellation.artifact.instruction.registry}</code></dd></div>
            <div><dt>{messages.review.instructionEvidence}</dt><dd><code>{digestPreview(cancellation.instruction_digest)}</code></dd></div>
            <div><dt>{messages.review.requestEvidence}</dt><dd><code>{digestPreview(cancellation.artifact_digest)}</code></dd></div>
            <div><dt>{messages.review.submissionReference}</dt><dd>{cancellation.signature ? <code>{cancellation.signature}</code> : messages.review.noneRecorded}</dd></div>
          </dl>
          <div className="settlement-status">
            <strong>{messages.review.noFundsMovedTitle}</strong>
            <small>{messages.review.noFundsMovedCopy}</small>
          </div>
          {cancellation.status === "finalized" && cancellation.finalized_evidence && <div className="settlement-status" aria-live="polite">
            <strong role="status">{messages.review.finalizedTitle}</strong>
            <small>{messages.review.finalizedCopy}</small>
            <details>
              <summary>{messages.review.finalizedProof}</summary>
              <dl className="summary-list">
                <div><dt>{messages.review.event}</dt><dd>{cancellation.finalized_evidence.event_type} · {cancellation.finalized_evidence.event_version}</dd></div>
                <div><dt>{messages.review.finalizedSlot}</dt><dd>{cancellation.finalized_evidence.slot}</dd></div>
                <div><dt>{messages.review.eventIndex}</dt><dd>{cancellation.finalized_evidence.event_index}</dd></div>
                <div><dt>{messages.review.eventSignature}</dt><dd><code>{cancellation.finalized_evidence.signature}</code></dd></div>
                <div><dt>{messages.review.evidenceDigest}</dt><dd><code>{digestPreview(cancellation.finalized_evidence.evidence_digest)}</code></dd></div>
              </dl>
            </details>
          </div>}
          {(cancellation.status === "prepared_unsubmitted" || cancellation.status === "signed") && <div className="evidence-actions">
            <p className="evidence-note">{messages.wallet.signNote}</p>
            <div className="button-row"><button type="button" onClick={sign} disabled={busy || workflowLocked}>{busy ? messages.wallet.sending : messages.wallet.sign}</button><button type="button" onClick={submit} disabled={busy || workflowLocked || (!signedBytes && !recoveryBytes.trim())}>{messages.wallet.send}</button></div>
            <label className="full-field"><span>{messages.wallet.recovery}</span><textarea value={recoveryBytes} onChange={(event) => setRecoveryBytes(event.target.value)} aria-describedby="cancellation-recovery-help" placeholder={messages.wallet.recoveryPlaceholder} rows={3} autoComplete="off" spellCheck={false} /><small id="cancellation-recovery-help">{messages.wallet.recoveryHelp}</small></label>
          </div>}
          {(cancellation.status === "unknown" || cancellation.status === "submitted" || cancellation.status === "processed" || cancellation.status === "confirmed") && <div className="evidence-actions"><p className="inline-warning" role="alert">{messages.evidence.providerNotProof}</p><button type="button" onClick={reconcile} disabled={busy || !serviceOnline || !accessToken.trim()}>{messages.evidence.reconcile}</button></div>}
          {cancellation.status === "finalized" && !cancellation.finalized_evidence && <p className="inline-warning" role="alert">{messages.status.finalizedIncomplete}</p>}
          {cancellation.last_error_code && <p className="inline-error" role="alert">{messages.status.lastError} {cancellation.last_error_code}</p>}
        </>}
      </article>
    </section>
  );
}
