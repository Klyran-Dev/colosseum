import { FormEvent, useMemo, useState } from "react";
import { fetchWithTimeout, REQUEST_TIMEOUT_MS, responseError } from "./api";
import { createClientUuid } from "./clientIdentity";
import { DEFAULT_LOCALE, DEFAULT_TIME_ZONE, formatAtomic as formatAtomicValue, formatTimestamp as formatTimestampValue } from "./formatters";
import RuleWorkbench from "./RuleWorkbenchPanel";
import {
  parseCreatedRuleDraft,
  GovernanceState,
  nextGovernanceStatuses,
  parseGovernanceState,
  transitionLabel,
} from "./governance";
import {
  buildRuleSet,
  defaultRuleWorkbenchDraft,
  RuleWorkbenchDraft,
} from "./ruleWorkbench";
import { parseTaxSimulation, TaxSimulation } from "./taxSimulation";
import TaxDocumentSimulationPanel from "./TaxDocumentSimulationPanel";

const DEFAULT_RULE_DOCUMENT = `{
  "id": "new-sales-tax",
  "version": 1,
  "components": [
    {
      "id": "sales-tax",
      "label": "Sales tax",
      "base": { "sources": ["Net"] },
      "rate": { "Flat": { "numerator": "10", "denominator": "100" } },
      "price_mode": "Exclusive",
      "credit_atomic": "0",
      "floor_atomic": null,
      "cap_atomic": null,
      "rounding": { "mode": "HalfUp" },
      "threshold": null,
      "exemption_codes": []
    }
  ]
}`;

type GovernancePanelProps = {
  apiBase: string;
  serviceOnline: boolean;
  accessToken: string;
  locale?: string;
  timeZone?: string;
};

function roleCopy(status: GovernanceState["status"]): string {
  switch (status) {
    case "draft": return "Author action required";
    case "simulated": return "Independent reviewer action required";
    case "reviewed": return "Independent approver action required";
    case "approved": return "Recorded approver publishes";
    case "published": return "Recorded approver retires";
    case "retired": return "Lifecycle complete";
  }
}

