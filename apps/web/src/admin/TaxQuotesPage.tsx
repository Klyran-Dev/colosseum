import { FormEvent, useMemo, useState } from "react";
import { useMutation, useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { createClientUuidV7 } from "../clientIdentity";
import { parseSettlementRoutes, type SettlementRoute } from "../checkout";
import { requestJson } from "./data";
import { AdminCapability, AdminSession } from "./model";

interface TaxQuotesPageProps {
  session: AdminSession | null;
  capability?: AdminCapability;
}

interface QuoteValues {
  grossAtomic: string;
  netAtomic: string;
  currency: string;
  atomicDecimals: string;
  ruleSetId: string;
  ruleVersion: string;
  jurisdiction: string;
  regime: string;
  jurisdictionEvidence: string;
  applicabilityContextId: string;
}

interface QuoteComponent {
  id: string;
  label: string;
  baseAtomic: string;
  effectiveBaseAtomic: string;
  taxAtomic: string;
  explanation: string;
  exemptionApplied: boolean;
}

interface QuoteResult {
  calculationId: string;
  status: string;
  expiresAtUnix: number;
  result: {
    ruleSetId: string;
    ruleVersion: number;
    engineVersion: string;
    atomicDecimals: number;
    currency: string;
    jurisdiction: string;
    factsDigest: number[];
    ruleDigest: number[];
    grossAtomic: string;
    totalTaxAtomic: string;
    totalNetAtomic: string;
    components: QuoteComponent[];
    treatment: string;
    arithmeticTaxAtomic: string;
    collectedTaxAtomic: string;
    selfAssessedTaxAtomic: string;
    withheldTaxAtomic: string;
    taxableBaseAtomic: string;
  };
}

const initialValues: QuoteValues = {
  grossAtomic: "",
  netAtomic: "",
  currency: "USD",
  atomicDecimals: "2",
  ruleSetId: "",
  ruleVersion: "",
  jurisdiction: "",
  regime: "",
  jurisdictionEvidence: "[]",
  applicabilityContextId: "",
};

function objectValue(value: unknown): Record<string, unknown> {
  return value && typeof value === "object" && !Array.isArray(value) ? value as Record<string, unknown> : {};
}

function stringValue(value: unknown, field: string): string {
  if (typeof value !== "string" || !value) throw new Error(`The quote response omitted ${field}.`);
  return value;
}

function integerValue(value: unknown, field: string, minimum = 0): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < minimum) throw new Error(`The quote response returned an invalid ${field}.`);
  return value;
}

function digest(value: unknown, field: string): number[] {
  if (!Array.isArray(value) || value.length !== 32 || value.some((byte) => typeof byte !== "number" || !Number.isInteger(byte) || byte < 0 || byte > 255)) throw new Error(`The quote response returned an invalid ${field}.`);
  return [...value] as number[];
}

