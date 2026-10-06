import { FormEvent, useState } from "react";
import { fetchWithTimeout, operatorResponseError, REQUEST_TIMEOUT_MS } from "./api";
import { DEFAULT_LOCALE, DEFAULT_TIME_ZONE, formatTimestamp } from "./formatters";
import { replayOpsMessagesFor, type ReplayOpsMessages } from "./replayOpsMessages";
import {
  buildRouteVerificationRunsUrl,
  buildReplayGapActionUrl,
  buildReplayGapListUrl,
  buildTargetedRecoveryUrl,
  MAX_OPERATOR_NOTE_LENGTH,
  MAX_REPLAY_LIST_LIMIT,
  MAX_TARGETED_RECOVERY_PAGES,
  parseReplayGap,
  parseReplayGapList,
  parseRouteVerificationRunList,
  parseTargetedRecoveryResult,
  ReplayGap,
  ReplayGapAction,
  ReplayGapStatus,
  RouteVerificationRun,
  REPLAY_STATUSES,
  validateRecoveryTarget,
  validateOperatorNote,
} from "./replayOps";
import { operatorErrorMessage, operatorFacingError } from "./operatorError";

type ReplayOpsPanelProps = {
  apiBase: string;
  serviceOnline: boolean;
  accessToken: string;
  locale?: string;
  timeZone?: string;
};

function displayStatus(status: ReplayGapStatus, messages: ReplayOpsMessages): string {
  return messages.status.labels[status] ?? status.replaceAll("_", " ");
}

function actionLabel(action: ReplayGapAction, messages: ReplayOpsMessages): string {
  return action === "acknowledge" ? messages.actions.acknowledge : messages.actions.requeue;
}

