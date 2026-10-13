import { FormEvent, useMemo, useState } from "react";
import { PublicKey } from "@solana/web3.js";
import { fetchWithTimeout, operatorResponseError, REQUEST_TIMEOUT_MS } from "./api";
import { createClientUuid } from "./clientIdentity";
import {
  parseCompiledSettlementAdjustmentArtifact,
  parseSettlementAdjustmentChainIntentResponse,
  signAndEncodeSettlementAdjustment,
} from "./checkout";
import {
  AdjustmentKind,
  AdjustmentStatus,
  AdjustmentChainIntentResponse,
  CompiledSettlementAdjustmentArtifact,
  SettlementAdjustment,
  buildSettlementAdjustmentUrl,
  nextAdjustmentStatuses,
  parseSettlementAdjustment,
  parseSettlementAdjustmentTransition,
  validateAdjustmentDelta,
  validateByteArray,
  validateUuidV7,
} from "./settlementAdjustments";
import WalletPicker from "./WalletPicker";
import { ensureConnectedWallet } from "./wallet";
import { DEFAULT_LOCALE } from "./formatters";
import { settlementAdjustmentMessagesFor, type SettlementAdjustmentMessages } from "./settlementAdjustmentMessages";
import { operatorErrorMessage, operatorFacingError } from "./operatorError";
import { shellMessagesFor } from "./shellMessages";

type SettlementAdjustmentPanelProps = {
  apiBase: string;
  serviceOnline: boolean;
  accessToken: string;
  locale?: string;
};

const KINDS: AdjustmentKind[] = ["Refund", "Reversal", "Correction", "Chargeback", "RemittanceAdjustment"];

function digestPreview(value: number[]): string {
  return value.map((byte) => byte.toString(16).padStart(2, "0")).join("").slice(0, 16) + "…";
}

function parseBytes(value: string, label: string, invalidJsonMessage: string): number[] {
  let parsed: unknown;
  try {
    parsed = JSON.parse(value);
  } catch {
    throw operatorFacingError(invalidJsonMessage);
  }
  return validateByteArray(parsed, label, true);
}

function statusLabel(status: string, messages: SettlementAdjustmentMessages): string {
  return messages.status.labels[status] ?? messages.status.labels.unknown;
}