export default function GovernancePanel({ apiBase, serviceOnline, accessToken, locale = DEFAULT_LOCALE, timeZone = DEFAULT_TIME_ZONE }: GovernancePanelProps) {
  const [ruleSetId, setRuleSetId] = useState("");
  const [state, setState] = useState<GovernanceState | null>(null);
  const [targetStatus, setTargetStatus] = useState<GovernanceState["status"] | "">("");
  const [evidence, setEvidence] = useState('{\n  "source": "operator-console"\n}');
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [draftExternalId, setDraftExternalId] = useState("new-sales-tax");
  const [draftVersion, setDraftVersion] = useState("1");
  const [draftJurisdiction, setDraftJurisdiction] = useState("US-CA");
  const [draftRegime, setDraftRegime] = useState("sales-tax");
  const [draftEffectiveFrom, setDraftEffectiveFrom] = useState(() => {
    const now = new Date();
    now.setSeconds(0, 0);
    return now.toISOString().slice(0, 16);
  });
  const [draftRuleDocument, setDraftRuleDocument] = useState(DEFAULT_RULE_DOCUMENT);
  const [ruleEditorMode, setRuleEditorMode] = useState<"structured" | "json">("structured");
  const [structuredRuleDraft, setStructuredRuleDraft] = useState<RuleWorkbenchDraft>(() => defaultRuleWorkbenchDraft());
  const [draftProvenance, setDraftProvenance] = useState('{\n  "source": "operator-console"\n}');
  const [draftIdempotencyKey, setDraftIdempotencyKey] = useState<string | null>(null);
  const [draftIdempotencyPayload, setDraftIdempotencyPayload] = useState<string | null>(null);
  const [isCreating, setIsCreating] = useState(false);
  const [simulationGross, setSimulationGross] = useState("");
  const [simulationNet, setSimulationNet] = useState("");
  const [simulationCurrency, setSimulationCurrency] = useState("");
  const [simulation, setSimulation] = useState<TaxSimulation | null>(null);
  const [isSimulating, setIsSimulating] = useState(false);
  const documentRuleSet = useMemo(() => {
    try {
      return ruleEditorMode === "structured" ? buildRuleSet(structuredRuleDraft) : JSON.parse(draftRuleDocument);
    } catch {
      return null;
    }
  }, [draftRuleDocument, ruleEditorMode, structuredRuleDraft]);

  async function loadGovernanceFor(id: string, announce = true) {
    if (!id.trim() || !accessToken.trim()) {
      setError("Enter a rule-set UUID and an in-memory token with tax:rule:read plus a governance role.");
      return false;
    }
    setError(null);
    if (announce) setNotice(null);
    setIsLoading(true);
    try {
      const response = await fetchWithTimeout(`${apiBase}/v1/tax/rules/${encodeURIComponent(id.trim())}/governance`, {
        headers: { authorization: `Bearer ${accessToken.trim()}` },
      }, REQUEST_TIMEOUT_MS.default);
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw new Error(responseError(response, payload, `Governance lookup failed (${response.status})`));
      const loaded = parseGovernanceState(payload);
      setRuleSetId(loaded.rule_set_id);
      setState(loaded);
      setSimulation(null);
      setTargetStatus(nextGovernanceStatuses(loaded.status)[0] ?? "");
      if (announce) setNotice("Rule details loaded. Review the decision history before acting.");
      return true;
    } catch (cause) {
      setState(null);
      setTargetStatus("");
      setError(cause instanceof Error ? cause.message : "Governance lookup failed");
      return false;
    } finally {
      setIsLoading(false);
    }
  }

  async function loadGovernance(event?: FormEvent<HTMLFormElement>) {
    event?.preventDefault();
    await loadGovernanceFor(ruleSetId);
  }

  async function createRuleDraft(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!serviceOnline || !accessToken.trim()) {
      setError("The API is unavailable or the author token is missing; no local rule is created.");
      return;
    }
    let ruleSet: unknown;
    let provenance: unknown;
    try {
      ruleSet = ruleEditorMode === "structured" ? buildRuleSet(structuredRuleDraft) : JSON.parse(draftRuleDocument);
      provenance = JSON.parse(draftProvenance);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Rule document and provenance must both be valid JSON.");
      return;
    }
    if (typeof ruleSet !== "object" || ruleSet === null || Array.isArray(ruleSet)
        || typeof provenance !== "object" || provenance === null || Array.isArray(provenance)) {
      setError("Rule document and provenance must be JSON objects.");
      return;
    }
    const version = Number(draftVersion);
    const ruleDocument = ruleSet as Record<string, unknown>;
    if (ruleDocument.id !== draftExternalId.trim() || ruleDocument.version !== version) {
      setError("Rule-set ID and version in the document must match the governed draft metadata.");
      return;
    }
    const effectiveFrom = Date.parse(draftEffectiveFrom);
    if (!Number.isSafeInteger(version) || version < 1 || !Number.isFinite(effectiveFrom)) {
      setError("Version must be a positive integer and effective-from must be a valid date.");
      return;
    }
    setError(null);
    setNotice(null);
    setIsCreating(true);
    try {
      const requestWithoutIdempotencyKey = JSON.stringify({
        external_id: draftExternalId.trim(),
        version,
        jurisdiction: draftJurisdiction.trim(),
        regime: draftRegime.trim(),
        effective_from_unix: Math.floor(effectiveFrom / 1000),
        effective_until_unix: null,
        rule_set: ruleSet,
        provenance,
      });
      const idempotencyKey = draftIdempotencyPayload === requestWithoutIdempotencyKey && draftIdempotencyKey
        ? draftIdempotencyKey
        : createClientUuid();
      setDraftIdempotencyKey(idempotencyKey);
      setDraftIdempotencyPayload(requestWithoutIdempotencyKey);
      const response = await fetchWithTimeout(`${apiBase}/v1/tax/rules`, {
        method: "POST",
        headers: {
          "content-type": "application/json",
          authorization: `Bearer ${accessToken.trim()}`,
        },
        body: JSON.stringify({
          idempotency_key: idempotencyKey,
          ...JSON.parse(requestWithoutIdempotencyKey),
        }),
      }, REQUEST_TIMEOUT_MS.default);
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw new Error(responseError(response, payload, `Rule draft creation failed (${response.status})`));
      const created = parseCreatedRuleDraft(payload);
      setRuleSetId(created.rule_set_id);
      const loaded = await loadGovernanceFor(created.rule_set_id, false);
      if (!loaded) return;
      setDraftIdempotencyKey(null);
      setDraftIdempotencyPayload(null);
      setNotice(`Draft ${created.external_id} v${created.version} was created with immutable creation evidence. It is not published.`);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Rule draft creation failed");
    } finally {
      setIsCreating(false);
    }
  }

  async function submitTransition(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!state || !targetStatus) return;
    let parsedEvidence: unknown;
    try {
      parsedEvidence = JSON.parse(evidence);
    } catch {
      setError("Governance evidence must be valid JSON.");
      return;
    }
    if (typeof parsedEvidence !== "object" || parsedEvidence === null || Array.isArray(parsedEvidence)) {
      setError("Governance evidence must be a JSON object.");
      return;
    }
    setError(null);
    setNotice(null);
    setIsSubmitting(true);
    try {
      const response = await fetchWithTimeout(`${apiBase}/v1/tax/rules/${encodeURIComponent(state.rule_set_id)}/transition`, {
        method: "POST",
        headers: {
          "content-type": "application/json",
          authorization: `Bearer ${accessToken.trim()}`,
        },
        body: JSON.stringify({ to_status: targetStatus, evidence: parsedEvidence }),
      }, REQUEST_TIMEOUT_MS.default);
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw new Error(responseError(response, payload, `Governance transition failed (${response.status})`));
      const reloaded = await loadGovernanceFor(state.rule_set_id, false);
      if (reloaded) {
        setNotice(`Transition to ${targetStatus} was accepted and the snapshot was reloaded from PostgreSQL.`);
      } else {
        setError("The transition was accepted, but the follow-up governance reload failed. Reload before taking another action.");
      }
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Governance transition failed");
    } finally {
      setIsSubmitting(false);
    }
  }

  async function simulatePersistedRule(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!state || !serviceOnline || !accessToken.trim()) {
      setError("Load the rule details and sign in before running a simulation.");
      return;
    }
    if (!/^[0-9]+$/.test(simulationGross) || !/^[0-9]+$/.test(simulationNet) || !simulationCurrency.trim()) {
      setError("Simulation gross, net, and currency must be valid non-negative facts.");
      return;
    }
    setError(null);
    setNotice(null);
    setIsSimulating(true);
    try {
      const response = await fetchWithTimeout(`${apiBase}/v1/tax/rules/${encodeURIComponent(state.rule_set_id)}/simulate`, {
        method: "POST",
        headers: {
          "content-type": "application/json",
          authorization: `Bearer ${accessToken.trim()}`,
        },
        body: JSON.stringify({
          facts: {
            gross_atomic: simulationGross,
            net_atomic: simulationNet,
            currency: simulationCurrency.trim().toUpperCase(),
            jurisdiction: state.jurisdiction,
          },
        }),
      }, REQUEST_TIMEOUT_MS.default);
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw new Error(responseError(response, payload, `Rule simulation failed (${response.status})`));
      setSimulation(parseTaxSimulation(payload));
      setNotice("The server simulated the persisted rule document. This result is unattested and cannot authorize settlement.");
    } catch (cause) {
      setSimulation(null);
      setError(cause instanceof Error ? cause.message : "Rule simulation failed");
    } finally {
      setIsSimulating(false);
    }
  }

  const nextStatuses = state ? nextGovernanceStatuses(state.status) : [];

  return (
    <section className="governance-layout" aria-label="Tax rule governance workspace">
      <article className="panel governance-control-panel">
        <div className="panel-heading">
          <div><p className="eyebrow">Tax rules / review</p><h2>Review and publish a rule</h2></div>
          <span className="status-pill">Access checked</span>
        </div>
        <p className="panel-copy">Load the current rule before acting. The service confirms your responsibilities, required approvals, effective dates, and whether the record has changed.</p>
        <details className="authoring-section" open>
          <summary>Create a governed draft</summary>
          <p className="evidence-note">The service checks the complete rule, records who created it and where it came from, and keeps it in draft until the required review and approval steps are complete.</p>
          <form onSubmit={createRuleDraft} className="form-grid">
            <label><span>External rule ID</span><input value={draftExternalId} onChange={(event) => { const value = event.target.value; setDraftExternalId(value); setStructuredRuleDraft((current) => ({ ...current, id: value })); }} autoComplete="off" required /></label>
            <label><span>Version</span><input inputMode="numeric" min="1" value={draftVersion} onChange={(event) => { const value = event.target.value; setDraftVersion(value); setStructuredRuleDraft((current) => ({ ...current, version: value })); }} required /></label>
            <label><span>Jurisdiction</span><input value={draftJurisdiction} onChange={(event) => setDraftJurisdiction(event.target.value.toUpperCase())} required /></label>
            <label><span>Tax regime</span><input value={draftRegime} onChange={(event) => setDraftRegime(event.target.value)} required /></label>
            <label className="full-field"><span>Effective from (browser local time, sent as UTC)</span><input type="datetime-local" value={draftEffectiveFrom} onChange={(event) => setDraftEffectiveFrom(event.target.value)} required /></label>
            <div className="full-field rule-editor-mode">
              <label><span>Rule editing mode</span><select value={ruleEditorMode} onChange={(event) => {
                const nextMode = event.target.value as "structured" | "json";
                if (nextMode === "json") {
                  try { setDraftRuleDocument(JSON.stringify(buildRuleSet(structuredRuleDraft), null, 2)); } catch { /* structured validation is shown inline */ }
                }
                setRuleEditorMode(nextMode);
                }}><option value="structured">Guided rule builder</option><option value="json">Advanced JSON</option></select></label>
              {ruleEditorMode === "structured" ? <RuleWorkbench draft={structuredRuleDraft} onChange={setStructuredRuleDraft} /> : <>
                <label className="full-field"><span>Rule document JSON (ID and version must match above)</span><textarea value={draftRuleDocument} onChange={(event) => setDraftRuleDocument(event.target.value)} rows={12} spellCheck={false} required /></label>
                <p className="field-help">Advanced mode is for exact rule exchange. The service still validates and governs every document; this browser does not calculate or publish it.</p>
              </>}
            </div>
            <label className="full-field"><span>Provenance JSON object (max 16 KiB)</span><textarea value={draftProvenance} onChange={(event) => setDraftProvenance(event.target.value)} rows={4} spellCheck={false} required /></label>
            <button type="submit" disabled={!serviceOnline || isCreating || !accessToken.trim()}>{isCreating ? "Creating immutable draft…" : "Create governed draft"}</button>
          </form>
        </details>
        <form onSubmit={loadGovernance} className="form-grid">
          <label className="full-field"><span>Rule reference</span><input value={ruleSetId} onChange={(event) => setRuleSetId(event.target.value)} placeholder="Paste the saved rule reference" autoComplete="off" spellCheck={false} required /></label>
          <button type="submit" disabled={!serviceOnline || isLoading || !ruleSetId.trim() || !accessToken.trim()}>{isLoading ? "Loading rule details…" : "Load rule details"}</button>
        </form>
        {!serviceOnline && <p className="inline-warning" role="alert">The service is unavailable. Governance actions are disabled and no local state is substituted.</p>}
        {error && <p className="inline-error" role="alert">{error}</p>}
        {notice && <p className="inline-success" role="status">{notice}</p>}
        {state && <form onSubmit={submitTransition} className="governance-action-form">
          <div className="governance-current-state"><span>Current status</span><strong>{state.status}</strong><small>{roleCopy(state.status)} · revision {state.governance_revision}</small></div>
          {nextStatuses.length > 0 ? <>
            <label><span>Next governed action</span><select value={targetStatus} onChange={(event) => setTargetStatus(event.target.value as GovernanceState["status"])}>{nextStatuses.map((status) => <option key={status} value={status}>{transitionLabel(state.status, status)}</option>)}</select></label>
            <label><span>Decision evidence (JSON object, max 16 KiB)</span><textarea value={evidence} onChange={(event) => setEvidence(event.target.value)} rows={6} spellCheck={false} /></label>
            <button type="submit" disabled={isSubmitting || !targetStatus || !accessToken.trim() || !serviceOnline}>{isSubmitting ? "Submitting transition…" : targetStatus ? transitionLabel(state.status, targetStatus) : "Advance rule"}</button>
          </> : <p className="inline-warning">This rule is retired. No further transition is available.</p>}
        </form>}
      </article>

      <article className="panel governance-evidence-panel" aria-live="polite">
        <div className="panel-heading"><div><p className="eyebrow">Immutable read model</p><h2>Governance evidence</h2></div>{state && <span className="status-pill status-pill-warning">{state.status}</span>}</div>
        <details className="authoring-section simulation-section" open>
          <summary>Try the saved rule</summary>
          <p className="evidence-note">This uses the saved rule and the service tax engine to show a what-if result. It does not create a quote, publish the rule, or authorize a payment.</p>
          <form onSubmit={simulatePersistedRule} className="form-grid">
            <label><span>Amount before tax</span><input inputMode="numeric" min="0" value={simulationGross} onChange={(event) => setSimulationGross(event.target.value)} required /></label>
            <label><span>Amount subject to tax</span><input inputMode="numeric" min="0" value={simulationNet} onChange={(event) => setSimulationNet(event.target.value)} required /></label>
            <label><span>Currency</span><input value={simulationCurrency} onChange={(event) => setSimulationCurrency(event.target.value.toUpperCase())} maxLength={12} required /></label>
            <p className="field-help full-field">Jurisdiction is bound to the loaded rule ({state?.jurisdiction ?? "load a rule first"}); client-supplied jurisdiction evidence is rejected.</p>
            <button type="submit" disabled={!state || !serviceOnline || isSimulating || !accessToken.trim()}>{isSimulating ? "Running controlled simulation…" : "Run controlled simulation"}</button>
          </form>
        {simulation && <div className="simulation-result" role="status">
            <div className="governance-current-state"><span>Unattested server result</span><strong>{formatAtomicValue(simulation.result.total_tax_atomic, simulation.result.currency, locale)} tax</strong><small>Calculation reference {simulation.calculation_id} · rule {simulation.result.rule_set_id} v{simulation.result.rule_version}</small></div>
            <dl className="summary-list"><div><dt>Net after tax</dt><dd>{formatAtomicValue(simulation.result.total_net_atomic, simulation.result.currency, locale)}</dd></div><div><dt>Engine</dt><dd>{simulation.result.engine_version}</dd></div><div><dt>Rule digest</dt><dd><code>{simulation.result.rule_digest.slice(0, 8).map((byte) => byte.toString(16).padStart(2, "0")).join("")}…</code></dd></div></dl>
            <div className="trace-list">{simulation.result.components.map((component) => <div className="trace-row" key={component.id}><div><strong>{component.label}</strong><small>{component.explanation}</small></div><strong>{formatAtomicValue(component.tax_atomic, simulation.result.currency, locale)}</strong></div>)}</div>
          </div>}
        </details>
        {!state ? <div className="empty-state"><span className="empty-icon">◎</span><strong>No rule loaded</strong><p>Enter a saved rule reference to inspect its status, effective dates, review history, and supporting details.</p></div> : <div className="governance-evidence-content">
          <dl className="summary-list"><div><dt>Rule identity</dt><dd>{state.external_id} v{state.version}</dd></div><div><dt>Jurisdiction / regime</dt><dd>{state.jurisdiction} / {state.regime}</dd></div><div><dt>Rule digest</dt><dd><code>{state.rule_digest_hex.slice(0, 16)}…</code></dd></div><div><dt>Effective from</dt><dd>{formatTimestampValue(state.effective_from, locale, timeZone)}</dd></div><div><dt>Author</dt><dd>{state.author_subject ?? "Not assigned"}</dd></div><div><dt>Reviewer</dt><dd>{state.reviewer_subject ?? "Not assigned"}</dd></div><div><dt>Approver</dt><dd>{state.approver_subject ?? "Not assigned"}</dd></div></dl>
          <div><h3 className="subheading">Decision trail <span>{state.events.length}</span></h3>{state.events.length === 0 ? <p className="evidence-note">No transition evidence has been recorded for this draft.</p> : <ol className="governance-timeline">{state.events.map((event) => <li key={event.id}><div className="timeline-marker" /><div><strong>{event.from_status} → {event.to_status}</strong><small>{event.action} by {event.actor_subject} · {formatTimestampValue(event.occurred_at, locale, timeZone)}</small><details><summary>Evidence</summary><pre>{JSON.stringify(event.evidence, null, 2)}</pre></details></div></li>)}</ol>}</div>
          </div>}
        <TaxDocumentSimulationPanel
          apiBase={apiBase}
          serviceOnline={serviceOnline}
          ruleSet={documentRuleSet}
          jurisdiction={state?.jurisdiction ?? draftJurisdiction}
          locale={locale}
        />
      </article>
    </section>
  );
}
