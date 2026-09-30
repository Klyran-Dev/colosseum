import { FormEvent, useState } from "react";
import { fetchWithTimeout, REQUEST_TIMEOUT_MS, responseError } from "./api";
import { createClientUuid } from "./clientIdentity";
import { DEFAULT_LOCALE, DEFAULT_TIME_ZONE, formatEpoch } from "./formatters";
import {
  buildTaxDocumentFacts,
  defaultDocumentLines,
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
  const [documentId, setDocumentId] = useState("invoice-1");
  const [currency, setCurrency] = useState("USD");
  const [jurisdiction, setJurisdiction] = useState("US-CA");
  const [ruleSetId, setRuleSetId] = useState("");
  const [ruleVersion, setRuleVersion] = useState("1");
  const [regime, setRegime] = useState("sales-tax");
  const [idempotencyKey, setIdempotencyKey] = useState(() => createClientUuid());
  const [jurisdictionEvidenceJson, setJurisdictionEvidenceJson] = useState("");
  const [lines, setLines] = useState<TaxDocumentLineDraft[]>(() => defaultDocumentLines());
  const [quote, setQuote] = useState<TaxDocumentQuote | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);

  function updateLine(index: number, patch: Partial<TaxDocumentLineDraft>) {
    setLines((current) => current.map((line, lineIndex) => lineIndex === index ? { ...line, ...patch } : line));
    setQuote(null);
  }

  function addLine() {
    if (lines.length >= 256) return;
    setLines((current) => [...current, { id: `line-${current.length + 1}`, grossAtomic: "1000", netAtomic: "1000", exemptionCodes: "" }]);
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
    if (!accessToken.trim()) return setError("An access token with tax:quote scope is required for a persisted document quote.");
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
      <div className="panel-heading"><div><p className="eyebrow">Authenticated document quote</p><h2 id="document-quote-title">Create persisted document evidence</h2></div><span className="status-pill">Server-owned</span></div>
      <p className="panel-copy">The API authenticates the merchant, verifies provider-signed jurisdiction evidence, resolves the published rule, calculates the complete document trace, and persists the result with UUIDv7 identity and idempotency.</p>
      <form onSubmit={createQuote} className="form-grid">
        <label className="full-field"><span>OIDC access token (memory-only)</span><textarea value={accessToken} readOnly placeholder="Provide a token with tax:quote scope" autoComplete="off" spellCheck={false} rows={3} aria-describedby="document-quote-auth-help" /><small id="document-quote-auth-help" className="field-help">The token is held by the application shell; this workspace never stores it in the quote payload.</small></label>
        <label><span>Document ID</span><input value={documentId} onChange={(event) => { setDocumentId(event.target.value); setQuote(null); }} maxLength={64} required /></label>
        <label><span>Currency</span><input value={currency} onChange={(event) => { setCurrency(event.target.value.toUpperCase()); setQuote(null); }} maxLength={12} required /></label>
        <label><span>Jurisdiction</span><input value={jurisdiction} onChange={(event) => { setJurisdiction(event.target.value.toUpperCase()); setQuote(null); }} maxLength={64} required /></label>
        <label><span>Tax regime</span><input value={regime} onChange={(event) => { setRegime(event.target.value); setQuote(null); }} maxLength={64} required /></label>
        <label><span>Published rule set ID</span><input value={ruleSetId} onChange={(event) => { setRuleSetId(event.target.value); setQuote(null); }} required /></label>
        <label><span>Rule version</span><input inputMode="numeric" value={ruleVersion} onChange={(event) => { setRuleVersion(event.target.value); setQuote(null); }} required /></label>
        <label className="full-field"><span>Idempotency key</span><input value={idempotencyKey} onChange={(event) => setIdempotencyKey(event.target.value)} autoComplete="off" required /><small className="field-help">Retrying this exact key and request replays the same persisted calculation; conflicting input fails closed.</small></label>
        <label className="full-field" htmlFor="jurisdiction-evidence"><span>Provider-signed jurisdiction evidence (JSON array)</span><textarea id="jurisdiction-evidence" value={jurisdictionEvidenceJson} onChange={(event) => { setJurisdictionEvidenceJson(event.target.value); setQuote(null); }} rows={7} autoComplete="off" spellCheck={false} required /><small className="field-help">Evidence is untrusted in the browser. The API verifies signatures, merchant/facts binding, validity, and governed jurisdiction precedence.</small></label>
        <button type="submit" disabled={!serviceOnline || isSubmitting || !accessToken.trim()}>{isSubmitting ? "Creating persisted document quote…" : "Create document quote"}</button>
      </form>
      {!serviceOnline && <p className="inline-warning" role="alert">The service is unavailable. No locally invented total is displayed.</p>}
      {error && <p className="inline-error" role="alert">{error}</p>}
      {notice && <p className="inline-success" role="status">{notice}</p>}
    </article>

    <article className="panel result-panel" aria-live="polite">
      <div className="panel-heading"><div><p className="eyebrow">Persisted calculation evidence</p><h2>Document result trace</h2></div>{quote && <span className={`status-pill ${quote.status === "attested" ? "status-pill-success" : "status-pill-warning"}`}>{quote.status === "attested" ? "Attested" : "Calculated"}</span>}</div>
      {!quote ? <div className="empty-state"><span className="empty-icon">∑</span><strong>No persisted document quote yet</strong><p>Submit authenticated facts and provider evidence to inspect the server-owned calculation.</p></div> : <div className="result-content">
        <div className="total-card"><span>Total tax withheld</span><strong>{formatAtomic(quote.result.total_tax_atomic, quote.result.currency)}</strong><small>Calculation ID {quote.calculation_id}</small></div>
        <dl className="summary-list"><div><dt>Document</dt><dd>{quote.result.document_id}</dd></div><div><dt>Gross</dt><dd>{formatAtomic(quote.result.gross_atomic, quote.result.currency)}</dd></div><div><dt>Net after tax</dt><dd>{formatAtomic(quote.result.total_net_atomic, quote.result.currency)}</dd></div><div><dt>Jurisdiction</dt><dd>{quote.result.jurisdiction}</dd></div><div><dt>Rule</dt><dd>{quote.result.rule_set_id} v{quote.result.rule_version}</dd></div><div><dt>Facts digest</dt><dd><code>{quote.result.facts_digest.slice(0, 8).map((byte) => byte.toString(16).padStart(2, "0")).join("")}…</code></dd></div><div><dt>Expires</dt><dd>{formatEpoch(quote.expires_at_unix, locale, timeZone)}</dd></div></dl>
        <div className="document-result-lines"><h3>Canonical line trace</h3>{quote.result.lines.map((line) => <div className="document-result-line" key={line.id}><div><strong>{line.id}</strong><small>Gross {formatAtomic(line.gross_atomic, quote.result.currency)} · net {formatAtomic(line.total_net_atomic, quote.result.currency)}</small></div><strong>{formatAtomic(line.total_tax_atomic, quote.result.currency)} tax</strong></div>)}</div>
        <button type="button" onClick={() => onOpenCheckout(quote.calculation_id)}>Open document checkout</button>
        {quote.status !== "attested" && <p className="inline-warning">The quote is calculated but not attested. Checkout will require a valid issuer attestation before compilation or signing.</p>}
      </div>}
    </article>
  </section>;
}
