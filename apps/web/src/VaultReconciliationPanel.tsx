import { FormEvent, useState } from "react";
import { fetchWithTimeout, REQUEST_TIMEOUT_MS, responseError } from "./api";
import { DEFAULT_LOCALE, DEFAULT_TIME_ZONE, formatInteger, formatTimestamp } from "./formatters";
import {
  buildVaultReconciliationRunsUrl,
  buildVaultReconciliationTargetsUrl,
  parseVaultReconciliationRuns,
  parseVaultReconciliationTargetViews,
  VaultReconciliationRun,
  VaultReconciliationTargetView,
} from "./vaultReconciliation";

type VaultReconciliationPanelProps = {
  apiBase: string;
  serviceOnline: boolean;
  accessToken: string;
  locale?: string;
  timeZone?: string;
};

function statusLabel(run: VaultReconciliationRun | null): string {
  if (!run) return "No observation yet";
  if (run.stale) return "Stale — stop and investigate";
  return run.status === "matched" ? "Matched" : run.status === "mismatch" ? "Mismatch — stop and investigate" : "Observation error — not matched";
}

function statusClass(run: VaultReconciliationRun | null): string {
  if (!run) return "status-pill status-pill-warning";
  if (run.stale) return "status-pill status-pill-danger";
  return run.status === "matched" ? "status-pill status-pill-success" : "status-pill status-pill-danger";
}

