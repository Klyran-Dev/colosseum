import { FormEvent, useMemo, useState } from "react";
import { PublicKey } from "@solana/web3.js";
import { fetchWithTimeout, REQUEST_TIMEOUT_MS, responseError } from "./api";
import { createClientUuid } from "./clientIdentity";
import {
  connectInjectedWallet,
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

type SettlementAdjustmentPanelProps = {
  apiBase: string;
  serviceOnline: boolean;
  accessToken: string;
};

const KINDS: AdjustmentKind[] = ["Refund", "Reversal", "Correction", "Chargeback", "RemittanceAdjustment"];
const DEFAULT_ASSET = JSON.stringify(Array(32).fill(4));
const DEFAULT_EVIDENCE = JSON.stringify(Array(32).fill(5));
const DEFAULT_ORIGINAL = "01992f1b-4a70-7e30-8ea2-b6e37a9d9d13";
const DEFAULT_CALCULATION = "01992f1b-4a70-7e30-8ea2-b6e37a9d9d14";

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
  const [originalSettlementId, setOriginalSettlementId] = useState(DEFAULT_ORIGINAL);
  const [assetId, setAssetId] = useState(DEFAULT_ASSET);
  const [kind, setKind] = useState<AdjustmentKind>("Refund");
  const [grossAtomic, setGrossAtomic] = useState("-250");
  const [taxAtomic, setTaxAtomic] = useState("-25");
  const [netAtomic, setNetAtomic] = useState("-225");
  const [reasonCode, setReasonCode] = useState("customer_refund");
  const [evidenceDigest, setEvidenceDigest] = useState(DEFAULT_EVIDENCE);
  const [sourceCalculationId, setSourceCalculationId] = useState(DEFAULT_CALCULATION);
  const [replacementCalculationId, setReplacementCalculationId] = useState("");
  const [lookupId, setLookupId] = useState("");
  const [record, setRecord] = useState<SettlementAdjustment | null>(null);
  const [transitionStatus, setTransitionStatus] = useState<"approved" | "rejected" | "voided">("approved");
  const [transitionEvidence, setTransitionEvidence] = useState('{"review_ticket":""}');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [walletAddress, setWalletAddress] = useState<PublicKey | null>(null);
  const [chainArtifact, setChainArtifact] = useState<CompiledSettlementAdjustmentArtifact | null>(null);
  const [chainStatus, setChainStatus] = useState<AdjustmentChainIntentResponse | null>(null);

  const availableTransitions = useMemo(() => nextAdjustmentStatuses(record?.status ?? ""), [record?.status]);

  function requireToken(): string {
    const token = accessToken.trim();
    if (!token) throw new Error("An OIDC access token with the required adjustment scope and role is required. It is held in memory only.");
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
      if (!availableTransitions.includes(transitionStatus)) throw new Error("The selected lifecycle transition is not valid for the server-owned status.");
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
      if (!chainArtifact) throw new Error("Prepare the server-owned record instruction before loading its chain intent.");
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
      if (!chainArtifact) throw new Error("Prepare the server-owned record instruction before signing.");
      const connected = await connectInjectedWallet();
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
      if (!chainArtifact) throw new Error("Prepare the server-owned record instruction before reconciliation.");
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
        <div className="panel-heading"><div><p className="eyebrow">Financial operations / evidence first</p><h2>Propose an adjustment</h2></div><span className="status-pill">No token movement</span></div>
        <p className="panel-copy">Record a bounded refund, reversal, correction, chargeback, or remittance adjustment against an immutable settlement. The API owns merchant scope, authority, and final validation.</p>
        <form className="form-grid" onSubmit={propose}>
          <label className="full-field"><span>OIDC access token (memory-only)</span><textarea value={accessToken ? "••••••••••••••••" : ""} readOnly placeholder="Provide the token in the application header" aria-describedby="adjustment-token-help" /></label>
          <small id="adjustment-token-help" className="field-help full-field">The token is held by the application in memory and is never persisted. Use a proposer role with <code>tax:adjustment:propose</code>.</small>
          <label className="full-field"><span>Idempotency key</span><input value={idempotencyKey} onChange={(event) => setIdempotencyKey(event.target.value)} required /></label>
          <label><span>Adjustment kind</span><select value={kind} onChange={(event) => setKind(event.target.value as AdjustmentKind)}>{KINDS.map((option) => <option key={option} value={option}>{option}</option>)}</select></label>
          <label><span>Reason code</span><input value={reasonCode} onChange={(event) => setReasonCode(event.target.value)} maxLength={256} required /></label>
          <label className="full-field"><span>Original settlement UUIDv7</span><input value={originalSettlementId} onChange={(event) => setOriginalSettlementId(event.target.value)} required /></label>
          <label><span>Gross delta (atomic units)</span><input inputMode="numeric" value={grossAtomic} onChange={(event) => setGrossAtomic(event.target.value)} required /></label>
          <label><span>Tax delta (atomic units)</span><input inputMode="numeric" value={taxAtomic} onChange={(event) => setTaxAtomic(event.target.value)} required /></label>
          <label><span>Net delta (atomic units)</span><input inputMode="numeric" value={netAtomic} onChange={(event) => setNetAtomic(event.target.value)} required /></label>
          <label className="full-field"><span>Asset ID (32-byte JSON array)</span><textarea value={assetId} onChange={(event) => setAssetId(event.target.value)} rows={2} required /></label>
          <label className="full-field"><span>Evidence digest (32-byte JSON array)</span><textarea value={evidenceDigest} onChange={(event) => setEvidenceDigest(event.target.value)} rows={2} required /></label>
          <label><span>Source calculation UUIDv7</span><input value={sourceCalculationId} onChange={(event) => setSourceCalculationId(event.target.value)} required /></label>
          <label><span>Replacement calculation UUIDv7 (optional)</span><input value={replacementCalculationId} onChange={(event) => setReplacementCalculationId(event.target.value)} /></label>
          <button type="submit" disabled={busy || !serviceOnline || !accessToken.trim()}>{busy ? "Submitting…" : "Propose adjustment"}</button>
        </form>
      </article>

      <article className="panel">
        <div className="panel-heading"><div><p className="eyebrow">Tenant-scoped read and review</p><h2>Inspect lifecycle evidence</h2></div><span className="status-pill status-pill-warning">Not payment</span></div>
        <p className="panel-copy">Load a server-owned adjustment and review exact deltas, digests, subjects, and status before requesting a governed transition.</p>
        <form className="form-grid" onSubmit={load}>
          <label className="full-field"><span>Adjustment UUIDv7</span><input value={lookupId} onChange={(event) => setLookupId(event.target.value)} placeholder="Paste the server-owned adjustment ID" required /></label>
          <button type="submit" disabled={busy || !serviceOnline || !accessToken.trim()}>Load adjustment</button>
        </form>
        {record ? <>
          <div className="adjustment-record-card" aria-live="polite">
            <div className="adjustment-status-line"><strong>{statusLabel(record.status)}</strong><span className="status-pill">{record.kind}</span></div>
            <dl className="summary-list">
              <div><dt>Gross delta</dt><dd>{record.delta.gross_atomic}</dd></div>
              <div><dt>Tax delta</dt><dd>{record.delta.tax_atomic}</dd></div>
              <div><dt>Net delta</dt><dd>{record.delta.net_atomic}</dd></div>
              <div><dt>Reason</dt><dd>{record.reason_code}</dd></div>
              <div><dt>Proposer</dt><dd>{record.proposer_subject}</dd></div>
              <div><dt>Approver</dt><dd>{record.approver_subject ?? "Not approved"}</dd></div>
              <div><dt>Adjustment digest</dt><dd><code>{digestPreview(record.adjustment_digest)}</code></dd></div>
              <div><dt>Evidence digest</dt><dd><code>{digestPreview(record.evidence_digest)}</code></dd></div>
            </dl>
          </div>
          <form className="adjustment-transition-form" onSubmit={transition}>
            <h3>Governed transition</h3>
            {availableTransitions.length ? <>
              <label><span>Target status</span><select value={transitionStatus} onChange={(event) => setTransitionStatus(event.target.value as "approved" | "rejected" | "voided")}>{availableTransitions.map((option) => <option key={option} value={option}>{statusLabel(option)}</option>)}</select></label>
              <label><span>Evidence object</span><textarea value={transitionEvidence} onChange={(event) => setTransitionEvidence(event.target.value)} rows={4} required /></label>
              <button type="submit" disabled={busy || !serviceOnline || !accessToken.trim()}>Record {statusLabel(transitionStatus)} evidence</button>
            </> : <p className="inline-warning">This status has no available browser transition. A posted state is intentionally not represented here.</p>}
          </form>
          {record.status === "approved" && <div className="adjustment-chain-card" aria-label="Wallet-bound adjustment evidence">
            <h3>Record finalized adjustment evidence</h3>
            <p className="panel-copy">This bounded Solana instruction records the approved adjustment and emits auditable evidence. It does not transfer tokens, issue a refund, file taxes, or establish legal compliance.</p>
            {!chainArtifact ? <button type="button" onClick={compileChainEvidence} disabled={busy || !serviceOnline || !accessToken.trim()}>Prepare wallet record</button> : <>
              <dl className="summary-list"><div><dt>Chain intent</dt><dd><code>{chainArtifact.chain_intent_id}</code></dd></div><div><dt>Registry</dt><dd><code>{chainArtifact.registry}</code></dd></div><div><dt>Registry proof</dt><dd>Finalized slot {chainArtifact.registry_slot} · {chainArtifact.instruction.accounts.length} exact account metas</dd></div><div><dt>Wallet</dt><dd>{walletAddress?.toBase58() ?? "Not connected"}</dd></div></dl>
              <p className="inline-warning">The connected wallet must equal the compiled approver signer. The API repeats this check against the complete signed message.</p>
              <button type="button" onClick={signAndSubmitChainEvidence} disabled={busy || !serviceOnline || !accessToken.trim()}>{busy ? "Waiting for wallet or server…" : "Sign and submit evidence"}</button>
              {chainStatus && <div className="settlement-status" aria-live="polite"><strong>Durable evidence status: {statusLabel(chainStatus.status)}</strong><small>{chainStatus.signature ? `Signature ${chainStatus.signature}` : "No server-bound chain signature yet."}</small>{chainStatus.status === "unknown" && <small role="status">Provider outcome is ambiguous. Do not authorize another transaction; reconcile this intent.</small>}{chainStatus.status !== "finalized" && chainStatus.signature && <div className="status-actions"><button type="button" onClick={refreshChainStatus} disabled={busy}>Refresh status</button><button type="button" onClick={reconcileChainEvidence} disabled={busy}>Reconcile finalized evidence</button></div>}{chainStatus.status === "finalized" && chainStatus.finalized_evidence && <><small role="status">Finalized typed event evidence matched. This is not a payment or compliance claim.</small><details><summary>Finalized event proof</summary><dl className="summary-list"><div><dt>Event</dt><dd>{chainStatus.finalized_evidence.event_type} · {chainStatus.finalized_evidence.event_version}</dd></div><div><dt>Finalized slot</dt><dd>{chainStatus.finalized_evidence.slot}</dd></div><div><dt>Event index</dt><dd>{chainStatus.finalized_evidence.event_index}</dd></div><div><dt>Event evidence digest</dt><dd><code>{digestPreview(chainStatus.finalized_evidence.event_evidence_digest)}</code></dd></div></dl></details></>}</div>}
            </>}
          </div>}
        </> : <div className="empty-state"><span className="empty-icon">◎</span><strong>No adjustment loaded</strong><p>Propose or load a server-owned record to inspect exact evidence and governed lifecycle options.</p></div>}
        <div className="adjustment-authority-note"><strong>Authority boundary:</strong> approval is not payment, tax advice, filing, or legal compliance. Token movement requires a separate wallet-signed Solana workflow and finalized chain evidence.</div>
      </article>
      {error && <p className="inline-error full-field" role="alert">{error}<button type="button" className="inline-retry" onClick={() => setError(null)}>Dismiss</button></p>}
      {notice && <p className="inline-success full-field" role="status">{notice}</p>}
      {!serviceOnline && <p className="inline-warning full-field" role="alert">The API is unavailable. No cached adjustment state or client-derived lifecycle success is shown.</p>}
    </section>
  );
}
