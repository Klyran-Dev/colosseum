import { FormEvent, useMemo, useState } from "react";
import { PublicKey } from "@solana/web3.js";
import { fetchWithTimeout, REQUEST_TIMEOUT_MS, responseError } from "./api";
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

type SettlementAdjustmentPanelProps = {
  apiBase: string;
  serviceOnline: boolean;
  accessToken: string;
};

const KINDS: AdjustmentKind[] = ["Refund", "Reversal", "Correction", "Chargeback", "RemittanceAdjustment"];

function digestPreview(value: number[]): string {
  return value.map((byte) => byte.toString(16).padStart(2, "0")).join("").slice(0, 16) + "…";
}

function parseBytes(value: string, label: string): number[] {
  let parsed: unknown;
  try {
    parsed = JSON.parse(value);
  } catch {
    throw new Error(`${label} must be valid JSON.`);
  }
  return validateByteArray(parsed, label, true);
}

function statusLabel(status: string): string {
  return status.replaceAll("_", " ");
}

export default function SettlementAdjustmentPanel({ apiBase, serviceOnline, accessToken }: SettlementAdjustmentPanelProps) {
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
    if (!token) throw new Error("Sign in with an account allowed to manage adjustments. The credential is held in memory only.");
    return token;
  }

  function parseEvidence(value: string): Record<string, unknown> {
    let parsed: unknown;
    try {
      parsed = JSON.parse(value);
    } catch {
      throw new Error("Transition evidence must be valid JSON.");
    }
    if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) throw new Error("Transition evidence must be a JSON object.");
    return parsed as Record<string, unknown>;
  }

  async function propose(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      const token = requireToken();
      const parsedAsset = parseBytes(assetId, "asset_id");
      const parsedEvidenceDigest = parseBytes(evidenceDigest, "evidence_digest");
      validateUuidV7(originalSettlementId.trim(), "original_settlement_id");
      validateUuidV7(sourceCalculationId.trim(), "source_calculation_id");
      if (replacementCalculationId.trim()) validateUuidV7(replacementCalculationId.trim(), "replacement_calculation_id");
      validateAdjustmentDelta(kind, { gross_atomic: grossAtomic.trim(), tax_atomic: taxAtomic.trim(), net_atomic: netAtomic.trim() });
      if (!reasonCode.trim() || reasonCode.trim().length > 256) throw new Error("reason_code must contain between 1 and 256 characters.");
      if (!idempotencyKey.trim()) throw new Error("An idempotency key is required for safe retry.");
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
      if (!response.ok) throw new Error(responseError(response, payload, `Adjustment proposal failed (${response.status})`));
      const loaded = parseSettlementAdjustment(payload);
      setRecord(loaded);
      setChainArtifact(null);
      setChainStatus(null);
      setLookupId(loaded.id);
      setNotice(`Adjustment ${loaded.id} is ${statusLabel(loaded.status)}. No funds moved; this is governance evidence only.`);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Adjustment proposal failed.");
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
      if (!response.ok) throw new Error(responseError(response, payload, `Adjustment lookup failed (${response.status})`));
      setRecord(parseSettlementAdjustment(payload));
      setChainArtifact(null);
      setChainStatus(null);
      setNotice("Adjustment state loaded from the tenant-scoped API. Approval remains evidence and does not mean payment.");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Adjustment lookup failed.");
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
      if (!record) throw new Error("Load or propose an adjustment before changing its lifecycle.");
      if (!availableTransitions.includes(transitionStatus)) throw new Error("That next step is not available for the current request status.");
      const evidence = parseEvidence(transitionEvidence);
      const response = await fetchWithTimeout(`${buildSettlementAdjustmentUrl(apiBase, record.id)}/transition`, {
        method: "POST",
        headers: { "content-type": "application/json", authorization: `Bearer ${token}` },
        body: JSON.stringify({ to_status: transitionStatus, evidence }),
      });
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw new Error(responseError(response, payload, `Adjustment transition failed (${response.status})`));
      const transitionResponse = parseSettlementAdjustmentTransition(payload);
      const reload = await fetchWithTimeout(buildSettlementAdjustmentUrl(apiBase, transitionResponse.adjustment_id), { headers: { authorization: `Bearer ${token}` } });
      const reloadPayload: unknown = await reload.json().catch(() => null);
      if (!reload.ok) throw new Error(responseError(reload, reloadPayload, `Adjustment refresh failed (${reload.status})`));
      const refreshed = parseSettlementAdjustment(reloadPayload);
      setRecord(refreshed);
      setChainArtifact(null);
      setChainStatus(null);
      setNotice(`Server accepted the ${statusLabel(transitionResponse.to_status)} transition. No token movement occurred.`);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Adjustment transition failed.");
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
      if (!record || record.status !== "approved") throw new Error("Only an approved adjustment can be prepared for wallet evidence recording.");
      const response = await fetchWithTimeout(`${buildSettlementAdjustmentUrl(apiBase, record.id)}/compile`, {
        method: "POST",
        headers: { "content-type": "application/json", authorization: `Bearer ${token}` },
        body: JSON.stringify({ confirm: true }),
      }, REQUEST_TIMEOUT_MS.default);
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw new Error(responseError(response, payload, `Adjustment compilation failed (${response.status})`));
      const artifact = parseCompiledSettlementAdjustmentArtifact(payload);
      setChainArtifact(artifact);
      setChainStatus({
        chain_intent_id: artifact.chain_intent_id,
        status: artifact.status,
        signature: null,
        provider_index: null,
        finalized_evidence: null,
      });
      setNotice("Ready for wallet signing: the server prepared an exact record-adjustment instruction. Review it before signing; no tokens move in this evidence instruction.");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Adjustment compilation failed.");
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
      if (!chainArtifact) throw new Error("Prepare the ledger record before loading its saved status.");
      const response = await fetchWithTimeout(`${apiBase}/v1/settlement/adjustment-chain-intents/${chainArtifact.chain_intent_id}`, {
        headers: { authorization: `Bearer ${token}` },
      });
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw new Error(responseError(response, payload, `Adjustment chain status failed (${response.status})`));
      const status = parseSettlementAdjustmentChainIntentResponse(payload);
      setChainStatus(status);
      setNotice(`The server reports durable adjustment evidence status: ${statusLabel(status.status)}. This is not payment confirmation.`);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Adjustment chain status failed.");
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
      if (!chainArtifact) throw new Error("Prepare the ledger record before signing.");
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
      if (!response.ok) throw new Error(responseError(response, payload, `Adjustment submission failed (${response.status})`));
      const status = parseSettlementAdjustmentChainIntentResponse(payload);
      setChainStatus(status);
      setNotice(status.status === "finalized"
        ? "The server returned finalized adjustment evidence. This record-only instruction did not move tokens or establish filing/compliance."
        : `Wallet submission accepted into durable status ${statusLabel(status.status)}. Finalized event reconciliation is still required.`);
    } catch (cause) {
      if (submissionAttempted) {
        try {
          await refreshChainStatus();
          setError("The submission request did not complete. The durable chain intent was refreshed; do not sign a second transaction.");
        } catch {
          setError("The adjustment submission outcome is unknown. Do not sign again; load or reconcile the durable chain intent before retrying.");
        }
      } else {
        setError(cause instanceof Error ? cause.message : "Wallet signing or adjustment submission failed.");
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
      if (!chainArtifact) throw new Error("Prepare the ledger record before reconciliation.");
      if (!chainStatus?.signature) throw new Error("A server-bound chain signature is required before reconciliation.");
      const response = await fetchWithTimeout(`${apiBase}/v1/settlement/adjustment-chain-intents/${chainArtifact.chain_intent_id}/reconcile`, {
        method: "POST",
        headers: { authorization: `Bearer ${token}` },
      }, REQUEST_TIMEOUT_MS.settlementSubmit);
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw new Error(responseError(response, payload, `Adjustment reconciliation failed (${response.status})`));
      const status = parseSettlementAdjustmentChainIntentResponse(payload);
      setChainStatus(status);
      setNotice(status.status === "finalized"
        ? "Finalized typed event and raw evidence matched the immutable adjustment intent. No token movement is implied."
        : `Reconciliation returned ${statusLabel(status.status)}. The indexer or provider may still be catching up.`);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Adjustment reconciliation failed.");
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="adjustment-layout" aria-label="Settlement adjustments workspace">
      <article className="panel">
        <div className="panel-heading"><div><p className="eyebrow">Payment corrections / review first</p><h2>Create an adjustment request</h2></div><span className="status-pill">No money moved</span></div>
        <p className="panel-copy">Create a refund, reversal, correction, chargeback, or remittance request against an existing payment. The service checks the account, approval rights, and exact amounts before saving it.</p>
        <form className="form-grid" onSubmit={propose}>
          <label className="full-field"><span>Sign-in status</span><textarea value={accessToken ? "Account ready" : ""} readOnly placeholder="Sign in to create a request" aria-describedby="adjustment-token-help" /></label>
          <small id="adjustment-token-help" className="field-help full-field">Your account must be allowed to create payment corrections. Sign-in data is never saved by this page.</small>
          <label className="full-field"><span>Request reference</span><input value={idempotencyKey} onChange={(event) => setIdempotencyKey(event.target.value)} aria-describedby="adjustment-reference-help" required /><small id="adjustment-reference-help">A unique reference prevents duplicate requests when you retry.</small></label>
          <label><span>Correction type</span><select value={kind} onChange={(event) => setKind(event.target.value as AdjustmentKind)}>{KINDS.map((option) => <option key={option} value={option}>{option}</option>)}</select></label>
          <label><span>Reason</span><input value={reasonCode} onChange={(event) => setReasonCode(event.target.value)} maxLength={256} required /></label>
          <label className="full-field"><span>Original payment reference</span><input value={originalSettlementId} onChange={(event) => setOriginalSettlementId(event.target.value)} required /></label>
          <label><span>Gross change</span><input inputMode="numeric" value={grossAtomic} onChange={(event) => setGrossAtomic(event.target.value)} aria-describedby="adjustment-amount-help" required /></label>
          <label><span>Tax change</span><input inputMode="numeric" value={taxAtomic} onChange={(event) => setTaxAtomic(event.target.value)} aria-describedby="adjustment-amount-help" required /></label>
          <label><span>Net change</span><input inputMode="numeric" value={netAtomic} onChange={(event) => setNetAtomic(event.target.value)} aria-describedby="adjustment-amount-help" required /></label>
          <small id="adjustment-amount-help" className="field-help full-field">Enter exact asset amounts. The service checks precision and conservation before saving.</small>
          <label className="full-field"><span>Asset address</span><textarea value={assetId} onChange={(event) => setAssetId(event.target.value)} rows={2} required /></label>
          <label className="full-field"><span>Supporting evidence</span><textarea value={evidenceDigest} onChange={(event) => setEvidenceDigest(event.target.value)} aria-describedby="adjustment-evidence-help" rows={2} required /><small id="adjustment-evidence-help">Advanced evidence supplied by the approved workflow. Do not edit it manually.</small></label>
          <label><span>Source calculation reference</span><input value={sourceCalculationId} onChange={(event) => setSourceCalculationId(event.target.value)} required /></label>
          <label><span>Replacement calculation reference (optional)</span><input value={replacementCalculationId} onChange={(event) => setReplacementCalculationId(event.target.value)} /></label>
          <button type="submit" disabled={busy || !serviceOnline || !accessToken.trim()}>{busy ? "Saving…" : "Create adjustment request"}</button>
        </form>
      </article>

      <article className="panel">
        <div className="panel-heading"><div><p className="eyebrow">Payment correction / review and approval</p><h2>Review an adjustment</h2></div><span className="status-pill status-pill-warning">No money moved</span></div>
        <p className="panel-copy">Load a saved request to review the exact changes, reason, evidence, and approvals before choosing its next step.</p>
        <form className="form-grid" onSubmit={load}>
          <label className="full-field"><span>Adjustment reference</span><input value={lookupId} onChange={(event) => setLookupId(event.target.value)} placeholder="Paste a saved adjustment reference" required /></label>
          <button type="submit" disabled={busy || !serviceOnline || !accessToken.trim()}>Load adjustment</button>
        </form>
        {record ? <>
          <div className="adjustment-record-card" aria-live="polite">
            <div className="adjustment-status-line"><strong>{statusLabel(record.status)}</strong><span className="status-pill">{record.kind}</span></div>
            <dl className="summary-list">
              <div><dt>Gross change</dt><dd>{record.delta.gross_atomic}</dd></div>
              <div><dt>Tax change</dt><dd>{record.delta.tax_atomic}</dd></div>
              <div><dt>Net change</dt><dd>{record.delta.net_atomic}</dd></div>
              <div><dt>Reason</dt><dd>{record.reason_code}</dd></div>
              <div><dt>Proposer</dt><dd>{record.proposer_subject}</dd></div>
              <div><dt>Approver</dt><dd>{record.approver_subject ?? "Not approved"}</dd></div>
              <div><dt>Request evidence</dt><dd><code>{digestPreview(record.adjustment_digest)}</code></dd></div>
              <div><dt>Supporting evidence</dt><dd><code>{digestPreview(record.evidence_digest)}</code></dd></div>
            </dl>
          </div>
          <form className="adjustment-transition-form" onSubmit={transition}>
            <h3>Choose the next step</h3>
            {availableTransitions.length ? <>
              <label><span>Next step</span><select value={transitionStatus} onChange={(event) => setTransitionStatus(event.target.value as "approved" | "rejected" | "voided")}>{availableTransitions.map((option) => <option key={option} value={option}>{statusLabel(option)}</option>)}</select></label>
              <label><span>Decision evidence</span><textarea value={transitionEvidence} onChange={(event) => setTransitionEvidence(event.target.value)} aria-describedby="adjustment-transition-help" rows={4} required /><small id="adjustment-transition-help">Advanced evidence for the audit record.</small></label>
              <button type="submit" disabled={busy || !serviceOnline || !accessToken.trim()}>Record {statusLabel(transitionStatus)} decision</button>
            </> : <p className="inline-warning">This status has no available browser transition. A posted state is intentionally not represented here.</p>}
          </form>
          {record.status === "approved" && <div className="adjustment-chain-card" aria-label="Wallet-bound adjustment evidence">
            <h3>Record the approved correction</h3>
            <p className="panel-copy">This ledger record confirms the approved correction and creates an audit trail. It does not transfer tokens, issue a refund, file taxes, or establish legal compliance.</p>
            <WalletPicker disabled={busy} connectEnabled={Boolean(chainArtifact)} connectedAddress={walletAddress?.toBase58()} onConnected={setWalletAddress} />
            {!chainArtifact ? <button type="button" onClick={compileChainEvidence} disabled={busy || !serviceOnline || !accessToken.trim()}>Prepare ledger record</button> : <>
              <dl className="summary-list"><div><dt>Ledger record reference</dt><dd><code>{chainArtifact.chain_intent_id}</code></dd></div><div><dt>Ledger registry</dt><dd><code>{chainArtifact.registry}</code></dd></div><div><dt>Registry proof</dt><dd>Confirmed ledger position {chainArtifact.registry_slot} · {chainArtifact.instruction.accounts.length} exact account records</dd></div><div><dt>Wallet</dt><dd>{walletAddress?.toBase58() ?? "Not connected"}</dd></div></dl>
              <p className="inline-warning">The connected wallet must equal the compiled approver signer. The API repeats this check against the complete signed message.</p>
              <button type="button" onClick={signAndSubmitChainEvidence} disabled={busy || !serviceOnline || !accessToken.trim()}>{busy ? "Waiting for wallet or service…" : "Sign and submit ledger record"}</button>
              {chainStatus && <div className="settlement-status" aria-live="polite"><strong>Durable evidence status: {statusLabel(chainStatus.status)}</strong><small>{chainStatus.signature ? `Signature ${chainStatus.signature}` : "No server-bound chain signature yet."}</small>{chainStatus.status === "unknown" && <small role="status">Provider outcome is ambiguous. Do not authorize another transaction; reconcile this intent.</small>}{chainStatus.status !== "finalized" && chainStatus.signature && <div className="status-actions"><button type="button" onClick={refreshChainStatus} disabled={busy}>Refresh status</button><button type="button" onClick={reconcileChainEvidence} disabled={busy}>Reconcile finalized evidence</button></div>}{chainStatus.status === "finalized" && chainStatus.finalized_evidence && <><small role="status">Finalized typed event evidence matched. This is not a payment or compliance claim.</small><details><summary>Finalized event proof</summary><dl className="summary-list"><div><dt>Event</dt><dd>{chainStatus.finalized_evidence.event_type} · {chainStatus.finalized_evidence.event_version}</dd></div><div><dt>Finalized slot</dt><dd>{chainStatus.finalized_evidence.slot}</dd></div><div><dt>Event index</dt><dd>{chainStatus.finalized_evidence.event_index}</dd></div><div><dt>Event evidence digest</dt><dd><code>{digestPreview(chainStatus.finalized_evidence.event_evidence_digest)}</code></dd></div></dl></details></>}</div>}
            </>}
          </div>}
        </> : <div className="empty-state"><span className="empty-icon">◎</span><strong>No adjustment loaded</strong><p>Create or load a request to review the exact changes and available approval steps.</p></div>}
        <div className="adjustment-authority-note"><strong>What this page can do:</strong> record a correction request and its approvals. Approval is not payment, tax advice, filing, or legal compliance. Token movement requires a separate wallet-approved ledger workflow and confirmed chain evidence.</div>
      </article>
      {error && <p className="inline-error full-field" role="alert">{error}<button type="button" className="inline-retry" onClick={() => setError(null)}>Dismiss</button></p>}
      {notice && <p className="inline-success full-field" role="status">{notice}</p>}
      {!serviceOnline && <p className="inline-warning full-field" role="alert">The API is unavailable. No cached adjustment state or client-derived lifecycle success is shown.</p>}
    </section>
  );
}