function parseQuoteResult(payload: unknown): QuoteResult {
  const source = objectValue(payload);
  const result = objectValue(source.result);
  const components = result.components;
  const treatmentOutcome = objectValue(result.treatment_outcome);
  if (!Array.isArray(components) || components.length > 64) throw new Error("The quote response returned an invalid component trace.");
  return {
    calculationId: stringValue(source.calculation_id, "calculation ID"),
    status: stringValue(source.status, "status"),
    expiresAtUnix: integerValue(source.expires_at_unix, "quote expiry", 1),
    result: {
      ruleSetId: stringValue(result.rule_set_id, "rule-set ID"),
      ruleVersion: integerValue(result.rule_version, "rule version", 1),
      engineVersion: stringValue(result.engine_version, "engine version"),
      atomicDecimals: integerValue(result.atomic_decimals, "atomic decimals"),
      currency: stringValue(result.currency, "currency"),
      jurisdiction: stringValue(result.jurisdiction, "jurisdiction"),
      factsDigest: digest(result.facts_digest, "facts digest"),
      ruleDigest: digest(result.rule_digest, "rule digest"),
      grossAtomic: stringValue(result.gross_atomic, "gross amount"),
      totalTaxAtomic: stringValue(result.total_tax_atomic, "total tax"),
      totalNetAtomic: stringValue(result.total_net_atomic, "total net"),
      components: components.map((raw) => {
        const component = objectValue(raw);
        return {
          id: stringValue(component.id, "component ID"),
          label: stringValue(component.label, "component label"),
          baseAtomic: stringValue(component.base_atomic, "component base"),
          effectiveBaseAtomic: stringValue(component.effective_base_atomic, "effective component base"),
          taxAtomic: stringValue(component.tax_atomic, "component tax"),
          explanation: stringValue(component.explanation, "component explanation"),
          exemptionApplied: component.exemption_applied === true,
        };
      }),
      treatment: stringValue(treatmentOutcome.treatment, "treatment outcome"),
      arithmeticTaxAtomic: stringValue(treatmentOutcome.arithmetic_tax_atomic, "arithmetic tax"),
      collectedTaxAtomic: stringValue(treatmentOutcome.collected_tax_atomic, "collected tax"),
      selfAssessedTaxAtomic: stringValue(treatmentOutcome.self_assessed_tax_atomic, "self-assessed tax"),
      withheldTaxAtomic: stringValue(treatmentOutcome.withheld_tax_atomic, "withheld tax"),
      taxableBaseAtomic: stringValue(treatmentOutcome.taxable_base_atomic, "taxable base"),
    },
  };
}

function formatAtomic(value: string, decimals: number, currency: string): string {
  if (!/^-?\d+$/u.test(value)) return "Unavailable";
  const negative = value.startsWith("-");
  const digits = negative ? value.slice(1) : value;
  const padded = digits.padStart(decimals + 1, "0");
  const whole = decimals ? padded.slice(0, -decimals) : padded;
  const fraction = decimals ? padded.slice(-decimals) : "";
  const normalized = `${negative ? "-" : ""}${whole}${fraction ? `.${fraction}` : ""}`;
  return `${normalized} ${currency}`;
}

function parseInteger(value: string, label: string, minimum: number): number {
  if (!/^\d+$/u.test(value)) throw new Error(`${label} must be a non-negative integer.`);
  const parsed = Number(value);
  if (!Number.isSafeInteger(parsed) || parsed < minimum) throw new Error(`${label} is outside the supported range.`);
  return parsed;
}

function parseJson(value: string, label: string): unknown {
  try { return JSON.parse(value); } catch { throw new Error(`${label} must contain valid JSON.`); }
}

function Field({ label, value, onChange, help, type = "text", disabled = false, required = true }: { label: string; value: string; onChange: (value: string) => void; help?: string; type?: string; disabled?: boolean; required?: boolean }) {
  const id = `quote-${label.toLowerCase().replace(/[^a-z0-9]+/gu, "-")}`;
  return <label className="block" htmlFor={id}><span className="mb-2 block text-sm font-bold text-admin-ink">{label}</span><input id={id} className="w-full rounded-xl border border-admin-line bg-admin-bg px-4 py-3 text-sm text-admin-ink outline-none transition focus:border-admin-primary focus:ring-4 focus:ring-admin-primary/10 disabled:cursor-not-allowed disabled:opacity-60" type={type} value={value} onChange={(event) => onChange(event.target.value)} disabled={disabled} required={required} />{help && <small className="mt-1.5 block text-xs leading-5 text-admin-muted">{help}</small>}</label>;
}