export default function SettlementAdjustmentPanel({ apiBase, serviceOnline, accessToken, locale = DEFAULT_LOCALE }: SettlementAdjustmentPanelProps) {
  const messages = settlementAdjustmentMessagesFor(locale);
  const shellMessages = shellMessagesFor(locale);
  const workflowLocked = !serviceOnline || !accessToken.trim();
  const [idempotencyKey, setIdempotencyKey] = useState(() => createClientUuid());
  const [originalSettlementId, setOriginalSettlementId] = useState("");
  const [assetId, setAssetId] = useState("");
  const [kind, setKind] = useState<AdjustmentKind>("Refund");
  const [grossAtomic, setGrossAtomic] = useState("");
  const [taxAtomic, setTaxAtomic] = useState("");
  const [netAtomic, setNetAtomic] = useState("");
  const [reasonCode, setReasonCode] = useState("");
  const [evidenceDigest, setEvidenceDigest] = useState("");
  const [sourceCalculationId, setSourceCalculationId] = useState("");
  const [replacementCalculationId, setReplacementCalculationId] = useState("");
  const [lookupId, setLookupId] = useState("");
  const [record, setRecord] = useState<SettlementAdjustment | null>(null);
  const [transitionStatus, setTransitionStatus] = useState<"approved" | "rejected" | "voided">("approved");
  const [transitionEvidence, setTransitionEvidence] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [walletAddress, setWalletAddress] = useState<PublicKey | null>(null);
  const [chainArtifact, setChainArtifact] = useState<CompiledSettlementAdjustmentArtifact | null>(null);
  const [chainStatus, setChainStatus] = useState<AdjustmentChainIntentResponse | null>(null);

  const availableTransitions = useMemo(() => nextAdjustmentStatuses(record?.status ?? ""), [record?.status]);

  function requireToken(): string {
    const token = accessToken.trim();
    if (!token) throw operatorFacingError(messages.errors.auth);
    return token;
  }

  function parseEvidence(value: string): Record<string, unknown> {
    let parsed: unknown;
    try {
      parsed = JSON.parse(value);
    } catch {
      throw operatorFacingError(messages.errors.evidenceJsonInvalid);
    }
    if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) throw operatorFacingError(messages.errors.evidenceObjectRequired);
    return parsed as Record<string, unknown>;
  }

  async function propose(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      const token = requireToken();
      const parsedAsset = parseBytes(assetId, "asset_id", messages.errors.bytesJsonInvalid);
      const parsedEvidenceDigest = parseBytes(evidenceDigest, "evidence_digest", messages.errors.bytesJsonInvalid);
      validateUuidV7(originalSettlementId.trim(), "original_settlement_id");
      validateUuidV7(sourceCalculationId.trim(), "source_calculation_id");
      if (replacementCalculationId.trim()) validateUuidV7(replacementCalculationId.trim(), "replacement_calculation_id");
      validateAdjustmentDelta(kind, { gross_atomic: grossAtomic.trim(), tax_atomic: taxAtomic.trim(), net_atomic: netAtomic.trim() });
      if (!reasonCode.trim() || reasonCode.trim().length > 256) throw operatorFacingError(messages.errors.reasonCodeInvalid);
      if (!idempotencyKey.trim()) throw operatorFacingError(messages.errors.idempotencyKeyRequired);
      const response = await fetchWithTimeout(`${apiBase.replace(/\/$/, "")}/v1/settlement/adjustments`, {
        method: "POST",
        headers: { "content-type": "application/json", authorization: `Bearer ${token}` },
        body: JSON.stringify({
          idempotency_key: idempotencyKey.trim(),
          original_settlement_id: originalSettlementId.trim(),
          asset_id: parsedAsset,
          kind,
          delta: { gross_atomic: grossAtomic.trim(), tax_atomic: taxAtomic.trim(), net_atomic: netAtomic.trim() },
          reason_code: reasonCode.trim(),
          evidence_digest: parsedEvidenceDigest,
          source_calculation_id: sourceCalculationId.trim(),
          replacement_calculation_id: replacementCalculationId.trim() || null,
        }),
      });
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw operatorResponseError(response, payload, messages.errors.httpError(messages.errors.proposal, response.status));
      const loaded = parseSettlementAdjustment(payload);
      setRecord(loaded);
      setChainArtifact(null);
      setChainStatus(null);
      setLookupId(loaded.id);
      setNotice(messages.notices.proposed(loaded.id, statusLabel(loaded.status, messages)));
    } catch (cause) {
      setError(operatorErrorMessage(cause, messages.errors.proposal));
    } finally {
      setBusy(false);
    }
  }

  async function load(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      const token = requireToken();
      const id = validateUuidV7(lookupId.trim(), "adjustment_id");
      const response = await fetchWithTimeout(buildSettlementAdjustmentUrl(apiBase, id), { headers: { authorization: `Bearer ${token}` } });
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw operatorResponseError(response, payload, messages.errors.httpError(messages.errors.lookup, response.status));
      setRecord(parseSettlementAdjustment(payload));
      setChainArtifact(null);
      setChainStatus(null);
      setNotice(messages.notices.loaded);
    } catch (cause) {
      setError(operatorErrorMessage(cause, messages.errors.lookup));
    } finally {
      setBusy(false);
    }
  }

  async function transition(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      const token = requireToken();
      if (!record) throw operatorFacingError(messages.review.noAdjustment);
      if (!availableTransitions.includes(transitionStatus)) throw operatorFacingError(messages.review.noTransition);
      const evidence = parseEvidence(transitionEvidence);
      const response = await fetchWithTimeout(`${buildSettlementAdjustmentUrl(apiBase, record.id)}/transition`, {
        method: "POST",
        headers: { "content-type": "application/json", authorization: `Bearer ${token}` },
        body: JSON.stringify({ to_status: transitionStatus, evidence }),
      });
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw operatorResponseError(response, payload, messages.errors.httpError(messages.errors.transition, response.status));
      const transitionResponse = parseSettlementAdjustmentTransition(payload);
      const reload = await fetchWithTimeout(buildSettlementAdjustmentUrl(apiBase, transitionResponse.adjustment_id), { headers: { authorization: `Bearer ${token}` } });
      const reloadPayload: unknown = await reload.json().catch(() => null);
      if (!reload.ok) throw operatorResponseError(reload, reloadPayload, messages.errors.httpError(messages.errors.lookup, reload.status));
      const refreshed = parseSettlementAdjustment(reloadPayload);
      setRecord(refreshed);
      setChainArtifact(null);
      setChainStatus(null);
      setNotice(messages.notices.transitionAccepted(statusLabel(transitionResponse.to_status, messages)));
    } catch (cause) {
      setError(operatorErrorMessage(cause, messages.errors.transition));
    } finally {
      setBusy(false);
    }
  }

  async function compileChainEvidence() {
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      const token = requireToken();
      if (!record || record.status !== "approved") throw operatorFacingError(messages.review.noTransition);
      const response = await fetchWithTimeout(`${buildSettlementAdjustmentUrl(apiBase, record.id)}/compile`, {
        method: "POST",
        headers: { "content-type": "application/json", authorization: `Bearer ${token}` },
        body: JSON.stringify({ confirm: true }),
      }, REQUEST_TIMEOUT_MS.default);
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw operatorResponseError(response, payload, messages.errors.httpError(messages.errors.compile, response.status));
      const artifact = parseCompiledSettlementAdjustmentArtifact(payload);
      setChainArtifact(artifact);
      setChainStatus({
        chain_intent_id: artifact.chain_intent_id,
        status: artifact.status,
        signature: null,
        provider_index: null,
        finalized_evidence: null,
      });
      setNotice(messages.notices.readyForWallet);
    } catch (cause) {
      setError(operatorErrorMessage(cause, messages.errors.compile));
    } finally {
      setBusy(false);
    }
  }

  async function refreshChainStatus() {
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      const token = requireToken();
      if (!chainArtifact) throw operatorFacingError(messages.chain.prepare);
      const response = await fetchWithTimeout(`${apiBase}/v1/settlement/adjustment-chain-intents/${chainArtifact.chain_intent_id}`, {
        headers: { authorization: `Bearer ${token}` },
      });
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw operatorResponseError(response, payload, messages.errors.httpError(messages.errors.refresh, response.status));
      const status = parseSettlementAdjustmentChainIntentResponse(payload);
      setChainStatus(status);
      setNotice(messages.notices.durableStatus(statusLabel(status.status, messages)));
    } catch (cause) {
      setError(operatorErrorMessage(cause, messages.errors.refresh));
    } finally {
      setBusy(false);
    }
  }

  async function signAndSubmitChainEvidence() {
    setBusy(true);
    setError(null);
    setNotice(null);
    let submissionAttempted = false;
    try {
      const token = requireToken();
      if (!chainArtifact) throw operatorFacingError(messages.chain.prepare);
      const connected = (await ensureConnectedWallet()).publicKey;
      setWalletAddress(connected);
      const signedTransactionBase64 = await signAndEncodeSettlementAdjustment(
        chainArtifact.instruction,
        connected,
        import.meta.env.VITE_SOLANA_RPC_URL ?? "",
      );
      submissionAttempted = true;
      const response = await fetchWithTimeout(`${apiBase.replace(/\/$/, "")}/v1/settlement/adjustment-chain-intents/submit`, {
        method: "POST",
        headers: { "content-type": "application/json", authorization: `Bearer ${token}` },
        body: JSON.stringify({
          chain_intent_id: chainArtifact.chain_intent_id,
          signed_transaction_base64: signedTransactionBase64,
          instruction: chainArtifact.instruction,
        }),
      }, REQUEST_TIMEOUT_MS.settlementSubmit);
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw operatorResponseError(response, payload, messages.errors.httpError(messages.errors.sign, response.status));
      const status = parseSettlementAdjustmentChainIntentResponse(payload);
      setChainStatus(status);
      setNotice(status.status === "finalized"
        ? messages.notices.submissionFinalized
        : messages.notices.submissionAccepted(statusLabel(status.status, messages)));
    } catch (cause) {
      if (submissionAttempted) {
        try {
          await refreshChainStatus();
          setError(messages.errors.sign);
        } catch {
          setError(messages.status.providerAmbiguous);
        }
      } else {
        setError(operatorErrorMessage(cause, messages.errors.sign));
      }
    } finally {
      setBusy(false);
    }
  }

  async function reconcileChainEvidence() {
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      const token = requireToken();
      if (!chainArtifact) throw operatorFacingError(messages.chain.prepare);
      if (!chainStatus?.signature) throw operatorFacingError(messages.chain.noSignature);
      const response = await fetchWithTimeout(`${apiBase}/v1/settlement/adjustment-chain-intents/${chainArtifact.chain_intent_id}/reconcile`, {
        method: "POST",
        headers: { authorization: `Bearer ${token}` },
      }, REQUEST_TIMEOUT_MS.settlementSubmit);
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw operatorResponseError(response, payload, messages.errors.httpError(messages.errors.reconcile, response.status));
      const status = parseSettlementAdjustmentChainIntentResponse(payload);
      setChainStatus(status);
      setNotice(status.status === "finalized"
        ? messages.notices.reconciliationFinalized
        : messages.notices.reconciliationPending(statusLabel(status.status, messages)));
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
        <form className="form-grid" onSubmit={propose}>
          <div className="workflow-status full-field" role="status" aria-describedby="adjustment-token-help">
            <div className="workflow-status-heading"><span>{messages.proposal.signInStatus}</span><strong>{accessToken ? messages.proposal.accountReady : shellMessages.session.signInToContinue}</strong></div>
            <small id="adjustment-token-help">{messages.proposal.accountHelp}</small>
          </div>
          <label className="full-field"><span>{messages.proposal.requestReference}</span><input value={idempotencyKey} onChange={(event) => setIdempotencyKey(event.target.value)} aria-describedby="adjustment-reference-help" required /><small id="adjustment-reference-help">{messages.proposal.requestHelp}</small></label>
          <label><span>{messages.proposal.correctionType}</span><select value={kind} onChange={(event) => setKind(event.target.value as AdjustmentKind)}>{KINDS.map((option) => <option key={option} value={option}>{option}</option>)}</select></label>
          <label><span>{messages.proposal.reason}</span><input value={reasonCode} onChange={(event) => setReasonCode(event.target.value)} maxLength={256} required /></label>
          <label className="full-field"><span>{messages.proposal.originalPayment}</span><input value={originalSettlementId} onChange={(event) => setOriginalSettlementId(event.target.value)} required /></label>
          <label><span>{messages.proposal.gross}</span><input inputMode="numeric" value={grossAtomic} onChange={(event) => setGrossAtomic(event.target.value)} aria-describedby="adjustment-amount-help" required /></label>
          <label><span>{messages.proposal.tax}</span><input inputMode="numeric" value={taxAtomic} onChange={(event) => setTaxAtomic(event.target.value)} aria-describedby="adjustment-amount-help" required /></label>
          <label><span>{messages.proposal.net}</span><input inputMode="numeric" value={netAtomic} onChange={(event) => setNetAtomic(event.target.value)} aria-describedby="adjustment-amount-help" required /></label>
          <small id="adjustment-amount-help" className="field-help full-field">{messages.proposal.amountHelp}</small>
          <label className="full-field"><span>{messages.proposal.asset}</span><textarea value={assetId} onChange={(event) => setAssetId(event.target.value)} rows={2} required /></label>
          <label className="full-field"><span>{messages.proposal.supportingEvidence}</span><textarea value={evidenceDigest} onChange={(event) => setEvidenceDigest(event.target.value)} aria-describedby="adjustment-evidence-help" rows={2} required /><small id="adjustment-evidence-help">{messages.proposal.evidenceHelp}</small></label>
          <label><span>{messages.proposal.source}</span><input value={sourceCalculationId} onChange={(event) => setSourceCalculationId(event.target.value)} required /></label>
          <label><span>{messages.proposal.replacement}</span><input value={replacementCalculationId} onChange={(event) => setReplacementCalculationId(event.target.value)} /></label>
          <button type="submit" disabled={busy || !serviceOnline || !accessToken.trim()}>{busy ? messages.proposal.saving : messages.proposal.create}</button>
        </form>
        </fieldset>
      </article>

      <article className="panel" aria-label={messages.review.ariaLabel}>
        <div className="panel-heading"><div><p className="eyebrow">{messages.review.eyebrow}</p><h2>{messages.review.title}</h2></div><span className="status-pill status-pill-warning">{messages.review.badge}</span></div>
        <p className="panel-copy">{messages.review.description}</p>
        <fieldset className="workflow-fieldset" disabled={workflowLocked || busy}>
        <legend className="sr-only">{messages.review.title}</legend>
        <form className="form-grid" onSubmit={load}>
          <label className="full-field"><span>{messages.review.adjustmentReference}</span><input value={lookupId} onChange={(event) => setLookupId(event.target.value)} placeholder={messages.review.placeholder} required /></label>
          <button type="submit" disabled={busy || !serviceOnline || !accessToken.trim()}>{busy ? messages.review.loading : messages.review.load}</button>
        </form>
        </fieldset>
        {record ? <>
          <div className="adjustment-record-card" aria-live="polite">
            <div className="adjustment-status-line"><strong>{statusLabel(record.status, messages)}</strong><span className="status-pill">{record.kind}</span></div>
            <dl className="summary-list">
              <div><dt>{messages.review.gross}</dt><dd>{record.delta.gross_atomic}</dd></div>
              <div><dt>{messages.review.tax}</dt><dd>{record.delta.tax_atomic}</dd></div>
              <div><dt>{messages.review.net}</dt><dd>{record.delta.net_atomic}</dd></div>
              <div><dt>{messages.review.reason}</dt><dd>{record.reason_code}</dd></div>
              <div><dt>{messages.review.proposer}</dt><dd>{record.proposer_subject}</dd></div>
              <div><dt>{messages.review.approver}</dt><dd>{record.approver_subject ?? messages.review.notApproved}</dd></div>
              <div><dt>{messages.review.requestEvidence}</dt><dd><code>{digestPreview(record.adjustment_digest)}</code></dd></div>
              <div><dt>{messages.review.supportingEvidence}</dt><dd><code>{digestPreview(record.evidence_digest)}</code></dd></div>
            </dl>
          </div>
          <fieldset className="workflow-fieldset" disabled={workflowLocked || busy}>
          <legend className="sr-only">{messages.review.nextStep}</legend>
          <form className="adjustment-transition-form" onSubmit={transition}>
            <h3>{messages.review.nextStep}</h3>
            {availableTransitions.length ? <>
              <label><span>{messages.review.nextStepLabel}</span><select value={transitionStatus} onChange={(event) => setTransitionStatus(event.target.value as "approved" | "rejected" | "voided")}>{availableTransitions.map((option) => <option key={option} value={option}>{statusLabel(option, messages)}</option>)}</select></label>
              <label><span>{messages.review.decisionEvidence}</span><textarea value={transitionEvidence} onChange={(event) => setTransitionEvidence(event.target.value)} aria-describedby="adjustment-transition-help" rows={4} required /><small id="adjustment-transition-help">{messages.review.decisionHelp}</small></label>
              <button type="submit" disabled={busy || !serviceOnline || !accessToken.trim()}>{messages.review.recordDecision} {statusLabel(transitionStatus, messages)}</button>
            </> : <p className="inline-warning">{messages.review.noTransition}</p>}
          </form>
          </fieldset>
          {record.status === "approved" && <div className="adjustment-chain-card" aria-label={messages.chain.ariaLabel}>
            <h3>{messages.chain.title}</h3>
            <p className="panel-copy">{messages.chain.description}</p>
            <WalletPicker locale={locale} disabled={busy || workflowLocked} connectEnabled={Boolean(chainArtifact)} connectedAddress={walletAddress?.toBase58()} onConnected={setWalletAddress} />
            {!chainArtifact ? <button type="button" onClick={compileChainEvidence} disabled={busy || workflowLocked}>{messages.chain.prepare}</button> : <>
              <dl className="summary-list"><div><dt>{messages.chain.ledgerReference}</dt><dd><code>{chainArtifact.chain_intent_id}</code></dd></div><div><dt>{messages.chain.ledgerRegistry}</dt><dd><code>{chainArtifact.registry}</code></dd></div><div><dt>{messages.chain.registryProof}</dt><dd>Confirmed ledger position {chainArtifact.registry_slot} · {chainArtifact.instruction.accounts.length} exact account records</dd></div><div><dt>{messages.chain.wallet}</dt><dd>{walletAddress?.toBase58() ?? messages.chain.notConnected}</dd></div></dl>
              <p className="inline-warning">{messages.chain.warning}</p>
              <button type="button" onClick={signAndSubmitChainEvidence} disabled={busy || workflowLocked}>{busy ? messages.chain.waiting : messages.chain.signSubmit}</button>
              {chainStatus && <div className="settlement-status" aria-live="polite"><strong>{messages.chain.durableStatus}: {statusLabel(chainStatus.status, messages)}</strong><small>{chainStatus.signature ? `Signature ${chainStatus.signature}` : messages.chain.noSignature}</small>{chainStatus.status === "unknown" && <small role="status">{messages.chain.providerAmbiguous}</small>}{chainStatus.status !== "finalized" && chainStatus.signature && <div className="status-actions"><button type="button" onClick={refreshChainStatus} disabled={busy}>{messages.chain.refresh}</button><button type="button" onClick={reconcileChainEvidence} disabled={busy}>{messages.chain.reconcile}</button></div>}{chainStatus.status === "finalized" && chainStatus.finalized_evidence && <><small role="status">{messages.chain.finalizedEvidence}</small><details><summary>{messages.chain.finalizedProof}</summary><dl className="summary-list"><div><dt>{messages.chain.event}</dt><dd>{chainStatus.finalized_evidence.event_type} · {chainStatus.finalized_evidence.event_version}</dd></div><div><dt>{messages.chain.slot}</dt><dd>{chainStatus.finalized_evidence.slot}</dd></div><div><dt>{messages.chain.index}</dt><dd>{chainStatus.finalized_evidence.event_index}</dd></div><div><dt>{messages.chain.evidenceDigest}</dt><dd><code>{digestPreview(chainStatus.finalized_evidence.event_evidence_digest)}</code></dd></div></dl></details></>}</div>}
            </>}
          </div>}
        </> : <div className="empty-state"><span className="empty-icon">◎</span><strong>{messages.review.noAdjustment}</strong><p>{messages.review.noAdjustmentCopy}</p></div>}
        <div className="adjustment-authority-note"><strong>{messages.review.authorityTitle}</strong> {messages.review.authorityCopy}</div>
      </article>
      {error && <p className="inline-error full-field" role="alert">{error}<button type="button" className="inline-retry" onClick={() => setError(null)}>{messages.common.dismiss}</button></p>}
      {notice && <p className="inline-success full-field" role="status">{notice}</p>}
      {!serviceOnline && <p className="inline-warning full-field" role="alert">{messages.common.unavailable}</p>}
    </section>
  );
}
