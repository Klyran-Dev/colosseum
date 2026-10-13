import { FormEvent, useState } from "react";
import { fetchWithTimeout, operatorResponseError, REQUEST_TIMEOUT_MS } from "./api";
import { DEFAULT_LOCALE, formatAtomic as formatAtomicValue } from "./formatters";
import {
  buildTaxDocumentFacts,
  emptyDocumentLines,
  parseTaxDocumentSimulation,
  TaxDocumentLineDraft,
  TaxDocumentSimulation,
  validateTaxDocumentLines,
} from "./taxDocumentSimulation";
import { taxDocumentSimulationMessagesFor } from "./taxDocumentSimulationMessages";
import { assetScaleMessagesFor, parseAssetScale } from "./assetScaleMessages";
import { operatorErrorMessage } from "./operatorError";
import TaxTreatmentOutcome from "./TaxTreatmentOutcome";

type Props = {
  apiBase: string;
  serviceOnline: boolean;
  ruleSet: unknown;
  jurisdiction: string;
  locale?: string;
};

export default function TaxDocumentSimulationPanel({ apiBase, serviceOnline, ruleSet, jurisdiction, locale = DEFAULT_LOCALE }: Props) {
  const messages = taxDocumentSimulationMessagesFor(locale);
  const assetScaleMessages = assetScaleMessagesFor(locale);
  const [documentId, setDocumentId] = useState("");
  const [currency, setCurrency] = useState("");
  const [atomicDecimals, setAtomicDecimals] = useState("");
  const [lines, setLines] = useState<TaxDocumentLineDraft[]>(() => emptyDocumentLines());
  const [openLineIndexes, setOpenLineIndexes] = useState<Set<number>>(() => new Set([0]));
  const [simulation, setSimulation] = useState<TaxDocumentSimulation | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [isExpanded, setIsExpanded] = useState(false);

  function updateLine(index: number, patch: Partial<TaxDocumentLineDraft>) {
    setLines((current) => current.map((line, lineIndex) => lineIndex === index ? { ...line, ...patch } : line));
    setSimulation(null);
  }

  function addLine() {
    if (lines.length >= 256) return;
    const nextIndex = lines.length;
    setLines((current) => [...current, { id: "", grossAtomic: "", netAtomic: "", exemptionCodes: "" }]);
    setOpenLineIndexes((current) => new Set([...current, nextIndex]));
  }

  function removeLine(index: number) {
    if (lines.length <= 1) return;
    setLines((current) => current.filter((_, lineIndex) => lineIndex !== index));
    setOpenLineIndexes((current) => new Set([...current].filter((lineIndex) => lineIndex !== index).map((lineIndex) => lineIndex > index ? lineIndex - 1 : lineIndex)));
    setSimulation(null);
  }

  async function simulate(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);
    setNotice(null);
    if (!serviceOnline) {
      setError(messages.state.unavailable);
      return;
    }
    if (ruleSet === null || typeof ruleSet !== "object") {
      setError(messages.state.invalidRule);
      return;
    }
    const lineErrors = validateTaxDocumentLines(lines);
    if (lineErrors.length > 0) {
      setError(lineErrors.join(" "));
      return;
    }
    setIsSubmitting(true);
    try {
      const facts = buildTaxDocumentFacts(documentId, parseAssetScale(atomicDecimals, assetScaleMessages.invalid), currency, jurisdiction, lines, assetScaleMessages.invalid);
      const response = await fetchWithTimeout(`${apiBase}/v1/tax/calculate-document`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ facts, rule_set: ruleSet }),
      }, REQUEST_TIMEOUT_MS.default);
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw operatorResponseError(response, payload, messages.state.failed(response.status));
      setSimulation(parseTaxDocumentSimulation(payload));
      setIsExpanded(true);
      setNotice(messages.state.notice);
    } catch (cause) {
      setSimulation(null);
      setError(operatorErrorMessage(cause, messages.state.failed()));
    } finally {
      setIsSubmitting(false);
    }
  }

  return (
    <details className="authoring-section simulation-section" open={isExpanded} onToggle={(event) => setIsExpanded(event.currentTarget.open)}>
      <summary>{messages.panel.summary}</summary>
      <p className="evidence-note">{messages.panel.evidenceNote}</p>
      <form onSubmit={simulate} className="document-simulation-form">
        <div className="form-grid">
          <label><span>{messages.form.documentId}</span><input value={documentId} onChange={(event) => setDocumentId(event.target.value)} maxLength={64} required /></label>
          <label><span>{messages.form.currency}</span><input value={currency} onChange={(event) => setCurrency(event.target.value.toUpperCase())} maxLength={12} required /></label>
          <label><span>{assetScaleMessages.label}</span><input inputMode="numeric" min="0" max="9" value={atomicDecimals} onChange={(event) => setAtomicDecimals(event.target.value)} aria-describedby="simulation-asset-precision-help" required /><small id="simulation-asset-precision-help" className="field-help">{assetScaleMessages.help}</small></label>
          <p className="field-help full-field">{messages.form.jurisdiction(jurisdiction)}</p>
        </div>
        <div className="document-lines" role="group" aria-labelledby="document-lines-heading">
          <div className="document-lines-heading"><div><h3 id="document-lines-heading">{messages.form.linesHeading}</h3><p className="field-help">{messages.form.linesDescription}</p></div><button type="button" className="secondary-button" onClick={addLine} disabled={lines.length >= 256}>{messages.form.addLine}</button></div>
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
              <span className="document-line-summary-title">{messages.form.line(index + 1)}</span>
              <span className="document-line-summary-id">{line.id.trim() || "—"}</span>
              <span className="document-line-summary-meta">{messages.form.gross} {line.grossAtomic || "—"} · {messages.form.net} {line.netAtomic || "—"}</span>
            </summary>
            <fieldset className="document-line-fields">
              <legend className="sr-only">{messages.form.line(index + 1)}</legend>
            <label><span>{messages.form.lineId}</span><input value={line.id} onChange={(event) => updateLine(index, { id: event.target.value })} maxLength={64} required /></label>
            <label><span>{messages.form.gross}</span><input inputMode="numeric" value={line.grossAtomic} onChange={(event) => updateLine(index, { grossAtomic: event.target.value })} required /></label>
            <label><span>{messages.form.net}</span><input inputMode="numeric" value={line.netAtomic} onChange={(event) => updateLine(index, { netAtomic: event.target.value })} required /></label>
            <label><span>{messages.form.exemptions}</span><input value={line.exemptionCodes} onChange={(event) => updateLine(index, { exemptionCodes: event.target.value })} placeholder={messages.form.exemptionsPlaceholder} /></label>
            <button type="button" className="secondary-button" onClick={() => removeLine(index)} disabled={lines.length <= 1}>{messages.form.remove}</button>
            </fieldset>
          </details>)}
        </div>
        <button type="submit" disabled={!serviceOnline || isSubmitting}>{isSubmitting ? messages.form.calculating : messages.form.submit}</button>
      </form>
      {!serviceOnline && <p className="inline-warning" role="alert">{messages.state.noLocalTotal}</p>}
      {error && <p className="inline-error" role="alert">{error}</p>}
      {notice && <p className="inline-success" role="status">{notice}</p>}
      {simulation && <div className="document-result" role="region" aria-label={messages.state.resultRegion}>
        <div className="total-card"><span>{messages.result.unattestedTax}</span><strong>{formatAtomicValue(simulation.result.total_tax_atomic, simulation.result.currency, locale, simulation.result.atomic_decimals)}</strong><small>{simulation.result.document_id} · {simulation.result.engine_version} · {messages.result.calculationReference} {simulation.calculation_id}</small></div>
        <TaxTreatmentOutcome outcome={simulation.result.treatment_outcome} currency={simulation.result.currency} atomicDecimals={simulation.result.atomic_decimals} locale={locale} formatAtomic={(value, currency, atomicDecimals) => formatAtomicValue(value, currency, locale, atomicDecimals)} />
        <dl className="summary-list"><div><dt>{messages.result.gross}</dt><dd>{formatAtomicValue(simulation.result.gross_atomic, simulation.result.currency, locale, simulation.result.atomic_decimals)}</dd></div><div><dt>{messages.result.netAfterTax}</dt><dd>{formatAtomicValue(simulation.result.total_net_atomic, simulation.result.currency, locale, simulation.result.atomic_decimals)}</dd></div><div><dt>{messages.result.factsDigest}</dt><dd><code>{simulation.result.facts_digest.slice(0, 8).map((byte) => byte.toString(16).padStart(2, "0")).join("")}…</code></dd></div>{simulation.result.fx_conversion && <div><dt>{messages.result.fxEvidence}</dt><dd>{simulation.result.fx_conversion.source_currency} → {simulation.result.fx_conversion.target_currency} · {simulation.result.fx_conversion.provider} · {simulation.result.fx_conversion.quote_id}</dd></div>}</dl>
        <div className="document-result-lines"><h3>{messages.result.canonicalTrace}</h3>{simulation.result.lines.map((line) => <div className="document-result-line" key={line.id}><div><strong>{line.id}</strong><small>{messages.result.gross} {formatAtomicValue(line.gross_atomic, simulation.result.currency, locale, simulation.result.atomic_decimals)} · {messages.result.netAfterTax.toLowerCase()} {formatAtomicValue(line.total_net_atomic, simulation.result.currency, locale, simulation.result.atomic_decimals)}</small></div><strong>{formatAtomicValue(line.total_tax_atomic, simulation.result.currency, locale, simulation.result.atomic_decimals)} {messages.result.tax}</strong></div>)}</div>
      </div>}
    </details>
  );
}