function QuoteResultCard({ quote }: { quote: QuoteResult }) {
  const result = quote.result;
  return <section className="space-y-5" aria-label="Verified quote result"><article className="rounded-2xl border border-admin-green/30 bg-admin-green-soft p-6"><div className="flex flex-wrap items-start justify-between gap-4"><div><p className="text-xs font-black uppercase tracking-[0.14em] text-admin-green">Server response</p><h2 className="mt-1 text-2xl font-black text-admin-ink">Verified quote result</h2><p className="mt-2 text-sm text-admin-muted">{result.jurisdiction} · {result.currency} · rule {result.ruleSetId} v{result.ruleVersion}</p></div><span className="rounded-full bg-admin-surface px-3 py-1 text-xs font-black text-admin-green">{quote.status}</span></div><div className="mt-6 grid gap-3 sm:grid-cols-3"><div className="rounded-xl bg-admin-surface p-4"><span className="block text-xs font-bold uppercase tracking-[0.1em] text-admin-muted">Gross</span><strong className="mt-2 block text-xl font-black text-admin-ink">{formatAtomic(result.grossAtomic, result.atomicDecimals, result.currency)}</strong></div><div className="rounded-xl bg-admin-surface p-4"><span className="block text-xs font-bold uppercase tracking-[0.1em] text-admin-muted">Tax removed</span><strong className="mt-2 block text-xl font-black text-admin-ink">{formatAtomic(result.totalTaxAtomic, result.atomicDecimals, result.currency)}</strong></div><div className="rounded-xl bg-admin-surface p-4"><span className="block text-xs font-bold uppercase tracking-[0.1em] text-admin-muted">Net</span><strong className="mt-2 block text-xl font-black text-admin-ink">{formatAtomic(result.totalNetAtomic, result.atomicDecimals, result.currency)}</strong></div></div><dl className="mt-6 grid gap-3 text-sm sm:grid-cols-2"><div><dt className="text-admin-muted">Calculation reference</dt><dd className="mt-1 break-all font-mono text-xs text-admin-ink">{quote.calculationId}</dd></div><div><dt className="text-admin-muted">Attestation</dt><dd className="mt-1 font-bold text-admin-amber-ink">Awaiting issuer attestation</dd></div><div><dt className="text-admin-muted">Engine</dt><dd className="mt-1 text-admin-ink">{result.engineVersion}</dd></div><div><dt className="text-admin-muted">Expires</dt><dd className="mt-1 text-admin-ink">{new Date(quote.expiresAtUnix * 1000).toLocaleString()}</dd></div></dl></article><article className="rounded-2xl border border-admin-line bg-admin-surface p-6 shadow-soft"><details open><summary className="cursor-pointer list-none text-lg font-black text-admin-ink">Why this tax?</summary><p className="mt-2 max-w-3xl text-sm leading-6 text-admin-muted">The explanation below is the server-returned calculation trace. The browser does not recompute or reinterpret the tax result.</p><div className="mt-5 overflow-x-auto"><table className="w-full min-w-[42rem] text-left text-sm"><caption className="sr-only">Tax component trace</caption><thead className="border-b border-admin-line text-xs uppercase tracking-[0.1em] text-admin-muted"><tr><th className="px-3 py-3 font-black">Component</th><th className="px-3 py-3 font-black">Tax base</th><th className="px-3 py-3 font-black">Tax</th><th className="px-3 py-3 font-black">Explanation</th></tr></thead><tbody className="divide-y divide-admin-line">{result.components.map((component) => <tr key={component.id}><td className="px-3 py-4 font-semibold text-admin-ink">{component.label}{component.exemptionApplied && <span className="ml-2 rounded-full bg-admin-amber-soft px-2 py-1 text-xs font-bold text-admin-amber-ink">Exemption applied</span>}</td><td className="px-3 py-4 text-admin-muted">{formatAtomic(component.effectiveBaseAtomic, result.atomicDecimals, result.currency)}</td><td className="px-3 py-4 font-semibold text-admin-ink">{formatAtomic(component.taxAtomic, result.atomicDecimals, result.currency)}</td><td className="max-w-md px-3 py-4 leading-6 text-admin-muted">{component.explanation}</td></tr>)}</tbody></table></div><dl className="mt-5 grid gap-3 border-t border-admin-line pt-5 text-sm sm:grid-cols-2 lg:grid-cols-5"><div><dt className="text-admin-muted">Treatment</dt><dd className="mt-1 font-bold text-admin-ink">{result.treatment}</dd></div><div><dt className="text-admin-muted">Taxable base</dt><dd className="mt-1 text-admin-ink">{formatAtomic(result.taxableBaseAtomic, result.atomicDecimals, result.currency)}</dd></div><div><dt className="text-admin-muted">Collected</dt><dd className="mt-1 text-admin-ink">{formatAtomic(result.collectedTaxAtomic, result.atomicDecimals, result.currency)}</dd></div><div><dt className="text-admin-muted">Self-assessed</dt><dd className="mt-1 text-admin-ink">{formatAtomic(result.selfAssessedTaxAtomic, result.atomicDecimals, result.currency)}</dd></div><div><dt className="text-admin-muted">Withheld</dt><dd className="mt-1 text-admin-ink">{formatAtomic(result.withheldTaxAtomic, result.atomicDecimals, result.currency)}</dd></div></dl></details></article><article className="rounded-2xl border border-admin-line bg-admin-surface-muted p-5 text-sm"><p className="font-bold text-admin-ink">Evidence commitments</p><div className="mt-3 grid gap-3 sm:grid-cols-2"><code className="break-all text-xs text-admin-muted">facts {result.factsDigest.join(",")}</code><code className="break-all text-xs text-admin-muted">rule {result.ruleDigest.join(",")}</code></div><Link to={`/admin/evidence?calculation_id=${encodeURIComponent(quote.calculationId)}`} className="mt-4 inline-flex text-sm font-bold text-admin-primary">Open evidence explorer →</Link></article></section>;
}

