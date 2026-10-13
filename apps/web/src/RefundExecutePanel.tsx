import { FormEvent, useState } from "react";
import { PublicKey } from "@solana/web3.js";
import { fetchWithTimeout, operatorResponseError, REQUEST_TIMEOUT_MS } from "./api";
import { createClientUuid } from "./clientIdentity";
import { formatInteger } from "./formatters";
import WalletPicker from "./WalletPicker";
import { refundExecuteMessagesFor } from "./refundExecuteMessages";
import { formatProtocolStatus } from "./protocolStatus";
import { operatorErrorMessage } from "./operatorError";
import {
  RefundExecuteResponse,
  parseRefundExecuteResponse,
  parseRefundExecuteSubmissionResponse,
  refundExecuteHasAllSignatures,
  signRefundExecuteRole,
} from "./refundExecutions";
import { shellMessagesFor } from "./shellMessages";

type RefundExecutePanelProps = {
  apiBase: string;
  serviceOnline: boolean;
  accessToken: string;
  locale: string;
};

function amountValue(value: string | null, locale: string): string {
  return value === null ? "Not present" : formatInteger(value, locale);
}

export default function RefundExecutePanel({ apiBase, serviceOnline, accessToken, locale }: RefundExecutePanelProps) {
  const messages = refundExecuteMessagesFor(locale);
  const shellMessages = shellMessagesFor(locale);
  const workflowLocked = !serviceOnline || !accessToken.trim();
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
      if (!claimIntentId.trim()) throw new Error(messages.errors.claimReferenceRequired);
      const response = await fetchWithTimeout(`${apiBase.replace(/\/$/, "")}/v1/settlement/refund-executions`, {
        method: "POST",
        headers: { "content-type": "application/json", authorization: `Bearer ${token}` },
        body: JSON.stringify({ idempotency_key: idempotencyKey.trim(), claim_intent_id: claimIntentId.trim(), confirm: true }),
      });
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw operatorResponseError(response, payload, messages.errors.preparationRequestFailed(response.status));
      const parsed = parseRefundExecuteResponse(payload);
      setExecute(parsed);
      setLookupId(parsed.intent_id);
      setSignedBytes(null);
      setRecoveryBytes("");
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
      if (!lookupId.trim()) throw new Error(messages.errors.releaseReferenceRequired);
      const response = await fetchWithTimeout(`${apiBase.replace(/\/$/, "")}/v1/settlement/refund-executions/${lookupId.trim()}`, { headers: { authorization: `Bearer ${token}` } });
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw operatorResponseError(response, payload, messages.errors.recoveryRequestFailed(response.status));
      setExecute(parseRefundExecuteResponse(payload));
      setSignedBytes(null);
      setRecoveryBytes("");
      setNotice(messages.errors.recoveryLoaded);
    } catch (cause) {
      setError(operatorErrorMessage(cause, messages.errors.recoveryFailed));
    } finally {
      setBusy(false);
    }
  }

  async function sign(role: "approver" | "refund_funder") {
    setBusy(true);
    clearMessages();
    try {
      if (!execute) throw new Error(messages.errors.prepareBeforeSigning);
      if (execute.status !== "prepared_unsubmitted" && execute.status !== "signed") throw new Error(messages.errors.signableStateRequired);
      const serialized = signedBytes ?? (recoveryBytes.trim() || null);
      if (role === "refund_funder" && !serialized) throw new Error(messages.errors.funderPrerequisite);
      const expected = new PublicKey(role === "approver" ? execute.artifact.instruction.accounts[0].pubkey : execute.artifact.refund_funder);
      const next = await signRefundExecuteRole(execute.artifact.instruction, role, expected, import.meta.env.VITE_SOLANA_RPC_URL ?? "", serialized);
      setSignedBytes(next);
      setRecoveryBytes("");
      setNotice(refundExecuteHasAllSignatures(next)
        ? messages.errors.bothSignaturesPresent
        : role === "approver"
          ? messages.errors.approverSignaturePresent
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
      if (!execute) throw new Error(messages.errors.prepareBeforeSubmission);
      const bytes = signedBytes ?? recoveryBytes.trim();
      if (!bytes || !refundExecuteHasAllSignatures(bytes)) throw new Error(messages.errors.signaturesRequired);
      const response = await fetchWithTimeout(`${apiBase.replace(/\/$/, "")}/v1/settlement/refund-executions/submit`, {
        method: "POST",
        headers: { "content-type": "application/json", authorization: `Bearer ${token}` },
        body: JSON.stringify({ intent_id: execute.intent_id, signed_transaction_base64: bytes }),
      }, REQUEST_TIMEOUT_MS.settlementSubmit);
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw operatorResponseError(response, payload, messages.errors.submissionRequestFailed(response.status));
      const result = parseRefundExecuteSubmissionResponse(payload);
      setExecute({ ...execute, status: result.status, signature: result.signature });
      setNotice(result.status === "unknown"
        ? messages.errors.submissionUnknown
        : messages.errors.submissionPending);
    } catch (cause) {
      setError(operatorErrorMessage(cause, messages.errors.submissionFailed));
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
          <label className="full-field"><span>{messages.proposal.finalizedRequest}</span><input aria-label={messages.proposal.finalizedRequest} value={claimIntentId} onChange={(event) => setClaimIntentId(event.target.value)} placeholder={messages.proposal.finalizedPlaceholder} required /></label>
          <label className="full-field"><span>{messages.proposal.requestReference}</span><input value={idempotencyKey} onChange={(event) => setIdempotencyKey(event.target.value)} aria-describedby="refund-release-reference-help" required /><small id="refund-release-reference-help">{messages.proposal.requestHelp}</small></label>
          <button type="submit" disabled={busy || !serviceOnline || !accessToken.trim()}>{busy ? messages.proposal.preparing : messages.proposal.prepare}</button>
        </form>
        <form className="form-grid" onSubmit={load}>
          <label className="full-field"><span>{messages.proposal.releaseReference}</span><input aria-label={messages.proposal.releaseReference} value={lookupId} onChange={(event) => setLookupId(event.target.value)} placeholder={messages.proposal.releasePlaceholder} /></label>
          <button type="submit" disabled={busy || !serviceOnline || !accessToken.trim()}>{messages.proposal.load}</button>
        </form>
        </fieldset>
        {!serviceOnline && <p className="inline-warning" role="alert">{messages.proposal.serviceUnavailable}</p>}
        {error && <p className="inline-error" role="alert">{error}</p>}
        {notice && <p className="inline-success" role="status">{notice}</p>}
      </article>

      <article className="panel" aria-label={messages.review.ariaLabel} aria-live="polite">
        <div className="panel-heading"><div><p className="eyebrow">{messages.review.eyebrow}</p><h2>{messages.review.title}</h2></div>{execute && <span className="status-pill">{formatProtocolStatus(execute.status, locale)}</span>}</div>
        <WalletPicker
          disabled={busy || workflowLocked}
          connectEnabled={Boolean(execute && (execute.status === "prepared_unsubmitted" || execute.status === "signed"))}
          connectedAddress={walletAddress?.toBase58()}
          onConnected={setWalletAddress}
          locale={locale}
        />
        {!execute ? <div className="empty-state"><span className="empty-icon">↗</span><strong>{messages.review.noRelease}</strong><p>{messages.review.noReleaseCopy}</p></div> : <>
          <dl className="summary-list">
            <div><dt>{messages.review.onChainRecord}</dt><dd><code>{execute.claim}</code></dd></div>
            <div><dt>{messages.review.confirmedRequest}</dt><dd><code>{execute.artifact.claim_signature}</code></dd></div>
            <div><dt>{messages.review.grossRefund}</dt><dd>{amountValue(execute.artifact.gross_atomic, locale)}</dd></div>
            <div><dt>{messages.review.taxIncluded}</dt><dd>{amountValue(execute.artifact.tax_atomic, locale)}</dd></div>
            <div><dt>{messages.review.refundAfterTax}</dt><dd>{amountValue(execute.artifact.net_atomic, locale)}</dd></div>
            <div><dt>{messages.review.asset}</dt><dd><code>{execute.artifact.mint}</code></dd></div>
            <div><dt>{messages.review.destination}</dt><dd><code>{execute.artifact.refund_destination}</code></dd></div>
            <div><dt>{messages.review.finalApprover}</dt><dd><code>{execute.artifact.instruction.accounts[0].pubkey}</code></dd></div>
            <div><dt>{messages.review.refundAccount}</dt><dd><code>{execute.artifact.refund_funder}</code></dd></div>
            <div><dt>{messages.review.submissionReference}</dt><dd>{execute.signature ? <code>{execute.signature}</code> : messages.review.noneRecorded}</dd></div>
          </dl>
          {(execute.status === "prepared_unsubmitted" || execute.status === "signed") && <div className="evidence-actions">
            <p className="evidence-note">{messages.wallet.twoPersonNote}</p>
            <div className="button-row"><button type="button" onClick={() => sign("approver")} disabled={busy || workflowLocked}>{messages.wallet.signApprover}</button><button type="button" onClick={() => sign("refund_funder")} disabled={busy || workflowLocked || (!signedBytes && !recoveryBytes.trim())}>{messages.wallet.signFunder}</button></div>
            <label className="full-field"><span>{messages.wallet.recovery}</span><textarea aria-label={messages.wallet.recovery} value={recoveryBytes} onChange={(event) => setRecoveryBytes(event.target.value)} placeholder={messages.wallet.recoveryPlaceholder} aria-describedby="refund-release-recovery-help" rows={3} autoComplete="off" spellCheck={false} /><small id="refund-release-recovery-help">{messages.wallet.recoveryHelp}</small></label>
            <button type="button" onClick={submit} disabled={busy || workflowLocked || (!signedBytes && !recoveryBytes.trim())}>{busy ? messages.wallet.sending : messages.wallet.send}</button>
          </div>}
          {(execute.status === "unknown" || execute.status === "submitted" || execute.status === "processed" || execute.status === "confirmed") && <p className="inline-warning">{messages.status.proofPending}</p>}
          {execute.status === "finalized" && <p className="inline-success" role="status">{messages.status.finalized}</p>}
          {execute.last_error_code && <p className="inline-error" role="alert">{messages.status.lastError} {execute.last_error_code}</p>}
        </>}
      </article>
    </section>
  );
}
