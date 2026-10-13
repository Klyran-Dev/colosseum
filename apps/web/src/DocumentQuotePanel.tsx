import { FormEvent, useEffect, useState } from "react";
import { fetchWithTimeout, operatorResponseError, REQUEST_TIMEOUT_MS } from "./api";
import { createClientUuid } from "./clientIdentity";
import { DEFAULT_LOCALE, DEFAULT_TIME_ZONE, formatEpoch } from "./formatters";
import { documentQuoteMessagesFor } from "./documentQuoteMessages";
import { assetScaleMessagesFor, parseAssetScale } from "./assetScaleMessages";
import { operatorErrorMessage, operatorFacingError } from "./operatorError";
import {
  buildTaxDocumentFacts,
  emptyDocumentLines,
  parseTaxDocumentQuote,
  TaxDocumentLineDraft,
  TaxDocumentQuote,
  validateTaxDocumentLines,
} from "./taxDocumentSimulation";
import TaxTreatmentOutcome from "./TaxTreatmentOutcome";

type Props = {
  apiBase: string;
  serviceOnline: boolean;
  accessToken: string;
  formatAtomic: (value: string, currency: string, atomicDecimals: number) => string;
  locale?: string;
  timeZone?: string;
  onOpenCheckout: (calculationId: string) => void;
};

function parseEvidence(value: string, errors: ReturnType<typeof documentQuoteMessagesFor>["errors"]): unknown[] {
  if (new TextEncoder().encode(value).length > 256 * 1024) throw operatorFacingError(errors.evidenceTooLarge);
  let parsed: unknown;
  try {
    parsed = JSON.parse(value);
  } catch {
    throw operatorFacingError(errors.evidenceInvalidJson);
  }
  if (!Array.isArray(parsed) || parsed.length === 0 || parsed.length > 64) throw operatorFacingError(errors.evidenceArrayBound);
  return parsed;
}

function positiveVersion(value: string, errors: ReturnType<typeof documentQuoteMessagesFor>["errors"]): number {
  const version = BigInt(value);
  if (version < 1n || version > BigInt(0x7fffffff)) throw operatorFacingError(errors.ruleVersionInvalid);
  return Number(version);
}