function CheckoutLinkPanel({ quote, session, capability }: { quote: QuoteResult; session: AdminSession | null; capability?: AdminCapability }) {
  const editable = Boolean(session && capability?.decision === "allow");
  const routesQuery = useQuery({ queryKey: ["admin", "checkout-routes", session?.email ?? "anonymous"], enabled: editable && quote.status === "attested", retry: false, queryFn: async () => parseSettlementRoutes(await requestJson("/api/v1/settlement/routes", session)) });
  const [routeId, setRouteId] = useState("");
  const [link, setLink] = useState<string | null>(null);
  const mutation = useMutation({ mutationFn: async () => {
    if (!routeId) throw new Error("Select an active settlement route before sharing checkout.");
    return requestJson("/api/v1/checkout/sessions", session, { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify({ calculation_id: quote.calculationId, route_id: routeId }) });
  }, onSuccess: (value) => {
    if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error("The checkout service returned an invalid link response.");
    const record = value as Record<string, unknown>;
    if (typeof record.session_id !== "string" || typeof record.public_token !== "string" || typeof record.expires_at_unix !== "number") throw new Error("The checkout service returned an incomplete link response.");
    setLink(`${window.location.origin}/checkout/${encodeURIComponent(record.session_id)}#token=${encodeURIComponent(record.public_token)}`);
  } });
  const routes = routesQuery.data ?? [];
  const selectedRoute: SettlementRoute | undefined = routes.find((route) => route.id === routeId);
  return <article className="rounded-2xl border border-admin-line bg-admin-surface p-6 shadow-soft"><div className="flex flex-wrap items-start justify-between gap-4"><div><p className="text-xs font-black uppercase tracking-[0.12em] text-admin-primary">Customer handoff</p><h2 className="mt-1 text-xl font-black text-admin-ink">Share a secure checkout link</h2><p className="mt-2 max-w-2xl text-sm leading-6 text-admin-muted">Create a short-lived guest session for this exact quote and an active verified route. The customer signs with their own wallet; this link does not expose your admin session.</p></div><span className={`rounded-full px-3 py-1 text-xs font-bold ${editable ? "bg-admin-green-soft text-admin-green" : "bg-admin-amber-soft text-admin-amber-ink"}`}>{editable ? "Merchant permission confirmed" : "Attestation and permission required"}</span></div><div className="mt-5 grid gap-4 sm:grid-cols-[minmax(0,1fr)_auto]"><label className="block"><span className="mb-2 block text-sm font-bold text-admin-ink">Verified settlement route</span><select className="w-full rounded-xl border border-admin-line bg-admin-bg px-4 py-3 text-sm text-admin-ink disabled:opacity-60" value={routeId} onChange={(event) => { setRouteId(event.target.value); setLink(null); }} disabled={!editable || routesQuery.isPending || mutation.isPending}><option value="">{routesQuery.isPending ? "Loading active routes…" : "Select a route"}</option>{routes.map((route) => <option key={route.id} value={route.id}>{route.route_key} v{route.version} · {route.mint}</option>)}</select><small className="mt-1.5 block text-xs leading-5 text-admin-muted">Only routes returned by the authenticated tenant route registry can be shared.</small></label><button type="button" className="self-end rounded-xl bg-admin-primary px-5 py-3 text-sm font-bold text-white disabled:cursor-not-allowed disabled:opacity-50" onClick={() => mutation.mutate()} disabled={!editable || !selectedRoute || mutation.isPending || routesQuery.isPending}>{mutation.isPending ? "Creating…" : "Create checkout link"}</button></div>{(mutation.error || routesQuery.error) && <p className="mt-4 rounded-xl border border-admin-red/30 bg-admin-red-soft p-4 text-sm text-admin-red" role="alert">{(mutation.error instanceof Error ? mutation.error.message : routesQuery.error instanceof Error ? routesQuery.error.message : "The active route list is unavailable.")}</p>}{link && <div className="mt-5 rounded-xl border border-admin-green/30 bg-admin-green-soft p-4" role="status"><strong className="block text-sm text-admin-ink">Checkout link ready</strong><p className="mt-1 text-xs leading-5 text-admin-muted">The raw token is shown here so you can paste it into a demo browser. It is not shown again by the API.</p><input readOnly value={link} className="mt-3 w-full rounded-lg border border-admin-line bg-admin-surface px-3 py-2 font-mono text-xs text-admin-ink" aria-label="Customer checkout link" /><a className="mt-3 inline-flex text-sm font-bold text-admin-primary" href={link} target="_blank" rel="noreferrer">Open customer checkout →</a></div>}</article>;
}

