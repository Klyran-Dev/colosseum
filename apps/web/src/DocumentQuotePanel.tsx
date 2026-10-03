import { FormEvent, useEffect, useState } from "react";
import { fetchWithTimeout, REQUEST_TIMEOUT_MS, responseError } from "./api";
import { createClientUuid } from "./clientIdentity";
import { DEFAULT_LOCALE, DEFAULT_TIME_ZONE, formatEpoch } from "./formatters";
import {
  buildTaxDocumentFacts,
  emptyDocumentLines,
  parseTaxDocumentQuote,
  TaxDocumentLineDraft,
  TaxDocumentQuote,
  validateTaxDocumentLines,
} from "./taxDocumentSimulation";

type Props = {
  apiBase: string;
  serviceOnline: boolean;
  accessToken: string;
  formatAtomic: (value: string, currency: string) => string;
  locale?: string;
  timeZone?: string;
  onOpenCheckout: (calculationId: string) => void;
};

function parseEvidence(value: string): unknown[] {
  if (new TextEncoder().encode(value).length > 256 * 1024) throw new Error("Jurisdiction evidence exceeds the 256 KiB browser safety bound.");
  let parsed: unknown;
  try {
    parsed = JSON.parse(value);
  } catch {
    throw new Error("Provider jurisdiction evidence must be valid JSON.");
  }
  if (!Array.isArray(parsed) || parsed.length === 0 || parsed.length > 64) throw new Error("Provider jurisdiction evidence must be a non-empty bounded JSON array.");
  return parsed;
}

