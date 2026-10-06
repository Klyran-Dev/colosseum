import { FormEvent, useState } from "react";
import { fetchWithTimeout, operatorResponseError, REQUEST_TIMEOUT_MS } from "./api";
import { DEFAULT_LOCALE, DEFAULT_TIME_ZONE, formatInteger, formatTimestamp } from "./formatters";
import { vaultReconciliationMessagesFor, type VaultReconciliationMessages } from "./vaultReconciliationMessages";
import {
  buildVaultReconciliationRunsUrl,
  buildVaultReconciliationTargetsUrl,
  parseVaultReconciliationRuns,
  parseVaultReconciliationTargetViews,
  VaultReconciliationRun,
  VaultReconciliationTargetView,
} from "./vaultReconciliation";
import { operatorErrorMessage } from "./operatorError";

type VaultReconciliationPanelProps = {
  apiBase: string;
  serviceOnline: boolean;
  accessToken: string;
  locale?: string;
  timeZone?: string;
};

function statusLabel(run: VaultReconciliationRun | null, messages: VaultReconciliationMessages): string {
  if (!run) return messages.status.noObservation;
  if (run.stale) return messages.status.stale;
  return run.status === "matched" ? messages.status.matched : run.status === "mismatch" ? messages.status.mismatch : messages.status.error;
}

function statusClass(run: VaultReconciliationRun | null): string {
  if (!run) return "status-pill status-pill-warning";
  if (run.stale) return "status-pill status-pill-danger";
  return run.status === "matched" ? "status-pill status-pill-success" : "status-pill status-pill-danger";
}