export default function TaxQuotesPage({ session, capability }: TaxQuotesPageProps) {
  const [values, setValues] = useState<QuoteValues>(initialValues);
  const [idempotencyKey, setIdempotencyKey] = useState(() => createClientUuidV7());
  const [validationError, setValidationError] = useState<string | null>(null);
  const mutation = useMutation({ mutationFn: (body: unknown) => requestJson("/api/v1/tax/quote", session, { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify(body) }), onSuccess: () => setValidationError(null) });
  const editable = Boolean(session && capability?.decision === "allow");
  const result = useMemo(() => mutation.data ? parseQuoteResult(mutation.data) : null, [mutation.data]);

  function update(key: keyof QuoteValues, value: string) { setValues((current) => ({ ...current, [key]: value })); setValidationError(null); mutation.reset(); }
  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!editable) return;
    try {
      const atomicDecimals = parseInteger(values.atomicDecimals, "Atomic decimals", 0);
      if (atomicDecimals > 9) throw new Error("Atomic decimals must be between 0 and 9.");
      parseInteger(values.ruleVersion, "Rule version", 1);
      if (!/^\d+$/u.test(values.grossAtomic) || !/^\d+$/u.test(values.netAtomic)) throw new Error("Gross and net amounts must be canonical non-negative atomic-unit strings.");
      const evidence = parseJson(values.jurisdictionEvidence, "Jurisdiction evidence");
      mutation.mutate({ idempotency_key: idempotencyKey, rule_set_id: values.ruleSetId.trim(), rule_version: Number(values.ruleVersion), jurisdiction: values.jurisdiction.trim(), regime: values.regime.trim(), facts: { gross_atomic: values.grossAtomic.trim(), net_atomic: values.netAtomic.trim(), atomic_decimals: atomicDecimals, currency: values.currency.trim().toUpperCase(), jurisdiction: values.jurisdiction.trim() }, ...(values.applicabilityContextId.trim() ? { applicability_context_id: values.applicabilityContextId.trim() } : {}), jurisdiction_evidence: evidence });
    } catch (cause) { setValidationError(cause instanceof Error ? cause.message : "The quote request is invalid."); }
  }

  return <><header className="mb-7 flex flex-col justify-between gap-5 sm:flex-row sm:items-end"><div><p className="mb-2 text-xs font-black uppercase tracking-[0.16em] text-admin-primary">Admin / tax quotes</p><h1 className="text-3xl font-black tracking-[-0.04em] text-admin-ink sm:text-4xl">Prepare a verified quote</h1><p className="mt-2 max-w-2xl text-sm leading-6 text-admin-muted">Create a durable, server-calculated quote with exact atomic amounts, jurisdiction evidence, and a replayable tax trace.</p></div><span className="rounded-xl border border-admin-line bg-admin-surface px-4 py-2.5 text-sm font-bold text-admin-muted shadow-soft">No wallet signing in this workspace</span></header><div className="grid gap-6 xl:grid-cols-[minmax(0,1fr)_20rem]"><article className="rounded-2xl border border-admin-line bg-admin-surface p-6 shadow-soft"><div className="mb-6 flex flex-wrap items-start justify-between gap-4"><div><p className="text-xs font-black uppercase tracking-[0.12em] text-admin-primary">Quote preparation</p><h2 className="mt-1 text-xl font-black text-admin-ink">Exact financial inputs</h2><p className="mt-2 max-w-2xl text-sm leading-6 text-admin-muted">The service owns tax arithmetic, jurisdiction resolution, and rule validity. This form only submits bounded facts and references.</p></div><span className={`rounded-full px-3 py-1 text-xs font-bold ${editable ? "bg-admin-green-soft text-admin-green" : "bg-admin-amber-soft text-admin-amber-ink"}`}>{editable ? "Server permission confirmed" : "Fresh server permission is required"}</span></div><form onSubmit={submit} className="grid gap-5 sm:grid-cols-2"><Field label="Gross amount · atomic units" value={values.grossAtomic} onChange={(value) => update("grossAtomic", value)} help="Canonical decimal string; no locale separators." type="text" disabled={!editable} /><Field label="Net amount · atomic units" value={values.netAtomic} onChange={(value) => update("netAtomic", value)} help="Expected pre-tax amount as atomic units." type="text" disabled={!editable} /><Field label="Asset currency" value={values.currency} onChange={(value) => update("currency", value)} help="Currency or asset code resolved by the service." disabled={!editable} /><Field label="Atomic decimals" value={values.atomicDecimals} onChange={(value) => update("atomicDecimals", value)} help="Declared asset precision from 0 through 9." type="number" disabled={!editable} /><Field label="Published rule-set ID" value={values.ruleSetId} onChange={(value) => update("ruleSetId", value)} help="Immutable server-published rule reference." disabled={!editable} /><Field label="Rule version" value={values.ruleVersion} onChange={(value) => update("ruleVersion", value)} help="Exact version selected for this quote." type="number" disabled={!editable} /><Field label="Jurisdiction" value={values.jurisdiction} onChange={(value) => update("jurisdiction", value)} help="Canonical jurisdiction code." disabled={!editable} /><Field label="Tax regime" value={values.regime} onChange={(value) => update("regime", value)} help="Published rule-release selector; not a legal conclusion." disabled={!editable} /><Field label="Applicability context ID" value={values.applicabilityContextId} onChange={(value) => update("applicabilityContextId", value)} help="Optional server-issued published context UUIDv7." disabled={!editable} required={false} /><label className="block sm:col-span-2" htmlFor="quote-jurisdiction-evidence"><span className="mb-2 block text-sm font-bold text-admin-ink">Provider-signed jurisdiction evidence JSON</span><textarea id="quote-jurisdiction-evidence" className="min-h-32 w-full rounded-xl border border-admin-line bg-admin-bg px-4 py-3 font-mono text-sm text-admin-ink outline-none transition focus:border-admin-primary focus:ring-4 focus:ring-admin-primary/10 disabled:cursor-not-allowed disabled:opacity-60" value={values.jurisdictionEvidence} onChange={(event) => update("jurisdictionEvidence", event.target.value)} disabled={!editable} required /><small className="mt-1.5 block text-xs leading-5 text-admin-muted">The service validates signatures and evidence scope. An empty array remains a server rejection for jurisdictions requiring evidence.</small></label><div className="rounded-xl border border-admin-line bg-admin-bg p-4 sm:col-span-2"><div className="flex flex-wrap items-center justify-between gap-3"><div><span className="block text-xs font-black uppercase tracking-[0.12em] text-admin-muted">Idempotency reference</span><code className="mt-1 block break-all text-xs text-admin-ink">{idempotencyKey}</code></div><button type="button" className="rounded-lg border border-admin-line px-3 py-2 text-xs font-bold text-admin-ink transition hover:bg-admin-surface-muted" onClick={() => { setIdempotencyKey(createClientUuidV7()); mutation.reset(); setValidationError(null); }}>New reference</button></div><p className="mt-2 text-xs leading-5 text-admin-muted">Keep the same reference to safely retry this exact request. Generate a new reference before changing any input.</p></div>{(validationError || mutation.error) && <div className="rounded-xl border border-admin-red/30 bg-admin-red-soft p-4 text-sm leading-5 text-admin-red" role="alert">{validationError ?? (mutation.error instanceof Error ? mutation.error.message : "The quote service rejected the request.")}</div>}{mutation.isPending && <p className="rounded-xl border border-admin-line bg-admin-surface-muted p-4 text-sm text-admin-muted" role="status">Waiting for the tax service to persist the quote…</p>}<div className="flex flex-wrap items-center justify-between gap-3 border-t border-admin-line pt-5 sm:col-span-2"><p className="max-w-xl text-xs leading-5 text-admin-muted">This request is tenant-bound by the authenticated principal. It does not claim legal compliance, filing, or tax advice.</p><button type="submit" disabled={!editable || mutation.isPending} className="rounded-xl bg-admin-primary px-5 py-3 text-sm font-bold text-white shadow-soft transition hover:bg-admin-primary-strong disabled:cursor-not-allowed disabled:opacity-50">Request verified quote</button></div></form></article><aside className="space-y-6"><article className="rounded-2xl border border-admin-line bg-admin-surface p-6 shadow-soft"><p className="text-xs font-black uppercase tracking-[0.12em] text-admin-muted">Guardrails</p><h2 className="mt-1 text-xl font-black text-admin-ink">What the browser cannot do</h2><ul className="mt-5 space-y-4 text-sm leading-6 text-admin-muted"><li><strong className="text-admin-ink">No tax math:</strong> totals and component traces come from the Rust engine.</li><li><strong className="text-admin-ink">No role grant:</strong> RBAC, ABAC, and PBAC remain server-owned.</li><li><strong className="text-admin-ink">No wallet authority:</strong> quote creation never signs or submits Solana transactions.</li></ul></article><article className="rounded-2xl border border-admin-line bg-admin-surface p-6 shadow-soft"><p className="text-xs font-black uppercase tracking-[0.12em] text-admin-muted">Next boundary</p><h2 className="mt-1 text-xl font-black text-admin-ink">Settlement handoff</h2><p className="mt-2 text-sm leading-6 text-admin-muted">After attestation, a separate wallet-bound compiler prepares a transaction. The admin UI does not treat a quote as payment success.</p>{result && <p className="mt-4 text-xs font-bold text-admin-amber-ink">Attestation is still required before settlement.</p>}</article></aside></div>{result && <div className="mt-6 space-y-6"><QuoteResultCard quote={result} /><CheckoutLinkPanel quote={result} session={session} capability={capability} /></div>}</>;
}

export { formatAtomic, parseQuoteResult };