export default function DocumentQuotePanel({
  apiBase,
  serviceOnline,
  accessToken,
  formatAtomic,
  locale = DEFAULT_LOCALE,
  timeZone = DEFAULT_TIME_ZONE,
  onOpenCheckout,
}: Props) {
  const messages = documentQuoteMessagesFor(locale);
  const assetScaleMessages = assetScaleMessagesFor(locale);
  const [documentId, setDocumentId] = useState("");
  const [currency, setCurrency] = useState("");
  const [atomicDecimals, setAtomicDecimals] = useState("");
  const [jurisdiction, setJurisdiction] = useState("");
  const [ruleSetId, setRuleSetId] = useState("");
  const [ruleVersion, setRuleVersion] = useState("");
  const [regime, setRegime] = useState("");
  const [idempotencyKey, setIdempotencyKey] = useState(() => createClientUuid());
  const [jurisdictionEvidenceJson, setJurisdictionEvidenceJson] = useState("");
  const [lines, setLines] = useState<TaxDocumentLineDraft[]>(() => emptyDocumentLines());
  const [openLineIndexes, setOpenLineIndexes] = useState<Set<number>>(() => new Set([0]));
  const [quote, setQuote] = useState<TaxDocumentQuote | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const interactionDisabled = isSubmitting || !serviceOnline || !accessToken.trim();

  useEffect(() => {
    setIdempotencyKey(createClientUuid());
  }, [documentId, currency, jurisdiction, ruleSetId, ruleVersion, regime, jurisdictionEvidenceJson, lines]);

  function updateLine(index: number, patch: Partial<TaxDocumentLineDraft>) {
    setLines((current) => current.map((line, lineIndex) => lineIndex === index ? { ...line, ...patch } : line));
    setQuote(null);
  }

  function addLine() {
    if (lines.length >= 256) return;
    const nextIndex = lines.length;
    setLines((current) => [...current, { id: "", grossAtomic: "", netAtomic: "", exemptionCodes: "" }]);
    setOpenLineIndexes((current) => new Set([...current, nextIndex]));
    setQuote(null);
  }

  function removeLine(index: number) {
    if (lines.length <= 1) return;
    setLines((current) => current.filter((_, lineIndex) => lineIndex !== index));
    setOpenLineIndexes((current) => new Set([...current].filter((lineIndex) => lineIndex !== index).map((lineIndex) => lineIndex > index ? lineIndex - 1 : lineIndex)));
    setQuote(null);
  }

  async function createQuote(event?: FormEvent<HTMLFormElement>) {
    event?.preventDefault();
    setError(null);
    setNotice(null);
    setQuote(null);
    if (!serviceOnline) return setError(messages.errors.serviceUnavailable);
    if (!accessToken.trim()) return setError(messages.errors.signInRequired);
    const lineErrors = validateTaxDocumentLines(lines);
    if (lineErrors.length > 0) return setError(lineErrors.join(" "));
    let facts: ReturnType<typeof buildTaxDocumentFacts>;
    let evidence: unknown[];
    let version: number;
    try {
      facts = buildTaxDocumentFacts(documentId, parseAssetScale(atomicDecimals, assetScaleMessages.invalid), currency, jurisdiction, lines, assetScaleMessages.invalid);
      evidence = parseEvidence(jurisdictionEvidenceJson, messages.errors);
      version = positiveVersion(ruleVersion, messages.errors);
      if (!ruleSetId.trim()) throw operatorFacingError(messages.errors.approvedTaxRuleRequired);
      if (!regime.trim()) throw operatorFacingError(messages.errors.taxCategoryRequired);
      if (!idempotencyKey.trim()) throw operatorFacingError(messages.errors.idempotencyRequired);
    } catch (cause) {
      return setError(operatorErrorMessage(cause, messages.errors.invalidFields));
    }
    setIsSubmitting(true);
    try {
      const response = await fetchWithTimeout(`${apiBase}/v1/tax/document-quote`, {
        method: "POST",
        headers: { "content-type": "application/json", authorization: `Bearer ${accessToken.trim()}` },
        body: JSON.stringify({
          schema: "solana-tax-protocol/tax-document/v1",
          idempotency_key: idempotencyKey.trim(),
          rule_set_id: ruleSetId.trim(),
          rule_version: version,
          jurisdiction: jurisdiction.trim(),
          regime: regime.trim(),
          facts,
          jurisdiction_evidence: evidence,
        }),
      }, REQUEST_TIMEOUT_MS.default);
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw operatorResponseError(response, payload, messages.errors.quoteRequestFailed(response.status));
      const stored = parseTaxDocumentQuote(payload);
      setQuote(stored);
      setNotice(stored.status === "attested"
        ? messages.result.attestedNotice
        : messages.result.calculatedNotice);
    } catch (cause) {
      setError(operatorErrorMessage(cause, messages.errors.quoteFailed));
    } finally {
      setIsSubmitting(false);
    }
  }

  return <section className="workspace-grid" aria-labelledby="document-quote-title">
    <article className="panel calculator-panel">
      <div className="panel-heading"><div><p className="eyebrow">{messages.form.eyebrow}</p><h2 id="document-quote-title">{messages.form.title}</h2></div><span className="status-pill">{messages.form.status}</span></div>
      <p className="panel-copy">{messages.form.description}</p>
      {!accessToken.trim() && <div className="workflow-lock" role="note"><strong>{messages.form.signInRequired}</strong><p>{messages.form.signInHelp}</p></div>}
      <form onSubmit={createQuote} className="form-grid">
        <div className="route-evidence full-field" role="status" aria-label={messages.form.sessionStatus}><strong>{accessToken.trim() ? messages.form.signedInSession : messages.form.signInRequired}</strong><small>{accessToken.trim() ? messages.form.signedInHelp : messages.form.signInHelp}</small></div>
        <fieldset className="workflow-fieldset" disabled={interactionDisabled}>
        <legend className="sr-only">{messages.form.title}</legend>
        <label><span>{messages.form.fields.documentId}</span><input value={documentId} onChange={(event) => { setDocumentId(event.target.value); setQuote(null); }} maxLength={64} required /></label>
        <label><span>{messages.form.fields.currency}</span><input value={currency} onChange={(event) => { setCurrency(event.target.value.toUpperCase()); setQuote(null); }} maxLength={12} required /></label>
        <label><span>{messages.form.fields.jurisdiction}</span><input value={jurisdiction} onChange={(event) => { setJurisdiction(event.target.value.toUpperCase()); setQuote(null); }} maxLength={64} required /></label>
        <label><span>{messages.form.fields.category}</span><input value={regime} onChange={(event) => { setRegime(event.target.value); setQuote(null); }} maxLength={64} placeholder={messages.form.fields.categoryPlaceholder} required /></label>
        <label><span>{messages.form.fields.approvedRule}</span><input value={ruleSetId} onChange={(event) => { setRuleSetId(event.target.value); setQuote(null); }} placeholder={messages.form.fields.rulePlaceholder} required /></label>
        <label><span>{messages.form.fields.ruleVersion}</span><input inputMode="numeric" value={ruleVersion} onChange={(event) => { setRuleVersion(event.target.value); setQuote(null); }} required /></label>
        <div className="route-evidence full-field"><strong>{messages.form.retryTitle}</strong><small>{messages.form.retryDescription}</small></div>
        <details className="route-evidence full-field">
          <summary>{messages.form.location.summary}</summary>
          <p className="field-help">{messages.form.location.description}</p>
          <label htmlFor="jurisdiction-evidence"><span>{messages.form.location.label}</span><textarea id="jurisdiction-evidence" value={jurisdictionEvidenceJson} onChange={(event) => { setJurisdictionEvidenceJson(event.target.value); setQuote(null); }} rows={7} autoComplete="off" spellCheck={false} placeholder={messages.form.location.placeholder} required /><small className="field-help">{messages.form.location.help}</small></label>
        </details>
        <div className="document-lines full-field" role="group" aria-labelledby="document-quote-lines-heading">
          <label><span>{assetScaleMessages.label}</span><input inputMode="numeric" min="0" max="9" value={atomicDecimals} onChange={(event) => setAtomicDecimals(event.target.value)} aria-describedby="document-asset-precision-help" required /><small id="document-asset-precision-help" className="field-help">{assetScaleMessages.help}</small></label>
          <div className="document-lines-heading"><div><h3 id="document-quote-lines-heading">{messages.form.lines.heading}</h3><p className="field-help">{messages.form.lines.description}</p></div><button type="button" className="secondary-button" onClick={addLine} disabled={interactionDisabled || lines.length >= 256}>{messages.form.lines.add}</button></div>
          {lines.map((line, index) => <details
            className="document-line"
            key={`${line.id}-${index}`}
            open={openLineIndexes.has(index)}
            onToggle={(event) => {
              const isOpen = event.currentTarget.open;
              setOpenLineIndexes((current) => {
                const next = new Set(current);
                if (isOpen) next.add(index);
                else next.delete(index);
                return next;
              });
            }}
          >
            <summary className="document-line-summary">
              <span className="document-line-summary-title">{messages.form.lines.line(index + 1)}</span>
              <span className="document-line-summary-id">{line.id.trim() || "—"}</span>
              <span className="document-line-summary-meta">{messages.form.lines.beforeTax} {line.grossAtomic || "—"} · {messages.form.lines.afterTax} {line.netAtomic || "—"}</span>
            </summary>
            <fieldset className="document-line-fields">
              <legend className="sr-only">{messages.form.lines.line(index + 1)}</legend>
            <label><span>{messages.form.lines.id}</span><input value={line.id} onChange={(event) => updateLine(index, { id: event.target.value })} maxLength={64} placeholder={messages.form.lines.idPlaceholder} required /></label>
            <label><span>{messages.form.lines.beforeTax}</span><input inputMode="numeric" value={line.grossAtomic} onChange={(event) => updateLine(index, { grossAtomic: event.target.value })} placeholder={messages.form.lines.beforeTaxPlaceholder} required /></label>
            <label><span>{messages.form.lines.afterTax}</span><input inputMode="numeric" value={line.netAtomic} onChange={(event) => updateLine(index, { netAtomic: event.target.value })} placeholder={messages.form.lines.afterTaxPlaceholder} required /></label>
            <label><span>{messages.form.lines.exemptions}</span><input value={line.exemptionCodes} onChange={(event) => updateLine(index, { exemptionCodes: event.target.value })} placeholder={messages.form.lines.optional} /></label>
            <button type="button" className="secondary-button" onClick={() => removeLine(index)} disabled={interactionDisabled || lines.length <= 1}>{messages.form.lines.remove}</button>
            </fieldset>
          </details>)}
        </div>
        <button type="submit" disabled={interactionDisabled}>{isSubmitting ? messages.form.calculating : messages.form.submit}</button>
        </fieldset>
      </form>
      {!serviceOnline && <p className="inline-warning" role="alert">{messages.form.offline}</p>}
      {error && <p className="inline-error" role="alert">{error} <button type="button" className="inline-retry" onClick={() => { void createQuote(); }} disabled={interactionDisabled}>{messages.form.retry}</button></p>}
      {notice && <p className="inline-success" role="status">{notice}</p>}
    </article>

    <article className="panel result-panel" aria-live="polite">
      <div className="panel-heading"><div><p className="eyebrow">{messages.result.eyebrow}</p><h2>{messages.result.title}</h2></div>{quote && <span className={`status-pill ${quote.status === "attested" ? "status-pill-success" : "status-pill-warning"}`}>{quote.status === "attested" ? messages.result.verified : messages.result.needsVerification}</span>}</div>
      {!quote ? <div className="empty-state"><span className="empty-icon">∑</span><strong>{messages.result.emptyTitle}</strong><p>{messages.result.emptyCopy}</p></div> : <div className="result-content">
        <div className="total-card"><span>{messages.result.totalTax}</span><strong>{formatAtomic(quote.result.total_tax_atomic, quote.result.currency, quote.result.atomic_decimals)}</strong><small>{messages.result.calculationId} {quote.calculation_id}</small></div>
        <TaxTreatmentOutcome outcome={quote.result.treatment_outcome} currency={quote.result.currency} atomicDecimals={quote.result.atomic_decimals} locale={locale} formatAtomic={formatAtomic} />
        <dl className="summary-list"><div><dt>{messages.result.document}</dt><dd>{quote.result.document_id}</dd></div><div><dt>{messages.result.gross}</dt><dd>{formatAtomic(quote.result.gross_atomic, quote.result.currency, quote.result.atomic_decimals)}</dd></div><div><dt>{messages.result.netAfterTax}</dt><dd>{formatAtomic(quote.result.total_net_atomic, quote.result.currency, quote.result.atomic_decimals)}</dd></div><div><dt>{messages.result.jurisdiction}</dt><dd>{quote.result.jurisdiction}</dd></div><div><dt>{messages.result.taxRule}</dt><dd>{quote.result.rule_set_id} v{quote.result.rule_version}</dd></div>{quote.result.fx_conversion && <div><dt>{messages.result.currencyConversion}</dt><dd>{quote.result.fx_conversion.source_currency} → {quote.result.fx_conversion.target_currency} · {quote.result.fx_conversion.provider} · {quote.result.fx_conversion.quote_id}</dd></div>}<div><dt>{messages.result.expires}</dt><dd>{formatEpoch(quote.expires_at_unix, locale, timeZone)}</dd></div></dl>
        <details className="technical-details"><summary>{messages.result.recordDetails}</summary><dl className="summary-list"><div><dt>{messages.result.calculationReference}</dt><dd><code>{quote.calculation_id}</code></dd></div><div><dt>{messages.result.factsFingerprint}</dt><dd><code>{quote.result.facts_digest.slice(0, 8).map((byte) => byte.toString(16).padStart(2, "0")).join("")}…</code></dd></div></dl></details>
        <div className="document-result-lines"><h3>{messages.result.invoiceLines}</h3>{quote.result.lines.map((line) => <div className="document-result-line" key={line.id}><div><strong>{line.id}</strong><small>{messages.result.beforeTax} {formatAtomic(line.gross_atomic, quote.result.currency, quote.result.atomic_decimals)} · {messages.result.afterTax} {formatAtomic(line.total_net_atomic, quote.result.currency, quote.result.atomic_decimals)}</small></div><strong>{formatAtomic(line.total_tax_atomic, quote.result.currency, quote.result.atomic_decimals)} {messages.result.tax}</strong></div>)}</div>
        <button type="button" onClick={() => onOpenCheckout(quote.calculation_id)}>{messages.result.openCheckout}</button>
        {quote.status !== "attested" && <p className="inline-warning">{messages.result.notAttested}</p>}
      </div>}
    </article>
  </section>;
}
