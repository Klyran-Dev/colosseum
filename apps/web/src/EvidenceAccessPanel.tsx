import { FormEvent, useState } from "react";
import { fetchWithTimeout, operatorResponseError, REQUEST_TIMEOUT_MS } from "./api";
import { DEFAULT_LOCALE, DEFAULT_TIME_ZONE, formatTimestamp } from "./formatters";
import { evidenceAccessMessagesFor, type EvidenceAccessMessages } from "./evidenceAccessMessages";
import {
  buildEvidenceAccessAuditUrl,
  MAX_EVIDENCE_ACCESS_AUDIT_LIST_LIMIT,
  parseEvidenceAccessAuditList,
  type EvidenceAccessAuditEvent,
} from "./evidenceAccess";
import { operatorErrorMessage, operatorFacingError } from "./operatorError";

type EvidenceAccessPanelProps = {
  apiBase: string;
  serviceOnline: boolean;
  accessToken: string;
  locale?: string;
  timeZone?: string;
};

function digestText(digest: number[] | null): string | null {
  return digest?.map((byte) => byte.toString(16).padStart(2, "0")).join("") ?? null;
}

function outcomeLabel(event: EvidenceAccessAuditEvent, messages: EvidenceAccessMessages): string {
  return event.outcome === "allowed" ? messages.ledger.allowed : messages.ledger.denied;
}

export default function EvidenceAccessPanel({
  apiBase,
  serviceOnline,
  accessToken,
  locale = DEFAULT_LOCALE,
  timeZone = DEFAULT_TIME_ZONE,
}: EvidenceAccessPanelProps) {
  const messages = evidenceAccessMessagesFor(locale);
  const [calculationId, setCalculationId] = useState("");
  const [limit, setLimit] = useState("50");
  const [events, setEvents] = useState<EvidenceAccessAuditEvent[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [hasLoaded, setHasLoaded] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  async function load(event?: FormEvent<HTMLFormElement>) {
    event?.preventDefault();
    setError(null);
    setNotice(null);
    setIsLoading(true);
    try {
      if (!accessToken.trim()) throw operatorFacingError(messages.control.signIn);
      const numericLimit = Number(limit);
      const url = buildEvidenceAccessAuditUrl(apiBase, numericLimit, calculationId.trim() || undefined);
      const response = await fetchWithTimeout(url, {
        headers: { authorization: `Bearer ${accessToken.trim()}` },
      }, REQUEST_TIMEOUT_MS.default);
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw operatorResponseError(response, payload, messages.common.httpError(messages.common.error, response.status));
      const parsed = parseEvidenceAccessAuditList(payload);
      setEvents(parsed);
      setHasLoaded(true);
      setNotice(messages.ledger.returned(parsed.length));
    } catch (cause) {
      setEvents([]);
      setHasLoaded(false);
      setError(operatorErrorMessage(cause, messages.common.error));
    } finally {
      setIsLoading(false);
    }
  }

  return (
    <section className="evidence-layout" aria-label={messages.control.ariaLabel}>
      <article className="panel evidence-control-panel">
        <div className="panel-heading">
          <div><p className="eyebrow">{messages.control.eyebrow}</p><h2>{messages.control.title}</h2></div>
          <span className="status-pill">{messages.control.badge}</span>
        </div>
        <p className="panel-copy">{messages.control.description}</p>
        <form onSubmit={load} className="form-grid">
          <label className="full-field">
            <span>{messages.control.calculationFilter}</span>
            <input value={calculationId} onChange={(event) => setCalculationId(event.target.value)} placeholder={messages.control.calculationPlaceholder} autoComplete="off" />
            <small className="field-help">{messages.control.calculationHelp}</small>
          </label>
          <label>
            <span>{messages.control.limit} (1–{MAX_EVIDENCE_ACCESS_AUDIT_LIST_LIMIT})</span>
            <input inputMode="numeric" min="1" max={MAX_EVIDENCE_ACCESS_AUDIT_LIST_LIMIT} value={limit} onChange={(event) => setLimit(event.target.value)} required />
          </label>
          <button type="submit" disabled={!serviceOnline || isLoading || !accessToken.trim()}>{isLoading ? messages.control.loading : messages.control.load}</button>
        </form>
        {!serviceOnline && <p className="inline-warning" role="alert">{messages.control.unavailable}</p>}
        {error && <p className="inline-error" role="alert">{error}</p>}
        {notice && <p className="inline-success" role="status">{notice}</p>}
        <div className="evidence-authority-note"><strong>{messages.ledger.authorityTitle}</strong><span>{messages.ledger.authorityCopy}</span></div>
      </article>

      <article className="panel evidence-ledger-panel" aria-live="polite">
        <div className="panel-heading">
          <div><p className="eyebrow">{messages.ledger.eyebrow}</p><h2>{messages.ledger.title}</h2></div>
          {hasLoaded && <span className="status-pill">{messages.ledger.returned(events.length)}</span>}
        </div>
        {!hasLoaded ? (
          <div className="empty-state"><span className="empty-icon">⌕</span><strong>{messages.ledger.notLoaded}</strong><p>{messages.ledger.notLoadedCopy}</p></div>
        ) : events.length === 0 ? (
          <div className="empty-state"><span className="empty-icon">✓</span><strong>{messages.ledger.empty}</strong><p>{messages.ledger.emptyCopy}</p></div>
        ) : (
          <div className="evidence-event-list">
            {events.map((auditEvent) => {
              const digest = digestText(auditEvent.facts_digest);
              return <article className="evidence-event-card" key={auditEvent.id}>
                <div className="evidence-event-heading">
                  <div><strong>{outcomeLabel(auditEvent, messages)}</strong><small>{auditEvent.purpose}</small></div>
                  <span className={`status-pill ${auditEvent.outcome === "denied" ? "status-pill-warning" : ""}`}>{auditEvent.outcome === "allowed" ? "✓" : "!"}</span>
                </div>
                <dl className="summary-list evidence-summary">
                  <div><dt>{messages.ledger.calculation}</dt><dd><code>{auditEvent.calculation_id}</code></dd></div>
                  <div><dt>{messages.ledger.actor}</dt><dd>{auditEvent.actor_subject}</dd></div>
                  <div><dt>{messages.ledger.purpose}</dt><dd>{auditEvent.purpose}</dd></div>
                  <div><dt>{messages.ledger.occurred}</dt><dd>{formatTimestamp(auditEvent.occurred_at, locale, timeZone)}</dd></div>
                </dl>
                <details className="technical-details">
                  <summary>{messages.ledger.technical}</summary>
                  <p>{messages.ledger.auditId} <code>{auditEvent.id}</code> · {messages.ledger.requestId} <code>{auditEvent.request_id}</code>.</p>
                  <p>{messages.ledger.factsDigest}: <code>{digest ?? messages.ledger.digestUnavailable}</code></p>
                </details>
              </article>;
            })}
          </div>
        )}
      </article>
    </section>
  );
}