export default function VaultReconciliationPanel({ apiBase, serviceOnline, accessToken, locale = DEFAULT_LOCALE, timeZone = DEFAULT_TIME_ZONE }: VaultReconciliationPanelProps) {
  const messages = vaultReconciliationMessagesFor(locale);
  const formatBalance = (value: string) => formatInteger(value, locale);
  const [targets, setTargets] = useState<VaultReconciliationTargetView[]>([]);
  const [selectedTargetId, setSelectedTargetId] = useState<string | null>(null);
  const [runs, setRuns] = useState<VaultReconciliationRun[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [isLoadingRuns, setIsLoadingRuns] = useState(false);
  const [hasLoaded, setHasLoaded] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  async function loadTargets(event?: FormEvent<HTMLFormElement>) {
    event?.preventDefault();
    setError(null);
    setNotice(null);
    if (!accessToken.trim()) {
      setError(messages.common.lookupError);
      return;
    }
    setIsLoading(true);
    try {
      const response = await fetchWithTimeout(buildVaultReconciliationTargetsUrl(apiBase), {
        headers: { authorization: `Bearer ${accessToken.trim()}` },
      }, REQUEST_TIMEOUT_MS.default);
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw operatorResponseError(response, payload, messages.common.httpError(messages.common.lookupError, response.status));
      const loaded = parseVaultReconciliationTargetViews(payload);
      setTargets(loaded);
      setRuns([]);
      setSelectedTargetId(loaded[0]?.target.id ?? null);
      setHasLoaded(true);
      setNotice(loaded.length === 0 ? messages.notice.empty : messages.notice.loaded(loaded.length));
    } catch (cause) {
      setTargets([]);
      setRuns([]);
      setSelectedTargetId(null);
      setHasLoaded(false);
      setError(operatorErrorMessage(cause, messages.common.lookupError));
    } finally {
      setIsLoading(false);
    }
  }

  async function loadRuns(targetId: string) {
    setSelectedTargetId(targetId);
    setError(null);
    setNotice(null);
    setIsLoadingRuns(true);
    try {
      const response = await fetchWithTimeout(buildVaultReconciliationRunsUrl(apiBase, targetId), {
        headers: { authorization: `Bearer ${accessToken.trim()}` },
      }, REQUEST_TIMEOUT_MS.default);
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw operatorResponseError(response, payload, messages.common.httpError(messages.common.historyError, response.status));
      setRuns(parseVaultReconciliationRuns(payload));
    } catch (cause) {
      setRuns([]);
      setError(operatorErrorMessage(cause, messages.common.historyError));
    } finally {
      setIsLoadingRuns(false);
    }
  }

  const selected = targets.find((view) => view.target.id === selectedTargetId) ?? null;
  const latest = selected?.latest_run ?? null;

  return (
    <section className="vault-layout" aria-label={messages.control.ariaLabel}>
      <article className="panel vault-control-panel">
        <div className="panel-heading">
          <div><p className="eyebrow">{messages.control.eyebrow}</p><h2>{messages.control.title}</h2></div>
          <span className="status-pill">{messages.control.badge}</span>
        </div>
        <p className="panel-copy">{messages.control.description}</p>
        <form onSubmit={loadTargets} className="form-grid">
          <button type="submit" disabled={isLoading || !serviceOnline || !accessToken.trim()}>{isLoading ? messages.control.loading : messages.control.load}</button>
        </form>
        {!serviceOnline && <p className="inline-warning" role="alert">{messages.control.unavailable}</p>}
        {error && <p className="inline-error" role="alert">{error}</p>}
        {notice && <p className="inline-success" role="status">{notice}</p>}
      </article>

      <article className="panel vault-targets-panel" aria-live="polite">
        <div className="panel-heading"><div><p className="eyebrow">{messages.targets.eyebrow}</p><h2>{messages.targets.title}</h2></div>{hasLoaded && <span className="status-pill">{messages.targets.count(targets.length)}</span>}</div>
        {!hasLoaded ? <div className="empty-state"><span className="empty-icon">◎</span><strong>{messages.targets.notLoaded}</strong><p>{messages.targets.notLoadedCopy}</p></div> : targets.length === 0 ? <div className="empty-state"><span className="empty-icon">—</span><strong>{messages.targets.empty}</strong><p>{messages.targets.emptyCopy}</p></div> : <div className="vault-target-list">{targets.map((view) => <button type="button" className={`vault-target-card ${view.target.id === selectedTargetId ? "vault-target-selected" : ""}`} key={view.target.id} onClick={() => loadRuns(view.target.id)} disabled={isLoadingRuns} aria-pressed={view.target.id === selectedTargetId}><span className="vault-target-topline"><strong>{view.target.tax_vault}</strong><span className={statusClass(view.latest_run)}>{statusLabel(view.latest_run, messages)}</span></span><small>{messages.targets.account} {view.target.stream_key} · {messages.targets.observationSlot} {view.latest_run?.observed_slot ?? "—"}</small><small>{messages.targets.expected} {view.latest_run?.expected_balance_atomic === null || view.latest_run?.expected_balance_atomic === undefined ? "—" : formatBalance(view.latest_run.expected_balance_atomic)} · {messages.targets.observed} {view.latest_run?.observed_balance_atomic === null || view.latest_run?.observed_balance_atomic === undefined ? "—" : formatBalance(view.latest_run.observed_balance_atomic)}</small></button>)}</div>}
      </article>

      {selected && <article className="panel vault-detail-panel">
        <div className="panel-heading"><div><p className="eyebrow">{messages.detail.eyebrow}</p><h2>{messages.detail.title}</h2></div><span className={statusClass(latest)}>{statusLabel(latest, messages)}</span></div>
        <dl className="summary-list vault-summary"><div><dt>{messages.detail.taxVault}</dt><dd><code>{selected.target.tax_vault}</code></dd></div><div><dt>{messages.detail.mintProgram}</dt><dd><code>{selected.target.mint}</code><br /><code>{selected.target.token_program}</code></dd></div><div><dt>{messages.detail.openingBalance}</dt><dd>{formatBalance(selected.target.opening_balance_atomic)} {messages.detail.atSlot} {selected.target.opening_balance_slot}</dd></div><div><dt>{messages.detail.openingReference}</dt><dd>{selected.target.opening_reference}</dd></div><div><dt>{messages.detail.receiptTax}</dt><dd>{formatBalance(selected.target.receipt_tax_atomic)}</dd></div><div><dt>{messages.detail.releases}</dt><dd>{formatBalance(selected.target.release_atomic)}</dd></div></dl>
        {latest?.stale && <p className="inline-error" role="alert">{messages.detail.staleAlert}</p>}
        {!latest?.stale && latest?.status === "mismatch" && <p className="inline-error" role="alert">{messages.detail.mismatchAlert}</p>}
        {!latest?.stale && latest?.status === "error" && <p className="inline-error" role="alert">{messages.detail.errorAlert} {latest.error_code ?? messages.detail.unknown}. This target is not matched.</p>}
        <div className="panel-heading vault-history-heading"><div><p className="eyebrow">{messages.detail.history}</p><h3>{messages.detail.recentRuns}</h3></div>{isLoadingRuns && <span className="status-pill">{messages.detail.loading}</span>}</div>
        {runs.length === 0 && !isLoadingRuns ? <p className="panel-copy">{messages.detail.notLoaded}</p> : <div className="vault-run-list">{runs.map((run) => <div className="vault-run-row" key={run.id}><span className={statusClass(run)}>{run.stale ? messages.status.staleShort : run.status === "matched" ? messages.status.matched : run.status === "mismatch" ? messages.status.mismatch : messages.status.error}</span><span>{messages.targets.expected} {run.expected_balance_atomic === null ? "—" : formatBalance(run.expected_balance_atomic)} · {messages.targets.observed} {run.observed_balance_atomic === null ? "—" : formatBalance(run.observed_balance_atomic)}</span><time dateTime={run.created_at}>{formatTimestamp(run.created_at, locale, timeZone)}</time>{run.error_code && <code>{run.error_code}</code>}</div>)}</div>}
      </article>}
    </section>
  );
}
