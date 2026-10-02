import { FormEvent, useState } from "react";
import { fetchWithTimeout, REQUEST_TIMEOUT_MS, responseError } from "./api";
import { DEFAULT_LOCALE, formatAtomic as formatAtomicValue } from "./formatters";
import {
  buildTaxDocumentFacts,
  defaultDocumentLines,
  parseTaxDocumentSimulation,
  TaxDocumentLineDraft,
  TaxDocumentSimulation,
  validateTaxDocumentLines,
} from "./taxDocumentSimulation";

type Props = {
  apiBase: string;
  serviceOnline: boolean;
  ruleSet: unknown;
  jurisdiction: string;
  locale?: string;
};

export default function TaxDocumentSimulationPanel({ apiBase, serviceOnline, ruleSet, jurisdiction, locale = DEFAULT_LOCALE }: Props) {
  const [documentId, setDocumentId] = useState("invoice-simulation-1");
  const [currency, setCurrency] = useState("USD");
  const [lines, setLines] = useState<TaxDocumentLineDraft[]>(() => defaultDocumentLines());
  const [simulation, setSimulation] = useState<TaxDocumentSimulation | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);

  function updateLine(index: number, patch: Partial<TaxDocumentLineDraft>) {
    setLines((current) => current.map((line, lineIndex) => lineIndex === index ? { ...line, ...patch } : line));
    setSimulation(null);
  }

  function addLine() {
    if (lines.length >= 256) return;
    setLines((current) => [...current, { id: `line-${current.length + 1}`, grossAtomic: "1000", netAtomic: "1000", exemptionCodes: "" }]);
  }

  function removeLine(index: number) {
    if (lines.length <= 1) return;
    setLines((current) => current.filter((_, lineIndex) => lineIndex !== index));
    setSimulation(null);
  }

  async function simulate(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);
    setNotice(null);
    if (!serviceOnline) {
      setError("The API is unavailable; no local tax result is substituted.");
      return;
    }
    if (ruleSet === null || typeof ruleSet !== "object") {
      setError("The current rule document is not valid. Fix the rule workbench before simulating a document.");
      return;
    }
    const lineErrors = validateTaxDocumentLines(lines);
    if (lineErrors.length > 0) {
      setError(lineErrors.join(" "));
      return;
    }
    setIsSubmitting(true);
    try {
      const facts = buildTaxDocumentFacts(documentId, currency, jurisdiction, lines);
      const response = await fetchWithTimeout(`${apiBase}/v1/tax/calculate-document`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ facts, rule_set: ruleSet }),
      }, REQUEST_TIMEOUT_MS.default);
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw new Error(responseError(response, payload, `Document simulation failed (${response.status})`));
      setSimulation(parseTaxDocumentSimulation(payload));
      setNotice("The server returned a deterministic, unattested document trace. It cannot create a quote or authorize settlement.");
    } catch (cause) {
      setSimulation(null);
      setError(cause instanceof Error ? cause.message : "Document simulation failed");
    } finally {
      setIsSubmitting(false);
    }
  }

  return (
    <details className="authoring-section simulation-section" open>
      <summary>Line and invoice tax simulation</summary>
      <p className="evidence-note">This is an explicit document-facts workbench. The Rust service validates bounds, canonicalizes line identity, applies the rule&apos;s Line or Invoice scope, and returns an unattested trace. No browser arithmetic or settlement authority is used.</p>
      <form onSubmit={simulate} className="document-simulation-form">
        <div className="form-grid">
          <label><span>Document ID</span><input value={documentId} onChange={(event) => setDocumentId(event.target.value)} maxLength={64} required /></label>
          <label><span>Currency</span><input value={currency} onChange={(event) => setCurrency(event.target.value.toUpperCase())} maxLength={12} required /></label>
          <p className="field-help full-field">Jurisdiction is bound to the active server-owned context: <strong>{jurisdiction || "not selected"}</strong>.</p>
        </div>
        <div className="document-lines" role="group" aria-labelledby="document-lines-heading">
          <div className="document-lines-heading"><div><h3 id="document-lines-heading">Tax lines</h3><p className="field-help">Canonical results are sorted by line ID. Maximum 256 lines.</p></div><button type="button" className="secondary-button" onClick={addLine} disabled={lines.length >= 256}>Add line</button></div>
          {lines.map((line, index) => <fieldset className="document-line" key={`${line.id}-${index}`}>
            <legend>Line {index + 1}</legend>
            <label><span>Line ID</span><input value={line.id} onChange={(event) => updateLine(index, { id: event.target.value })} maxLength={64} required /></label>
            <label><span>Gross atomic units</span><input inputMode="numeric" value={line.grossAtomic} onChange={(event) => updateLine(index, { grossAtomic: event.target.value })} required /></label>
            <label><span>Net atomic units</span><input inputMode="numeric" value={line.netAtomic} onChange={(event) => updateLine(index, { netAtomic: event.target.value })} required /></label>
            <label><span>Exemption codes</span><input value={line.exemptionCodes} onChange={(event) => updateLine(index, { exemptionCodes: event.target.value })} placeholder="One per line or comma-separated" /></label>
            <button type="button" className="secondary-button" onClick={() => removeLine(index)} disabled={lines.length <= 1}>Remove</button>
          </fieldset>)}
        </div>
        <button type="submit" disabled={!serviceOnline || isSubmitting}>{isSubmitting ? "Calculating bounded document trace…" : "Simulate document tax"}</button>
      </form>
      {!serviceOnline && <p className="inline-warning" role="alert">The service is unavailable. This workbench does not display a locally invented total.</p>}
      {error && <p className="inline-error" role="alert">{error}</p>}
      {notice && <p className="inline-success" role="status">{notice}</p>}
      {simulation && <div className="document-result" role="region" aria-label="Document simulation result">
        <div className="total-card"><span>Unattested document tax</span><strong>{formatAtomicValue(simulation.result.total_tax_atomic, simulation.result.currency, locale)}</strong><small>{simulation.result.document_id} · {simulation.result.engine_version} · calculation {simulation.calculation_id}</small></div>
        <dl className="summary-list"><div><dt>Gross</dt><dd>{formatAtomicValue(simulation.result.gross_atomic, simulation.result.currency, locale)}</dd></div><div><dt>Net after tax</dt><dd>{formatAtomicValue(simulation.result.total_net_atomic, simulation.result.currency, locale)}</dd></div><div><dt>Facts digest</dt><dd><code>{simulation.result.facts_digest.slice(0, 8).map((byte) => byte.toString(16).padStart(2, "0")).join("")}…</code></dd></div>{simulation.result.fx_conversion && <div><dt>FX evidence</dt><dd>{simulation.result.fx_conversion.source_currency} → {simulation.result.fx_conversion.target_currency} · {simulation.result.fx_conversion.provider} · {simulation.result.fx_conversion.quote_id}</dd></div>}</dl>
        <div className="document-result-lines"><h3>Canonical line trace</h3>{simulation.result.lines.map((line) => <div className="document-result-line" key={line.id}><div><strong>{line.id}</strong><small>Gross {formatAtomicValue(line.gross_atomic, simulation.result.currency, locale)} · net {formatAtomicValue(line.total_net_atomic, simulation.result.currency, locale)}</small></div><strong>{formatAtomicValue(line.total_tax_atomic, simulation.result.currency, locale)} tax</strong></div>)}</div>
      </div>}
    </details>
  );
}