export default function VaultReconciliationPanel({ apiBase, serviceOnline, accessToken, locale = DEFAULT_LOCALE, timeZone = DEFAULT_TIME_ZONE }: VaultReconciliationPanelProps) {
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
      setError("An in-memory token with tax:vault:read and a governed vault role is required.");
      return;
    }
    setIsLoading(true);
    try {
      const response = await fetchWithTimeout(buildVaultReconciliationTargetsUrl(apiBase), {
        headers: { authorization: `Bearer ${accessToken.trim()}` },
      }, REQUEST_TIMEOUT_MS.default);
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw new Error(responseError(response, payload, `Vault reconciliation lookup failed (${response.status})`));
      const loaded = parseVaultReconciliationTargetViews(payload);
      setTargets(loaded);
      setRuns([]);
      setSelectedTargetId(loaded[0]?.target.id ?? null);
      setHasLoaded(true);
      setNotice(loaded.length === 0 ? "No active balance sources are configured for this account. This is not a matched state." : `Loaded ${loaded.length} verified balance source${loaded.length === 1 ? "" : "s"}.`);
    } catch (cause) {
      setTargets([]);
      setRuns([]);
      setSelectedTargetId(null);
      setHasLoaded(false);
      setError(cause instanceof Error ? cause.message : "Vault reconciliation lookup failed; retry or contact an operator.");
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
      if (!response.ok) throw new Error(responseError(response, payload, `Reconciliation history lookup failed (${response.status})`));
      setRuns(parseVaultReconciliationRuns(payload));
    } catch (cause) {
      setRuns([]);
      setError(cause instanceof Error ? cause.message : "Reconciliation history lookup failed; the target state is not being inferred.");
    } finally {
      setIsLoadingRuns(false);
    }
  }

  const selected = targets.find((view) => view.target.id === selectedTargetId) ?? null;
  const latest = selected?.latest_run ?? null;

  return (
    <section className="vault-layout" aria-label="Vault reconciliation workspace">
      <article className="panel vault-control-panel">
        <div className="panel-heading">
          <div><p className="eyebrow">Balance checks / view only</p><h2>Check tax balances</h2></div>
          <span className="status-pill">View only</span>
        </div>
        <p className="panel-copy">Compare recorded tax balances with the latest verified chain observation. This workspace cannot change balances, release funds, or clear an alert.</p>
        <form onSubmit={loadTargets} className="form-grid">
          <button type="submit" disabled={isLoading || !serviceOnline || !accessToken.trim()}>{isLoading ? "Loading evidence…" : "Load reconciliation targets"}</button>
        </form>
        {!serviceOnline && <p className="inline-warning" role="alert">The API is unavailable. No cached or client-derived reconciliation state is shown.</p>}
        {error && <p className="inline-error" role="alert">{error}</p>}
        {notice && <p className="inline-success" role="status">{notice}</p>}
      </article>

      <article className="panel vault-targets-panel" aria-live="polite">
        <div className="panel-heading"><div><p className="eyebrow">Verified balance sources</p><h2>Active tax accounts</h2></div>{hasLoaded && <span className="status-pill">{targets.length} account{targets.length === 1 ? "" : "s"}</span>}</div>
        {!hasLoaded ? <div className="empty-state"><span className="empty-icon">◎</span><strong>Balance evidence not loaded</strong><p>Sign in to inspect active tax accounts and their latest finalized observations.</p></div> : targets.length === 0 ? <div className="empty-state"><span className="empty-icon">—</span><strong>No balance sources configured</strong><p>Opening balances require a separate approval process. An empty list is not evidence that funds reconcile.</p></div> : <div className="vault-target-list">{targets.map((view) => <button type="button" className={`vault-target-card ${view.target.id === selectedTargetId ? "vault-target-selected" : ""}`} key={view.target.id} onClick={() => loadRuns(view.target.id)} disabled={isLoadingRuns} aria-pressed={view.target.id === selectedTargetId}><span className="vault-target-topline"><strong>{view.target.tax_vault}</strong><span className={statusClass(view.latest_run)}>{statusLabel(view.latest_run)}</span></span><small>Account {view.target.stream_key} · observation slot {view.latest_run?.observed_slot ?? "not observed"}</small><small>Expected {view.latest_run?.expected_balance_atomic === null || view.latest_run?.expected_balance_atomic === undefined ? "—" : formatBalance(view.latest_run.expected_balance_atomic)} · observed {view.latest_run?.observed_balance_atomic === null || view.latest_run?.observed_balance_atomic === undefined ? "—" : formatBalance(view.latest_run.observed_balance_atomic)}</small></button>)}</div>}
      </article>

      {selected && <article className="panel vault-detail-panel">
        <div className="panel-heading"><div><p className="eyebrow">Immutable target identity</p><h2>Evidence detail</h2></div><span className={statusClass(latest)}>{statusLabel(latest)}</span></div>
        <dl className="summary-list vault-summary"><div><dt>Tax vault</dt><dd><code>{selected.target.tax_vault}</code></dd></div><div><dt>Mint / token program</dt><dd><code>{selected.target.mint}</code><br /><code>{selected.target.token_program}</code></dd></div><div><dt>Opening balance</dt><dd>{formatBalance(selected.target.opening_balance_atomic)} at slot {selected.target.opening_balance_slot}</dd></div><div><dt>Opening reference</dt><dd>{selected.target.opening_reference}</dd></div><div><dt>Finalized receipt tax</dt><dd>{formatBalance(selected.target.receipt_tax_atomic)}</dd></div><div><dt>Finalized governed releases</dt><dd>{formatBalance(selected.target.release_atomic)}</dd></div></dl>
        {latest?.stale && <p className="inline-error" role="alert">The latest finalized observation is stale and cannot prove the current vault balance. Stop release operations until a fresh observation is recorded.</p>}
        {!latest?.stale && latest?.status === "mismatch" && <p className="inline-error" role="alert">The latest finalized observation does not match the evidence-derived expectation. Stop release operations and follow the vault incident runbook.</p>}
        {!latest?.stale && latest?.status === "error" && <p className="inline-error" role="alert">The latest observation could not be proven. Error code: {latest.error_code ?? "unknown"}. This target is not matched.</p>}
        <div className="panel-heading vault-history-heading"><div><p className="eyebrow">Append-only history</p><h3>Recent runs</h3></div>{isLoadingRuns && <span className="status-pill">Loading…</span>}</div>
        {runs.length === 0 && !isLoadingRuns ? <p className="panel-copy">Run history has not been loaded for this target.</p> : <div className="vault-run-list">{runs.map((run) => <div className="vault-run-row" key={run.id}><span className={statusClass(run)}>{run.stale ? "stale" : run.status}</span><span>expected {run.expected_balance_atomic === null ? "—" : formatBalance(run.expected_balance_atomic)} · observed {run.observed_balance_atomic === null ? "—" : formatBalance(run.observed_balance_atomic)}</span><time dateTime={run.created_at}>{formatTimestamp(run.created_at, locale, timeZone)}</time>{run.error_code && <code>{run.error_code}</code>}</div>)}</div>}
      </article>}
    </section>
  );
}