export default function ReplayOpsPanel({ apiBase, serviceOnline, accessToken, locale = DEFAULT_LOCALE, timeZone = DEFAULT_TIME_ZONE }: ReplayOpsPanelProps) {
  const messages = replayOpsMessagesFor(locale);
  const [streamKey, setStreamKey] = useState("mainnet/settlement");
  const [status, setStatus] = useState<ReplayGapStatus>("dead_lettered");
  const [limit, setLimit] = useState("50");
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
  const [routeVerificationRuns, setRouteVerificationRuns] = useState<RouteVerificationRun[]>([]);
  const [routeVerificationLoaded, setRouteVerificationLoaded] = useState(false);
  const [routeVerificationLoading, setRouteVerificationLoading] = useState(false);
  const [routeVerificationError, setRouteVerificationError] = useState<string | null>(null);

  async function fetchGaps(): Promise<ReplayGap[]> {
    if (!accessToken.trim()) throw operatorFacingError(messages.common.error);
    const parsedLimit = Number(limit);
    const response = await fetchWithTimeout(buildReplayGapListUrl(apiBase, streamKey.trim(), status, parsedLimit), {
      headers: { authorization: `Bearer ${accessToken.trim()}` },
    }, REQUEST_TIMEOUT_MS.default);
    const payload: unknown = await response.json().catch(() => null);
    if (!response.ok) throw operatorResponseError(response, payload, messages.common.httpError(messages.common.error, response.status));
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
      setNotice(`${messages.queue.returned(loaded.length)}.`);
    } catch (cause) {
      setGaps([]);
      setHasLoaded(false);
      setError(operatorErrorMessage(cause, messages.common.error));
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
      setError(operatorErrorMessage(cause, messages.common.error));
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
      if (!response.ok) throw operatorResponseError(response, payload, messages.common.httpError(actionLabel(action, messages), response.status));
      const actionResult = parseReplayGap(payload);
      try {
        const reloaded = await fetchGaps();
        setGaps(reloaded);
        setHasLoaded(true);
        setNotice(`${actionLabel(action, messages)} accepted by the server; the projection was reloaded.`);
      } catch (reloadCause) {
        setGaps((current) => current.map((currentGap) => currentGap.id === actionResult.id ? actionResult : currentGap));
        setHasLoaded(true);
        setError(operatorErrorMessage(reloadCause, messages.common.projectionReloadFailed(actionLabel(action, messages))));
      }
      setNotes((current) => ({ ...current, [gap.id]: "" }));
    } catch (cause) {
      setError(operatorErrorMessage(cause, messages.common.actionFailed(actionLabel(action, messages))));
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
      address = validateRecoveryTarget(recoveryAddress, messages.targeted.address);
      signature = validateRecoveryTarget(recoverySignature, messages.targeted.signature);
      note = validateOperatorNote(recoveryNote);
    } catch (cause) {
      setError(operatorErrorMessage(cause, messages.common.recoveryError));
      return;
    }
    const pages = Number(recoveryPages);
    if (!Number.isSafeInteger(pages) || pages < 1 || pages > MAX_TARGETED_RECOVERY_PAGES) {
      setError(messages.common.pagesOutOfRange(MAX_TARGETED_RECOVERY_PAGES));
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
      if (!response.ok) throw operatorResponseError(response, payload, messages.common.httpError(messages.common.recoveryError, response.status));
      const result = parseTargetedRecoveryResult(payload);
      setNotice(`${messages.targeted.recover}: ${result.slot}; ${result.raw_evidence_count} record${result.raw_evidence_count === 1 ? "" : "s"} persisted. Audit ${result.audit_id}.`);
      setRecoveryNote("");
    } catch (cause) {
      setError(operatorErrorMessage(cause, messages.common.recoveryError));
    } finally {
      setIsRecovering(false);
    }
  }

  async function loadRouteVerificationRuns() {
    setRouteVerificationError(null);
    setRouteVerificationLoading(true);
    try {
      if (!accessToken.trim()) throw operatorFacingError(messages.common.error);
      const response = await fetchWithTimeout(buildRouteVerificationRunsUrl(apiBase, 20), {
        headers: { authorization: `Bearer ${accessToken.trim()}` },
      }, REQUEST_TIMEOUT_MS.default);
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw operatorResponseError(response, payload, messages.common.httpError(messages.common.routeError, response.status));
      try {
        setRouteVerificationRuns(parseRouteVerificationRunList(payload));
      } catch {
        throw operatorFacingError(messages.common.malformedRouteEvidence);
      }
      setRouteVerificationLoaded(true);
    } catch (cause) {
      setRouteVerificationRuns([]);
      setRouteVerificationLoaded(false);
      setRouteVerificationError(operatorErrorMessage(cause, messages.common.routeError));
    } finally {
      setRouteVerificationLoading(false);
    }
  }

  function routeVerificationStatusLabel(status: RouteVerificationRun["status"]): string {
    return messages.route.statusLabels[status] ?? status;
  }

  return (
    <section className="replay-layout" aria-label={messages.control.ariaLabel}>
      <article className="panel replay-control-panel">
        <div className="panel-heading">
          <div><p className="eyebrow">{messages.control.eyebrow}</p><h2>{messages.control.title}</h2></div>
          <span className="status-pill">{messages.control.badge}</span>
        </div>
        <p className="panel-copy">{messages.control.description}</p>
        <form onSubmit={loadGaps} className="form-grid">
          <label className="full-field"><span>{messages.control.stream}</span><input value={streamKey} onChange={(event) => setStreamKey(event.target.value)} placeholder={messages.control.streamPlaceholder} autoComplete="off" required /></label>
          <label><span>{messages.control.status}</span><select value={status} onChange={(event) => setStatus(event.target.value as ReplayGapStatus)}>{REPLAY_STATUSES.map((value) => <option value={value} key={value}>{displayStatus(value, messages)}</option>)}</select></label>
          <label><span>{messages.control.limit} (1–{MAX_REPLAY_LIST_LIMIT})</span><input inputMode="numeric" min="1" max={MAX_REPLAY_LIST_LIMIT} value={limit} onChange={(event) => setLimit(event.target.value)} required /></label>
          <button type="submit" disabled={!serviceOnline || isLoading || !accessToken.trim() || !streamKey.trim()}>{isLoading ? messages.control.loading : messages.control.load}</button>
        </form>
        {!serviceOnline && <p className="inline-warning" role="alert">{messages.control.unavailable}</p>}
        {error && <p className="inline-error" role="alert">{error}</p>}
        {notice && <p className="inline-success" role="status">{notice}</p>}
        <div className="replay-authority-note"><strong>{messages.control.authorityTitle}</strong><span>{messages.control.authorityCopy}</span></div>
        <details className="replay-route-verification">
          <summary>{messages.route.summary}</summary>
          <p className="panel-copy">{messages.route.description}</p>
          <button type="button" onClick={loadRouteVerificationRuns} disabled={!serviceOnline || routeVerificationLoading || !accessToken.trim()}>
            {routeVerificationLoading ? messages.route.loading : messages.route.load}
          </button>
          {!serviceOnline && <p className="inline-warning" role="alert">{messages.route.unavailable}</p>}
          {routeVerificationError && <p className="inline-error" role="alert">{routeVerificationError}</p>}
          {routeVerificationLoaded && routeVerificationRuns.length === 0 && <div className="empty-state"><strong>{messages.route.empty}</strong><p>{messages.route.emptyCopy}</p></div>}
          {routeVerificationLoaded && routeVerificationRuns.length > 0 && <div className="route-verification-run-list" aria-live="polite">{routeVerificationRuns.map((run) => <article className="route-verification-run-card" key={run.id}>
            <div className="replay-gap-heading"><div><strong>{routeVerificationStatusLabel(run.status)}</strong><small>{run.run_kind} · {run.id}</small></div><span className={`status-pill ${run.status === "completed" ? "" : "status-pill-warning"}`}>{messages.route.failed(run.failed_count)}</span></div>
            <dl className="summary-list replay-summary"><div><dt>{messages.route.routesExamined}</dt><dd>{run.examined_count}</dd></div><div><dt>{messages.route.activated}</dt><dd>{run.activated_count}</dd></div><div><dt>{messages.route.alreadyActive}</dt><dd>{run.already_active_count}</dd></div><div><dt>{messages.route.started}</dt><dd>{formatTimestamp(run.started_at, locale, timeZone)}</dd></div>{run.completed_at && <div><dt>{messages.route.completed}</dt><dd>{formatTimestamp(run.completed_at, locale, timeZone)}</dd></div>}{run.failure_code && <div><dt>{messages.route.failure}</dt><dd>{run.failure_code}</dd></div>}</dl>
            <details className="technical-details"><summary>{messages.route.evidenceReference}</summary><p>{messages.route.run} <code>{run.id}</code>{run.operator_approval_id ? <> · {messages.route.approval} <code>{run.operator_approval_id}</code></> : null}. {messages.route.evidenceCopy}</p></details>
          </article>)}</div>}
        </details>
        <div className="replay-targeted-recovery">
          <div><p className="eyebrow">{messages.targeted.eyebrow}</p><h3>{messages.targeted.title}</h3></div>
          <p className="panel-copy">{messages.targeted.description}</p>
          <form onSubmit={recoverTargetedSignature} className="form-grid">
            <label><span>{messages.targeted.address}</span><input value={recoveryAddress} onChange={(event) => setRecoveryAddress(event.target.value)} placeholder={messages.targeted.addressPlaceholder} autoComplete="off" required /></label>
            <label><span>{messages.targeted.signature}</span><input value={recoverySignature} onChange={(event) => setRecoverySignature(event.target.value)} placeholder={messages.targeted.signaturePlaceholder} autoComplete="off" required /></label>
            <label><span>{messages.targeted.pages} (1–{MAX_TARGETED_RECOVERY_PAGES})</span><input inputMode="numeric" min="1" max={MAX_TARGETED_RECOVERY_PAGES} value={recoveryPages} onChange={(event) => setRecoveryPages(event.target.value)} required /></label>
            <label className="full-field"><span>{messages.targeted.note}</span><textarea value={recoveryNote} onChange={(event) => setRecoveryNote(event.target.value)} maxLength={MAX_OPERATOR_NOTE_LENGTH} rows={2} placeholder={messages.targeted.notePlaceholder} required /></label>
            <button type="submit" disabled={!serviceOnline || isRecovering || !accessToken.trim()}>{isRecovering ? messages.targeted.recovering : messages.targeted.recover}</button>
          </form>
        </div>
      </article>

      <article className="panel replay-evidence-panel" aria-live="polite">
        <div className="panel-heading"><div><p className="eyebrow">{messages.queue.eyebrow}</p><h2>{messages.queue.title}</h2></div>{hasLoaded && <span className="status-pill status-pill-warning">{messages.queue.returned(gaps.length)}</span>}</div>
        {!hasLoaded ? <div className="empty-state"><span className="empty-icon">↻</span><strong>{messages.queue.notLoaded}</strong><p>{messages.queue.notLoadedCopy}</p></div> : gaps.length === 0 ? <div className="empty-state"><span className="empty-icon">✓</span><strong>{messages.queue.noMatches}</strong><p>{messages.queue.noMatchesCopy}</p></div> : <div className="replay-gap-list">{gaps.map((gap) => {
          const acknowledged = gap.operator_acknowledged_at !== null;
          const acknowledgeBusy = busyKey === `${gap.id}:acknowledge`;
          const requeueBusy = busyKey === `${gap.id}:requeue`;
          return <article className="replay-gap-card" key={gap.id}>
            <div className="replay-gap-heading"><div><strong>{displayStatus(gap.status, messages)}</strong><small>{gap.stream_key} · gap {gap.id}</small></div><span className="status-pill status-pill-warning">{messages.gap.attempts(gap.attempts)}</span></div>
            <dl className="summary-list replay-summary"><div><dt>{messages.gap.slotRange}</dt><dd>{gap.start_slot}–{gap.end_slot}</dd></div><div><dt>{messages.gap.observedSlot}</dt><dd>{gap.observed_finalized_slot}</dd></div><div><dt>{messages.gap.reason}</dt><dd>{gap.reason}</dd></div><div><dt>{messages.gap.lastError}</dt><dd>{gap.last_error_code ?? messages.gap.none}</dd></div><div><dt>{messages.gap.created}</dt><dd>{formatTimestamp(gap.created_at, locale, timeZone)}</dd></div>{gap.dead_lettered_at && <div><dt>{messages.gap.deadLettered}</dt><dd>{formatTimestamp(gap.dead_lettered_at, locale, timeZone)}</dd></div>}{acknowledged && <div><dt>{messages.gap.reviewedBy}</dt><dd>{gap.operator_acknowledged_by} · {formatTimestamp(gap.operator_acknowledged_at ?? gap.created_at, locale, timeZone)}</dd></div>}</dl>
            {gap.status === "dead_lettered" && <div className="replay-action-box"><label><span>{messages.gap.note}</span><textarea value={notes[gap.id] ?? ""} onChange={(event) => setNotes((current) => ({ ...current, [gap.id]: event.target.value }))} rows={2} placeholder={acknowledged ? messages.gap.acknowledgedPlaceholder : messages.gap.notePlaceholder} disabled={busyKey !== null} /></label><div className="replay-action-buttons">{!acknowledged && <button type="button" onClick={() => performAction(gap, "acknowledge")} disabled={busyKey !== null}>{acknowledgeBusy ? messages.gap.acknowledgeBusy : messages.gap.acknowledge}</button>}{acknowledged && <button type="button" onClick={() => performAction(gap, "requeue")} disabled={busyKey !== null}>{requeueBusy ? messages.gap.requeueBusy : messages.gap.requeue}</button>}</div></div>}
            {gap.status !== "dead_lettered" && <p className="evidence-note">{messages.gap.noMutation}</p>}
          </article>;
        })}</div>}
      </article>
    </section>
  );
}