function positiveVersion(value: string): number {
  const version = BigInt(value);
  if (version < 1n || version > BigInt(0x7fffffff)) throw new Error("Rule version must be a positive bounded integer.");
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
  const [documentId, setDocumentId] = useState("");
  const [currency, setCurrency] = useState("");
  const [jurisdiction, setJurisdiction] = useState("");
  const [ruleSetId, setRuleSetId] = useState("");
  const [ruleVersion, setRuleVersion] = useState("");
  const [regime, setRegime] = useState("");
  const [idempotencyKey, setIdempotencyKey] = useState(() => createClientUuid());
  const [jurisdictionEvidenceJson, setJurisdictionEvidenceJson] = useState("");
  const [lines, setLines] = useState<TaxDocumentLineDraft[]>(() => emptyDocumentLines());
  const [quote, setQuote] = useState<TaxDocumentQuote | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);

  useEffect(() => {
    setIdempotencyKey(createClientUuid());
  }, [documentId, currency, jurisdiction, ruleSetId, ruleVersion, regime, jurisdictionEvidenceJson, lines]);

  function updateLine(index: number, patch: Partial<TaxDocumentLineDraft>) {
    setLines((current) => current.map((line, lineIndex) => lineIndex === index ? { ...line, ...patch } : line));
    setQuote(null);
  }

  function addLine() {
    if (lines.length >= 256) return;
    setLines((current) => [...current, { id: "", grossAtomic: "", netAtomic: "", exemptionCodes: "" }]);
    setQuote(null);
  }

  function removeLine(index: number) {
    if (lines.length <= 1) return;
    setLines((current) => current.filter((_, lineIndex) => lineIndex !== index));
    setQuote(null);
  }

  async function createQuote(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);
    setNotice(null);
    setQuote(null);
    if (!serviceOnline) return setError("The API is unavailable; no browser-side tax result is substituted.");
    if (!accessToken.trim()) return setError("Sign in with an account allowed to create document quotes.");
    const lineErrors = validateTaxDocumentLines(lines);
    if (lineErrors.length > 0) return setError(lineErrors.join(" "));
    let facts: ReturnType<typeof buildTaxDocumentFacts>;
    let evidence: unknown[];
    let version: number;
    try {
      facts = buildTaxDocumentFacts(documentId, currency, jurisdiction, lines);
      evidence = parseEvidence(jurisdictionEvidenceJson);
      version = positiveVersion(ruleVersion);
      if (!ruleSetId.trim()) throw new Error("Published rule set ID is required.");
      if (!regime.trim()) throw new Error("Tax regime is required.");
      if (!idempotencyKey.trim()) throw new Error("An idempotency key is required for durable quote replay.");
    } catch (cause) {
      return setError(cause instanceof Error ? cause.message : "Document quote fields are invalid.");
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
      if (!response.ok) throw new Error(responseError(response, payload, `Document quote failed (${response.status}).`));
      const stored = parseTaxDocumentQuote(payload);
      setQuote(stored);
      setNotice(stored.status === "attested"
        ? "Persisted document evidence returned by the authenticated API with issuer attestation. It is not settled yet."
        : "Persisted document evidence returned by the authenticated API. It is not attested or settled yet.");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Document quote failed.");
    } finally {
      setIsSubmitting(false);
    }
  }

  return <section className="workspace-grid" aria-labelledby="document-quote-title">
    <article className="panel calculator-panel">
      <div className="panel-heading"><div><p className="eyebrow">Invoice tax</p><h2 id="document-quote-title">Calculate tax for an invoice</h2></div><span className="status-pill">Ready to review</span></div>
      <p className="panel-copy">Enter the invoice details and verified location evidence. The service applies the selected published rule, saves the calculation, and keeps the result available for review and payment.</p>
      <form onSubmit={createQuote} className="form-grid">
        <div className="route-evidence full-field" role="status" aria-label="Sign-in status"><strong>{accessToken.trim() ? "Signed-in session ready" : "Sign-in required"}</strong><small>{accessToken.trim() ? "Your session is held in memory and is used only for this request. The credential is never displayed in this workspace." : "Sign in before preparing an invoice calculation."}</small></div>
        <label><span>Document ID</span><input value={documentId} onChange={(event) => { setDocumentId(event.target.value); setQuote(null); }} maxLength={64} required /></label>
        <label><span>Currency</span><input value={currency} onChange={(event) => { setCurrency(event.target.value.toUpperCase()); setQuote(null); }} maxLength={12} required /></label>
        <label><span>Jurisdiction</span><input value={jurisdiction} onChange={(event) => { setJurisdiction(event.target.value.toUpperCase()); setQuote(null); }} maxLength={64} required /></label>
        <label><span>Tax regime</span><input value={regime} onChange={(event) => { setRegime(event.target.value); setQuote(null); }} maxLength={64} required /></label>
        <label><span>Published rule set ID</span><input value={ruleSetId} onChange={(event) => { setRuleSetId(event.target.value); setQuote(null); }} required /></label>
        <label><span>Rule version</span><input inputMode="numeric" value={ruleVersion} onChange={(event) => { setRuleVersion(event.target.value); setQuote(null); }} required /></label>
        <div className="route-evidence full-field"><strong>Retry protection selected automatically</strong><small>A secure retry reference is assigned automatically and stays the same while these invoice details remain unchanged.</small></div>
        <label className="full-field" htmlFor="jurisdiction-evidence"><span>Location evidence</span><textarea id="jurisdiction-evidence" value={jurisdictionEvidenceJson} onChange={(event) => { setJurisdictionEvidenceJson(event.target.value); setQuote(null); }} rows={7} autoComplete="off" spellCheck={false} required /><small className="field-help">The service checks the provider signature, account and facts, validity period, and approved location policy before using this evidence.</small></label>
        <div className="document-lines full-field" role="group" aria-labelledby="document-quote-lines-heading">
          <div className="document-lines-heading"><div><h3 id="document-quote-lines-heading">Invoice lines</h3><p className="field-help">Enter each line explicitly. The service validates line identity, amounts, exemptions, and conservation before saving the quote.</p></div><button type="button" className="secondary-button" onClick={addLine} disabled={lines.length >= 256}>Add line</button></div>
          {lines.map((line, index) => <fieldset className="document-line" key={`${line.id}-${index}`}>
            <legend>Line {index + 1}</legend>
            <label><span>Line ID</span><input value={line.id} onChange={(event) => updateLine(index, { id: event.target.value })} maxLength={64} placeholder="Enter a unique line ID" required /></label>
            <label><span>Amount before tax</span><input inputMode="numeric" value={line.grossAtomic} onChange={(event) => updateLine(index, { grossAtomic: event.target.value })} placeholder="Enter the line amount" required /></label>
            <label><span>Amount after tax</span><input inputMode="numeric" value={line.netAtomic} onChange={(event) => updateLine(index, { netAtomic: event.target.value })} placeholder="Enter the taxable amount" required /></label>
            <label><span>Exemption codes</span><input value={line.exemptionCodes} onChange={(event) => updateLine(index, { exemptionCodes: event.target.value })} placeholder="Optional" /></label>
            <button type="button" className="secondary-button" onClick={() => removeLine(index)} disabled={lines.length <= 1}>Remove line</button>
          </fieldset>)}
        </div>
        <button type="submit" disabled={!serviceOnline || isSubmitting || !accessToken.trim()}>{isSubmitting ? "Calculating invoice tax…" : "Calculate invoice tax"}</button>
      </form>
      {!serviceOnline && <p className="inline-warning" role="alert">The service is unavailable. No locally invented total is displayed.</p>}
      {error && <p className="inline-error" role="alert">{error}</p>}
      {notice && <p className="inline-success" role="status">{notice}</p>}
    </article>

    <article className="panel result-panel" aria-live="polite">
      <div className="panel-heading"><div><p className="eyebrow">Persisted calculation evidence</p><h2>Document result trace</h2></div>{quote && <span className={`status-pill ${quote.status === "attested" ? "status-pill-success" : "status-pill-warning"}`}>{quote.status === "attested" ? "Attested" : "Calculated"}</span>}</div>
      {!quote ? <div className="empty-state"><span className="empty-icon">∑</span><strong>No invoice calculation yet</strong><p>Enter the invoice details and location evidence to review the tax result.</p></div> : <div className="result-content">
        <div className="total-card"><span>Total tax withheld</span><strong>{formatAtomic(quote.result.total_tax_atomic, quote.result.currency)}</strong><small>Calculation ID {quote.calculation_id}</small></div>
        <dl className="summary-list"><div><dt>Document</dt><dd>{quote.result.document_id}</dd></div><div><dt>Gross</dt><dd>{formatAtomic(quote.result.gross_atomic, quote.result.currency)}</dd></div><div><dt>Net after tax</dt><dd>{formatAtomic(quote.result.total_net_atomic, quote.result.currency)}</dd></div><div><dt>Jurisdiction</dt><dd>{quote.result.jurisdiction}</dd></div><div><dt>Rule</dt><dd>{quote.result.rule_set_id} v{quote.result.rule_version}</dd></div><div><dt>Facts digest</dt><dd><code>{quote.result.facts_digest.slice(0, 8).map((byte) => byte.toString(16).padStart(2, "0")).join("")}…</code></dd></div>{quote.result.fx_conversion && <div><dt>FX evidence</dt><dd>{quote.result.fx_conversion.source_currency} → {quote.result.fx_conversion.target_currency} · {quote.result.fx_conversion.provider} · {quote.result.fx_conversion.quote_id}</dd></div>}<div><dt>Expires</dt><dd>{formatEpoch(quote.expires_at_unix, locale, timeZone)}</dd></div></dl>
        <div className="document-result-lines"><h3>Invoice lines</h3>{quote.result.lines.map((line) => <div className="document-result-line" key={line.id}><div><strong>{line.id}</strong><small>Before tax {formatAtomic(line.gross_atomic, quote.result.currency)} · after tax {formatAtomic(line.total_net_atomic, quote.result.currency)}</small></div><strong>{formatAtomic(line.total_tax_atomic, quote.result.currency)} tax</strong></div>)}</div>
        <button type="button" onClick={() => onOpenCheckout(quote.calculation_id)}>Open document checkout</button>
        {quote.status !== "attested" && <p className="inline-warning">The quote is calculated but not attested. Checkout will require a valid issuer attestation before compilation or signing.</p>}
      </div>}
    </article>
  </section>;
}
