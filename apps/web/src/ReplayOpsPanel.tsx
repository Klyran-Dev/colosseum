import { FormEvent, useState } from "react";
import { fetchWithTimeout, REQUEST_TIMEOUT_MS, responseError } from "./api";
import { DEFAULT_LOCALE, DEFAULT_TIME_ZONE, formatTimestamp } from "./formatters";
import {
  buildReplayGapActionUrl,
  buildReplayGapListUrl,
  buildTargetedRecoveryUrl,
  MAX_OPERATOR_NOTE_LENGTH,
  MAX_REPLAY_LIST_LIMIT,
  MAX_TARGETED_RECOVERY_PAGES,
  parseReplayGap,
  parseReplayGapList,
  parseTargetedRecoveryResult,
  ReplayGap,
  ReplayGapAction,
  ReplayGapStatus,
  REPLAY_STATUSES,
  validateRecoveryTarget,
  validateOperatorNote,
} from "./replayOps";

type ReplayOpsPanelProps = {
  apiBase: string;
  serviceOnline: boolean;
  locale?: string;
  timeZone?: string;
};

function displayStatus(status: ReplayGapStatus): string {
  return status.replaceAll("_", " ");
}

function actionLabel(action: ReplayGapAction): string {
  return action === "acknowledge" ? "Acknowledge review" : "Requeue repair";
}

