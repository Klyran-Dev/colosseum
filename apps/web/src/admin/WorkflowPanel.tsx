import { FormEvent, useMemo, useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { requestJson } from "./data";
import { AdminCapability, AdminSection, AdminSession } from "./model";
import { createClientUuidV7 } from "../clientIdentity";
import { BROWSER_AUTH_CONFIG, BROWSER_AUTH_SERVICE_BASE_URL } from "../authConfig";

type WorkflowSection = Exclude<AdminSection, "overview" | "settings">;

interface WorkflowPanelProps {
  section: WorkflowSection;
  session: AdminSession | null;
  capability?: AdminCapability;
}

type WorkflowValues = Record<string, string>;

const initialValues: WorkflowValues = {
  gross_atomic: "",
  net_atomic: "",
  currency: "USDC",
  atomic_decimals: "6",
  rule_set_id: "",
  rule_version: "1",
  effective_from_unix: "",
  effective_until_unix: "",
  jurisdiction: "",
  regime: "",
  jurisdiction_evidence: "[]",
  rule_external_id: "",
  rule_document: "{}",
  review_evidence: "{}",
  intent_id: "",
  claim_id: "",
  route_id: "",
  destination: "",
  nonce: "",
  amount_atomic: "",
  reference_digest: "[]",
  execute_after_unix: "",
  expires_at_unix: "",
  invitation_email: "",
  server_role_id: "",
  calculation_id: "",
};

const sectionConfig: Record<WorkflowSection, { eyebrow: string; title: string; description: string; submit: string; endpoint: string; fields: readonly { key: keyof WorkflowValues; label: string; help: string; type?: "text" | "number" | "email" | "textarea" }[] }> = {
  quotes: {
    eyebrow: "Quote preparation",
    title: "Prepare a verified tax quote",
    description: "Submit exact atomic amounts and provider-signed jurisdiction evidence to the tax service.",
    submit: "Request verified quote",
    endpoint: "/api/v1/tax/quote",
    fields: [
      { key: "gross_atomic", label: "Gross amount · atomic units", help: "Integer atomic units only.", type: "number" },
      { key: "net_atomic", label: "Net amount · atomic units", help: "Integer atomic units only.", type: "number" },
      { key: "currency", label: "Asset mint or currency code", help: "Resolved and validated by the service." },
      { key: "atomic_decimals", label: "Atomic decimals", help: "The asset precision declared by the route.", type: "number" },
      { key: "rule_set_id", label: "Published rule-set ID", help: "A server-published immutable rule set." },
      { key: "rule_version", label: "Rule version", help: "The exact published version.", type: "number" },
      { key: "jurisdiction", label: "Jurisdiction", help: "Canonical jurisdiction code." },
      { key: "regime", label: "Tax regime", help: "Server-validates the regime against the rule set." },
      { key: "jurisdiction_evidence", label: "Jurisdiction evidence JSON", help: "At least one provider-signed evidence envelope is required.", type: "textarea" },
    ],
  },
  rules: {
    eyebrow: "Rule governance",
    title: "Create a rule version for review",
    description: "Rule documents remain immutable after publication and are evaluated by the server-owned tax engine.",
    submit: "Create rule draft",
    endpoint: "/api/v1/tax/rules",
    fields: [
      { key: "rule_external_id", label: "Rule-set external ID", help: "Stable identifier for the governed rule family." },
      { key: "rule_version", label: "Version", help: "A positive integer version.", type: "number" },
      { key: "effective_from_unix", label: "Effective from · Unix seconds", help: "The bounded effective window for this immutable version.", type: "number" },
      { key: "effective_until_unix", label: "Effective until · Unix seconds", help: "Leave blank for an open-ended effective window.", type: "number" },
      { key: "jurisdiction", label: "Jurisdiction", help: "Scope of the rule version." },
      { key: "regime", label: "Tax regime", help: "Legal/tax regime owner validates this value." },
      { key: "rule_document", label: "Canonical rule document JSON", help: "Cycles, ambiguity, and unsupported constructs are rejected by the service.", type: "textarea" },
      { key: "review_evidence", label: "Review provenance JSON", help: "Source digest, tax owner, and review reference are required.", type: "textarea" },
    ],
  },
  settlements: {
    eyebrow: "Settlement review",
    title: "Reconcile a settlement intent",
    description: "Review finalized chain evidence before any correction or downstream action is considered.",
    submit: "Request reconciliation",
    endpoint: "/api/v1/settlement/intents",
    fields: [
      { key: "intent_id", label: "Settlement intent ID", help: "The server-issued intent identifier." },
      { key: "review_evidence", label: "Review evidence JSON", help: "Provider and finalized-chain evidence are validated server-side.", type: "textarea" },
    ],
  },
  treasury: {
    eyebrow: "Treasury controls",
    title: "Prepare a bounded release claim",
    description: "Select a server-verified route and submit an immutable, idempotent release claim for independent approval.",
    submit: "Create release claim",
    endpoint: "/api/v1/vault/release-claims",
    fields: [
      { key: "route_id", label: "Verified route ID", help: "The server-owned finalized settlement route; account addresses are never client-selectable." },
      { key: "destination", label: "Destination token account", help: "Base58 destination token account validated by the service." },
      { key: "nonce", label: "Nonce · canonical decimal string", help: "Positive u64 serialized as a decimal string." },
      { key: "amount_atomic", label: "Amount · atomic units", help: "Positive u64 serialized as a decimal string." },
      { key: "reference_digest", label: "Reference digest JSON", help: "Exactly 32 byte values; the service binds this digest to the claim.", type: "textarea" },
      { key: "execute_after_unix", label: "Execute after · Unix seconds", help: "Bounded execution window start.", type: "number" },
      { key: "expires_at_unix", label: "Expires at · Unix seconds", help: "Must be after execute_after and within the bounded window.", type: "number" },
    ],
  },
  access: {
    eyebrow: "Access administration",
    title: "Invite a governed operator",
    description: "Invitation and responsibility assignment are validated by the auth service; the browser never grants a role.",
    submit: "Request invitation",
    endpoint: "/api/v1/admin/invitations",
    fields: [
      { key: "invitation_email", label: "Operator work email", help: "The auth service verifies ownership and invitation state.", type: "email" },
      { key: "server_role_id", label: "Server role identifier", help: "Validated against the server role catalog; no client role mapping is trusted." },
    ],
  },
  evidence: {
    eyebrow: "Evidence explorer",
    title: "Replay a calculation record",
    description: "Replay uses the original facts, rule digest, engine version, and evidence commitments returned by the service.",
    submit: "Request evidence replay",
    endpoint: "/api/v1/tax/quotes",
    fields: [
      { key: "calculation_id", label: "Calculation ID", help: "The server-issued calculation identifier." },
    ],
  },
};

function jsonValue(value: string, label: string): unknown {
  try { return JSON.parse(value); } catch { throw new Error(`${label} must contain valid JSON.`); }
}

function buildPayload(section: WorkflowSection, values: WorkflowValues, idempotencyKey: string): { path: string; method: "POST" | "GET"; body?: unknown; headers?: Record<string, string> } {
  if (section === "quotes") return { path: "/api/v1/tax/quote", method: "POST", body: { idempotency_key: idempotencyKey, rule_set_id: values.rule_set_id.trim(), rule_version: Number(values.rule_version), jurisdiction: values.jurisdiction.trim(), regime: values.regime.trim(), facts: { gross_atomic: values.gross_atomic.trim(), net_atomic: values.net_atomic.trim(), atomic_decimals: Number(values.atomic_decimals), currency: values.currency.trim().toUpperCase(), jurisdiction: values.jurisdiction.trim() }, jurisdiction_evidence: jsonValue(values.jurisdiction_evidence, "Jurisdiction evidence") } };
  if (section === "rules") return { path: "/api/v1/tax/rules", method: "POST", body: { idempotency_key: idempotencyKey, external_id: values.rule_external_id.trim(), version: Number(values.rule_version), jurisdiction: values.jurisdiction.trim(), regime: values.regime.trim(), effective_from_unix: Number(values.effective_from_unix), effective_until_unix: values.effective_until_unix.trim() ? Number(values.effective_until_unix) : null, rule_set: jsonValue(values.rule_document, "Rule document"), provenance: jsonValue(values.review_evidence, "Review provenance") } };
  if (section === "settlements") return { path: `/api/v1/settlement/intents/${encodeURIComponent(values.intent_id.trim())}/reconcile`, method: "POST", body: { evidence: jsonValue(values.review_evidence, "Review evidence") } };
  if (section === "treasury") return { path: "/api/v1/vault/release-claims", method: "POST", headers: { "idempotency-key": idempotencyKey }, body: { idempotency_key: idempotencyKey, route_id: values.route_id.trim(), destination: values.destination.trim(), nonce: values.nonce.trim(), amount_atomic: values.amount_atomic.trim(), reference_digest: jsonValue(values.reference_digest, "Reference digest"), execute_after_unix: Number(values.execute_after_unix), expires_at_unix: Number(values.expires_at_unix) } };
  if (section === "access") { const authBase = BROWSER_AUTH_SERVICE_BASE_URL || BROWSER_AUTH_CONFIG.emailOtp?.baseUrl || ""; return { path: `${authBase}/v1/admin/invitations`, method: "POST", headers: { "idempotency-key": idempotencyKey }, body: { email: values.invitation_email.trim(), role: values.server_role_id.trim() } }; }
  return { path: `/api/v1/tax/quotes/${encodeURIComponent(values.calculation_id.trim())}/replay`, method: "GET" };
}

export default function WorkflowPanel({ section, session, capability }: WorkflowPanelProps) {
  const config = sectionConfig[section];
  const [values, setValues] = useState<WorkflowValues>(initialValues);
  const [validationError, setValidationError] = useState<string | null>(null);
  const [idempotencyKey, setIdempotencyKey] = useState(() => createClientUuidV7());
  const mutation = useMutation({ mutationFn: (request: { path: string; method: "POST" | "GET"; body?: unknown; headers?: Record<string, string> }) => requestJson(request.path, session, { method: request.method, headers: { ...(request.headers ?? {}), ...(request.body === undefined ? {} : { "content-type": "application/json" }) }, ...(request.body === undefined ? {} : { body: JSON.stringify(request.body) }) }) });
  const editable = Boolean(session && capability?.decision === "allow");
  const resultText = useMemo(() => mutation.data ? JSON.stringify(mutation.data, null, 2) : null, [mutation.data]);
  function update(key: keyof WorkflowValues, value: string) { setValues((current) => ({ ...current, [key]: value })); setValidationError(null); mutation.reset(); }
  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!editable) return;
    try { setValidationError(null); mutation.mutate(buildPayload(section, values, idempotencyKey)); } catch (cause) { setValidationError(cause instanceof Error ? cause.message : "The request contains invalid fields."); }
  }
  if (section === "evidence") return null;
  return <article className="rounded-2xl border border-admin-line bg-admin-surface p-6 shadow-soft"><div className="mb-6 flex flex-col justify-between gap-4 sm:flex-row sm:items-start"><div><p className="text-xs font-black uppercase tracking-[0.12em] text-admin-primary">{config.eyebrow}</p><h2 className="mt-1 text-xl font-black text-admin-ink">{config.title}</h2><p className="mt-2 max-w-2xl text-sm leading-6 text-admin-muted">{config.description}</p></div><span className={`rounded-full px-3 py-1 text-xs font-bold ${editable ? "bg-admin-green-soft text-admin-green" : "bg-admin-amber-soft text-admin-amber-ink"}`}>{editable ? "Server permission confirmed" : "Awaiting server permission"}</span></div><form onSubmit={submit} className="grid gap-5 sm:grid-cols-2">{config.fields.map((field) => <label key={field.key} htmlFor={field.key} className={field.type === "textarea" ? "sm:col-span-2" : ""}><span className="mb-2 block text-sm font-bold text-admin-ink">{field.label}</span>{field.type === "textarea" ? <textarea id={field.key} className="min-h-32 w-full rounded-xl border border-admin-line bg-admin-bg px-4 py-3 text-sm font-mono text-admin-ink outline-none transition focus:border-admin-primary focus:ring-4 focus:ring-admin-primary/10" value={values[field.key]} onChange={(event) => update(field.key, event.target.value)} disabled={!editable} required /> : <input id={field.key} className="w-full rounded-xl border border-admin-line bg-admin-bg px-4 py-3 text-sm text-admin-ink outline-none transition focus:border-admin-primary focus:ring-4 focus:ring-admin-primary/10 disabled:cursor-not-allowed disabled:opacity-60" type={field.type ?? "text"} value={values[field.key]} onChange={(event) => update(field.key, event.target.value)} disabled={!editable} required />}<small className="mt-1.5 block text-xs leading-5 text-admin-muted">{field.help}</small></label>)}<div className="sm:col-span-2 rounded-xl border border-admin-line bg-admin-bg p-4"><div className="flex flex-wrap items-center justify-between gap-3"><div><span className="block text-xs font-black uppercase tracking-[0.12em] text-admin-muted">Safe retry reference</span><code className="mt-1 block break-all text-xs text-admin-ink">{idempotencyKey}</code></div><button type="button" className="rounded-lg border border-admin-line px-3 py-2 text-xs font-bold text-admin-ink transition hover:bg-admin-surface-muted" onClick={() => { setIdempotencyKey(createClientUuidV7()); mutation.reset(); setValidationError(null); }}>New reference</button></div><p className="mt-2 text-xs leading-5 text-admin-muted">Keep this reference to safely retry the same request. Start a new reference before changing a previously submitted request.</p></div>{(validationError || mutation.error) && <div className="sm:col-span-2 rounded-xl border border-admin-red/30 bg-admin-red-soft p-4 text-sm leading-5 text-admin-red" role="alert">{validationError ?? (mutation.error instanceof Error ? mutation.error.message : "The server rejected the request.")}</div>}{mutation.isSuccess && <div className="sm:col-span-2 rounded-xl border border-admin-green/30 bg-admin-green-soft p-4 text-sm leading-5 text-admin-green" role="status"><strong className="block">Server response received</strong><pre className="mt-2 max-h-48 overflow-auto whitespace-pre-wrap text-xs">{resultText}</pre></div>}<div className="sm:col-span-2 flex flex-wrap items-center justify-between gap-3 border-t border-admin-line pt-5"><p className="max-w-xl text-xs leading-5 text-admin-muted">Submission is bound to the authenticated tenant and server capability snapshot. This form never signs a wallet transaction or grants a role.</p><button type="submit" disabled={!editable || mutation.isPending} className="rounded-xl bg-admin-primary px-5 py-3 text-sm font-bold text-white shadow-soft transition hover:bg-admin-primary-strong disabled:cursor-not-allowed disabled:opacity-50">{mutation.isPending ? "Waiting for service…" : config.submit}</button></div></form></article>;
}