export default function ReplayOpsPanel({ apiBase, serviceOnline, locale = DEFAULT_LOCALE, timeZone = DEFAULT_TIME_ZONE }: ReplayOpsPanelProps) {
  const [streamKey, setStreamKey] = useState("mainnet/settlement");
  const [status, setStatus] = useState<ReplayGapStatus>("dead_lettered");
  const [limit, setLimit] = useState("50");
  const [accessToken, setAccessToken] = useState("");
  const [gaps, setGaps] = useState<ReplayGap[]>([]);
  const [notes, setNotes] = useState<Record<string, string>>({});
  const [busyKey, setBusyKey] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [hasLoaded, setHasLoaded] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [recoveryAddress, setRecoveryAddress] = useState("");
  const [recoverySignature, setRecoverySignature] = useState("");
  const [recoveryPages, setRecoveryPages] = useState("8");
  const [recoveryNote, setRecoveryNote] = useState("");
  const [isRecovering, setIsRecovering] = useState(false);

  async function fetchGaps(): Promise<ReplayGap[]> {
    if (!accessToken.trim()) throw new Error("An in-memory token with tax:indexer:read and indexer_operator is required.");
    const parsedLimit = Number(limit);
    const response = await fetchWithTimeout(buildReplayGapListUrl(apiBase, streamKey.trim(), status, parsedLimit), {
      headers: { authorization: `Bearer ${accessToken.trim()}` },
    }, REQUEST_TIMEOUT_MS.default);
    const payload: unknown = await response.json().catch(() => null);
    if (!response.ok) throw new Error(responseError(response, payload, `Replay-gap lookup failed (${response.status})`));
    return parseReplayGapList(payload);
  }

  async function loadGaps(event?: FormEvent<HTMLFormElement>) {
    event?.preventDefault();
    setError(null);
    setNotice(null);
    setIsLoading(true);
    try {
      const loaded = await fetchGaps();
      setGaps(loaded);
      setHasLoaded(true);
      setNotice(`Loaded ${loaded.length} server-owned replay gap${loaded.length === 1 ? "" : "s"}.`);
    } catch (cause) {
      setGaps([]);
      setHasLoaded(false);
      setError(cause instanceof Error ? cause.message : "Replay-gap lookup failed");
    } finally {
      setIsLoading(false);
    }
  }

  async function performAction(gap: ReplayGap, action: ReplayGapAction) {
    const noteValue = notes[gap.id] ?? "";
    let note: string;
    try {
      note = validateOperatorNote(noteValue);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Enter an operator note before acting.");
      return;
    }
    setError(null);
    setNotice(null);
    setBusyKey(`${gap.id}:${action}`);
    try {
      const response = await fetchWithTimeout(buildReplayGapActionUrl(apiBase, gap.id, action), {
        method: "POST",
        headers: {
          "content-type": "application/json",
          authorization: `Bearer ${accessToken.trim()}`,
        },
        body: JSON.stringify({ note }),
      }, REQUEST_TIMEOUT_MS.default);
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw new Error(responseError(response, payload, `${actionLabel(action)} failed (${response.status})`));
      const actionResult = parseReplayGap(payload);
      try {
        const reloaded = await fetchGaps();
        setGaps(reloaded);
        setHasLoaded(true);
        setNotice(`${actionLabel(action)} accepted by the server; the projection was reloaded.`);
      } catch (reloadCause) {
        setGaps((current) => current.map((currentGap) => currentGap.id === actionResult.id ? actionResult : currentGap));
        setHasLoaded(true);
        setError(`The server accepted ${actionLabel(action).toLowerCase()}, but the follow-up projection reload failed: ${reloadCause instanceof Error ? reloadCause.message : "unknown error"}`);
      }
      setNotes((current) => ({ ...current, [gap.id]: "" }));
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : `${actionLabel(action)} failed`);
    } finally {
      setBusyKey(null);
    }
  }

  async function recoverTargetedSignature(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);
    setNotice(null);
    let address: string;
    let signature: string;
    let note: string;
    try {
      address = validateRecoveryTarget(recoveryAddress, "Address");
      signature = validateRecoveryTarget(recoverySignature, "Signature");
      note = validateOperatorNote(recoveryNote);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Targeted recovery input is malformed.");
      return;
    }
    const pages = Number(recoveryPages);
    if (!Number.isSafeInteger(pages) || pages < 1 || pages > MAX_TARGETED_RECOVERY_PAGES) {
      setError(`Recovery page budget must be between 1 and ${MAX_TARGETED_RECOVERY_PAGES}.`);
      return;
    }
    setIsRecovering(true);
    try {
      const response = await fetchWithTimeout(buildTargetedRecoveryUrl(apiBase), {
        method: "POST",
        headers: { "content-type": "application/json", authorization: `Bearer ${accessToken.trim()}` },
        body: JSON.stringify({ address, signature, note, max_pages: pages }),
      }, REQUEST_TIMEOUT_MS.default);
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw new Error(responseError(response, payload, `Targeted recovery failed (${response.status})`));
      const result = parseTargetedRecoveryResult(payload);
      setNotice(`Finalized evidence recovered at slot ${result.slot}; ${result.raw_evidence_count} event record${result.raw_evidence_count === 1 ? "" : "s"} persisted. Audit ${result.audit_id}.`);
      setRecoveryNote("");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Targeted recovery failed; inspect the server audit state before retrying.");
    } finally {
      setIsRecovering(false);
    }
  }

  return (
    <section className="replay-layout" aria-label="Indexer replay operations workspace">
      <article className="panel replay-control-panel">
        <div className="panel-heading">
          <div><p className="eyebrow">Chain data / recovery control</p><h2>Review replay gaps</h2></div>
          <span className="status-pill">Operator role required</span>
        </div>
        <p className="panel-copy">Inspect durable projection-repair work and record an incident-backed review before requeueing. This workspace never treats a replay action as finalized settlement.</p>
        <form onSubmit={loadGaps} className="form-grid">
          <label className="full-field"><span>OIDC access token (memory-only)</span><textarea value={accessToken} onChange={(event) => setAccessToken(event.target.value)} placeholder="Token with tax:indexer:read and indexer_operator" autoComplete="off" spellCheck={false} rows={3} required /></label>
          <label className="full-field"><span>Replay stream</span><input value={streamKey} onChange={(event) => setStreamKey(event.target.value)} placeholder="mainnet/settlement" autoComplete="off" required /></label>
          <label><span>Status filter</span><select value={status} onChange={(event) => setStatus(event.target.value as ReplayGapStatus)}>{REPLAY_STATUSES.map((value) => <option value={value} key={value}>{displayStatus(value)}</option>)}</select></label>
          <label><span>Maximum rows (1–{MAX_REPLAY_LIST_LIMIT})</span><input inputMode="numeric" min="1" max={MAX_REPLAY_LIST_LIMIT} value={limit} onChange={(event) => setLimit(event.target.value)} required /></label>
          <button type="submit" disabled={!serviceOnline || isLoading || !accessToken.trim() || !streamKey.trim()}>{isLoading ? "Loading server projection…" : "Load replay gaps"}</button>
        </form>
        {!serviceOnline && <p className="inline-warning" role="alert">The service is unavailable. No cached or fabricated replay state is shown.</p>}
        {error && <p className="inline-error" role="alert">{error}</p>}
        {notice && <p className="inline-success" role="status">{notice}</p>}
        <div className="replay-authority-note"><strong>Authority boundary</strong><span>JWT scope and role are checked by the API. Acknowledgement is review evidence; requeue only schedules bounded projection repair.</span></div>
        <div className="replay-targeted-recovery">
          <div><p className="eyebrow">Finalized evidence repair</p><h3>Recover one transaction signature</h3></div>
          <p className="panel-copy">The API discovers the signature at finalized commitment, verifies the exact transaction, and rebuilds projection evidence. It never submits funds or advances the forward checkpoint.</p>
          <form onSubmit={recoverTargetedSignature} className="form-grid">
            <label><span>Target address</span><input value={recoveryAddress} onChange={(event) => setRecoveryAddress(event.target.value)} placeholder="Base58 address" autoComplete="off" required /></label>
            <label><span>Transaction signature</span><input value={recoverySignature} onChange={(event) => setRecoverySignature(event.target.value)} placeholder="Base58 signature" autoComplete="off" required /></label>
            <label><span>Page budget (1–{MAX_TARGETED_RECOVERY_PAGES})</span><input inputMode="numeric" min="1" max={MAX_TARGETED_RECOVERY_PAGES} value={recoveryPages} onChange={(event) => setRecoveryPages(event.target.value)} required /></label>
            <label className="full-field"><span>Incident / recovery note (required)</span><textarea value={recoveryNote} onChange={(event) => setRecoveryNote(event.target.value)} maxLength={MAX_OPERATOR_NOTE_LENGTH} rows={2} placeholder="Reference the incident and why this evidence repair is required" required /></label>
            <button type="submit" disabled={!serviceOnline || isRecovering || !accessToken.trim()}>{isRecovering ? "Recovering finalized evidence…" : "Recover finalized evidence"}</button>
          </form>
        </div>
      </article>

      <article className="panel replay-evidence-panel" aria-live="polite">
        <div className="panel-heading"><div><p className="eyebrow">Durable server projection</p><h2>Replay-gap queue</h2></div>{hasLoaded && <span className="status-pill status-pill-warning">{gaps.length} returned</span>}</div>
        {!hasLoaded ? <div className="empty-state"><span className="empty-icon">↻</span><strong>No replay projection loaded</strong><p>Authenticate as an indexer operator and load a bounded server projection before taking any action.</p></div> : gaps.length === 0 ? <div className="empty-state"><span className="empty-icon">✓</span><strong>No gaps match this filter</strong><p>This is a server response, not an inference that the chain or every projection is healthy.</p></div> : <div className="replay-gap-list">{gaps.map((gap) => {
          const acknowledged = gap.operator_acknowledged_at !== null;
          const acknowledgeBusy = busyKey === `${gap.id}:acknowledge`;
          const requeueBusy = busyKey === `${gap.id}:requeue`;
          return <article className="replay-gap-card" key={gap.id}>
            <div className="replay-gap-heading"><div><strong>{displayStatus(gap.status)}</strong><small>{gap.stream_key} · gap {gap.id}</small></div><span className="status-pill status-pill-warning">{gap.attempts}/{8} attempts</span></div>
            <dl className="summary-list replay-summary"><div><dt>Slot range</dt><dd>{gap.start_slot}–{gap.end_slot}</dd></div><div><dt>Observed finalized slot</dt><dd>{gap.observed_finalized_slot}</dd></div><div><dt>Reason</dt><dd>{gap.reason}</dd></div><div><dt>Last error</dt><dd>{gap.last_error_code ?? "None recorded"}</dd></div><div><dt>Created</dt><dd>{formatTimestamp(gap.created_at, locale, timeZone)}</dd></div>{gap.dead_lettered_at && <div><dt>Dead-lettered</dt><dd>{formatTimestamp(gap.dead_lettered_at, locale, timeZone)}</dd></div>}{acknowledged && <div><dt>Reviewed by</dt><dd>{gap.operator_acknowledged_by} · {formatTimestamp(gap.operator_acknowledged_at ?? gap.created_at, locale, timeZone)}</dd></div>}</dl>
            {gap.status === "dead_lettered" && <div className="replay-action-box"><label><span>Incident / operator note (required, max {MAX_OPERATOR_NOTE_LENGTH} UTF-8 bytes)</span><textarea value={notes[gap.id] ?? ""} onChange={(event) => setNotes((current) => ({ ...current, [gap.id]: event.target.value }))} rows={2} placeholder={acknowledged ? "Explain why bounded repair may be requeued" : "Reference the incident and review decision"} disabled={busyKey !== null} /></label><div className="replay-action-buttons">{!acknowledged && <button type="button" onClick={() => performAction(gap, "acknowledge")} disabled={busyKey !== null}>{acknowledgeBusy ? "Recording review…" : "Acknowledge review"}</button>}{acknowledged && <button type="button" onClick={() => performAction(gap, "requeue")} disabled={busyKey !== null}>{requeueBusy ? "Requeueing bounded repair…" : "Requeue repair"}</button>}</div></div>}
            {gap.status !== "dead_lettered" && <p className="evidence-note">No operator mutation is available for this state. The API remains the authority for replay lifecycle transitions.</p>}
          </article>;
        })}</div>}
      </article>
    </section>
  );
}
