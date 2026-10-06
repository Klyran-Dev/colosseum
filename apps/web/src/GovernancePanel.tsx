import { FormEvent, useMemo, useState } from "react";
import { fetchWithTimeout, operatorResponseError, REQUEST_TIMEOUT_MS } from "./api";
import { createClientUuid } from "./clientIdentity";
import { DEFAULT_LOCALE, DEFAULT_TIME_ZONE, formatAtomic as formatAtomicValue, formatTimestamp as formatTimestampValue } from "./formatters";
import RuleWorkbench from "./RuleWorkbenchPanel";
import {
  parseCreatedRuleDraft,
  GovernanceState,
  nextGovernanceStatuses,
  parseGovernanceState,
} from "./governance";
import {
  buildRuleSet,
  defaultRuleWorkbenchDraft,
  RuleWorkbenchDraft,
} from "./ruleWorkbench";
import { parseTaxSimulation, TaxSimulation } from "./taxSimulation";
import TaxDocumentSimulationPanel from "./TaxDocumentSimulationPanel";
import { governanceMessagesFor } from "./governanceMessages";
import { assetScaleMessagesFor, parseAssetScale } from "./assetScaleMessages";
import { operatorErrorMessage } from "./operatorError";
import TaxTreatmentOutcome from "./TaxTreatmentOutcome";

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

export default function GovernancePanel({ apiBase, serviceOnline, accessToken, locale = DEFAULT_LOCALE, timeZone = DEFAULT_TIME_ZONE }: GovernancePanelProps) {
  const messages = governanceMessagesFor(locale);
  const assetScaleMessages = assetScaleMessagesFor(locale);
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
  const [draftSourceReference, setDraftSourceReference] = useState("");
  const [draftSourceDigest, setDraftSourceDigest] = useState("");
  const [draftLegalTaxOwner, setDraftLegalTaxOwner] = useState("");
  const [draftReviewedAt, setDraftReviewedAt] = useState("");
  const [draftReviewReference, setDraftReviewReference] = useState("");
  const [draftIdempotencyKey, setDraftIdempotencyKey] = useState<string | null>(null);
  const [draftIdempotencyPayload, setDraftIdempotencyPayload] = useState<string | null>(null);
  const [isCreating, setIsCreating] = useState(false);
  const [simulationGross, setSimulationGross] = useState("");
  const [simulationNet, setSimulationNet] = useState("");
  const [simulationAtomicDecimals, setSimulationAtomicDecimals] = useState("");
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
      setError(messages.errors.loadAuth);
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
      if (!response.ok) throw operatorResponseError(response, payload, messages.common.httpError(messages.errors.load, response.status));
      const loaded = parseGovernanceState(payload);
      setRuleSetId(loaded.rule_set_id);
      setState(loaded);
      setSimulation(null);
      setTargetStatus(nextGovernanceStatuses(loaded.status)[0] ?? "");
      if (announce) setNotice(messages.notices.loaded);
      return true;
    } catch (cause) {
      setState(null);
      setTargetStatus("");
      setError(operatorErrorMessage(cause, messages.errors.load));
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
      setError(messages.errors.unavailable);
      return;
    }
    let ruleSet: unknown;
    let provenance: Record<string, unknown>;
    try {
      ruleSet = ruleEditorMode === "structured" ? buildRuleSet(structuredRuleDraft) : JSON.parse(draftRuleDocument);
    } catch (cause) {
      setError(operatorErrorMessage(cause, messages.errors.jsonDocument));
      return;
    }
    if (typeof ruleSet !== "object" || ruleSet === null || Array.isArray(ruleSet)) {
      setError(messages.errors.jsonObject);
      return;
    }
    const version = Number(draftVersion);
    const ruleDocument = ruleSet as Record<string, unknown>;
    if (ruleDocument.id !== draftExternalId.trim() || ruleDocument.version !== version) {
      setError(messages.errors.metadata);
      return;
    }
    const effectiveFrom = Date.parse(draftEffectiveFrom);
    if (!Number.isSafeInteger(version) || version < 1 || !Number.isFinite(effectiveFrom)) {
      setError(messages.errors.effectiveDate);
      return;
    }
    const sourceReference = draftSourceReference.trim();
    const sourceDigest = draftSourceDigest.trim();
    const legalTaxOwner = draftLegalTaxOwner.trim();
    const reviewReference = draftReviewReference.trim();
    const reviewedAt = Date.parse(draftReviewedAt);
    if (!sourceReference || sourceReference.length > 512 || !/^[0-9a-f]{64}$/i.test(sourceDigest)
        || !legalTaxOwner || legalTaxOwner.length > 256 || !reviewReference || reviewReference.length > 512
        || !Number.isFinite(reviewedAt) || reviewedAt < 1000 || reviewedAt > 4102444800000) {
      setError(messages.errors.provenance);
      return;
    }
    provenance = {
      schema: "tax-rule-provenance/v1",
      source_reference: sourceReference,
      source_digest_hex: sourceDigest,
      legal_tax_owner: legalTaxOwner,
      jurisdiction_scope: draftJurisdiction.trim(),
      reviewed_at_unix: Math.floor(reviewedAt / 1000),
      review_reference: reviewReference,
    };
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
      if (!response.ok) throw operatorResponseError(response, payload, messages.common.httpError(messages.errors.draft, response.status));
      const created = parseCreatedRuleDraft(payload);
      setRuleSetId(created.rule_set_id);
      const loaded = await loadGovernanceFor(created.rule_set_id, false);
      if (!loaded) return;
      setDraftIdempotencyKey(null);
      setDraftIdempotencyPayload(null);
      setNotice(messages.notices.draftCreated(created.external_id, created.version));
    } catch (cause) {
      setError(operatorErrorMessage(cause, messages.errors.draft));
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
      setError(messages.errors.evidenceJson);
      return;
    }
    if (typeof parsedEvidence !== "object" || parsedEvidence === null || Array.isArray(parsedEvidence)) {
      setError(messages.errors.evidenceObject);
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
      if (!response.ok) throw operatorResponseError(response, payload, messages.common.httpError(messages.errors.transition, response.status));
      const reloaded = await loadGovernanceFor(state.rule_set_id, false);
      if (reloaded) {
        setNotice(messages.notices.transitionUpdated);
      } else {
        setError(messages.notices.transitionReloadFailed);
      }
    } catch (cause) {
      setError(operatorErrorMessage(cause, messages.errors.transition));
    } finally {
      setIsSubmitting(false);
    }
  }

  async function simulatePersistedRule(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!state || !serviceOnline || !accessToken.trim()) {
      setError(messages.errors.simulationAuth);
      return;
    }
    if (!/^[0-9]+$/.test(simulationGross) || !/^[0-9]+$/.test(simulationNet) || !/^[0-9]$/.test(simulationAtomicDecimals) || !simulationCurrency.trim()) {
      setError(messages.errors.simulationFacts);
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
            atomic_decimals: parseAssetScale(simulationAtomicDecimals, assetScaleMessages.invalid),
            currency: simulationCurrency.trim().toUpperCase(),
            jurisdiction: state.jurisdiction,
          },
        }),
      }, REQUEST_TIMEOUT_MS.default);
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw operatorResponseError(response, payload, messages.common.httpError(messages.errors.simulation, response.status));
      setSimulation(parseTaxSimulation(payload));
      setNotice(messages.notices.simulation);
    } catch (cause) {
      setSimulation(null);
      setError(operatorErrorMessage(cause, messages.errors.simulation));
    } finally {
      setIsSimulating(false);
    }
  }

  const nextStatuses = state ? nextGovernanceStatuses(state.status) : [];

  return (
    <section className="governance-layout" aria-label={messages.section.ariaLabel}>
      <article className="panel governance-control-panel">
        <div className="panel-heading">
          <div><p className="eyebrow">{messages.section.eyebrow}</p><h2>{messages.section.title}</h2></div>
          <span className="status-pill">{messages.section.badge}</span>
        </div>
        <p className="panel-copy">{messages.section.description}</p>
        <details className="authoring-section" open>
          <summary>{messages.draft.summary}</summary>
          <p className="evidence-note">{messages.draft.copy}</p>
          <form onSubmit={createRuleDraft} className="form-grid">
            <label><span>{messages.draft.ruleName}</span><input value={draftExternalId} onChange={(event) => { const value = event.target.value; setDraftExternalId(value); setStructuredRuleDraft((current) => ({ ...current, id: value })); }} autoComplete="off" required /></label>
            <label><span>{messages.draft.version}</span><input inputMode="numeric" min="1" value={draftVersion} onChange={(event) => { const value = event.target.value; setDraftVersion(value); setStructuredRuleDraft((current) => ({ ...current, version: value })); }} required /></label>
            <label><span>{messages.draft.jurisdiction}</span><input value={draftJurisdiction} onChange={(event) => setDraftJurisdiction(event.target.value.toUpperCase())} required /></label>
            <label><span>{messages.draft.category}</span><input value={draftRegime} onChange={(event) => setDraftRegime(event.target.value)} required /></label>
            <label className="full-field"><span>{messages.draft.startsOn}</span><input type="datetime-local" value={draftEffectiveFrom} onChange={(event) => setDraftEffectiveFrom(event.target.value)} aria-describedby="effective-from-help" required /><small id="effective-from-help" className="field-help">{messages.draft.effectiveHelp}</small></label>
            <div className="full-field rule-editor-mode">
              <label><span>{messages.draft.entryLabel}</span><select value={ruleEditorMode} onChange={(event) => {
                const nextMode = event.target.value as "structured" | "json";
                if (nextMode === "json") {
                  try { setDraftRuleDocument(JSON.stringify(buildRuleSet(structuredRuleDraft), null, 2)); } catch { /* structured validation is shown inline */ }
                }
                setRuleEditorMode(nextMode);
                }}><option value={messages.editor.modeValues.structured}>{messages.draft.guided}</option><option value={messages.editor.modeValues.json}>{messages.draft.advanced}</option></select></label>
              {ruleEditorMode === "structured" ? <RuleWorkbench draft={structuredRuleDraft} onChange={setStructuredRuleDraft} locale={locale} /> : <>
                <label className="full-field"><span>{messages.draft.jsonLabel}</span><textarea value={draftRuleDocument} onChange={(event) => setDraftRuleDocument(event.target.value)} rows={12} spellCheck={false} required /></label>
                <p className="field-help">{messages.draft.jsonHelp}</p>
              </>}
            </div>
            <fieldset className="full-field release-evidence">
              <legend>{messages.evidence.legend}</legend>
              <p className="field-help">{messages.evidence.copy}</p>
              <div className="form-grid">
                <label><span>{messages.evidence.sourceRecord}</span><input value={draftSourceReference} onChange={(event) => setDraftSourceReference(event.target.value)} autoComplete="off" placeholder={messages.evidence.sourcePlaceholder} required /></label>
                <label><span>{messages.evidence.sourceDigest}</span><input value={draftSourceDigest} onChange={(event) => setDraftSourceDigest(event.target.value)} autoComplete="off" inputMode="text" pattern="[0-9a-fA-F]{64}" placeholder={messages.evidence.digestPlaceholder} required /><small className="field-help">{messages.evidence.digestHelp}</small></label>
                <label><span>{messages.evidence.owner}</span><input value={draftLegalTaxOwner} onChange={(event) => setDraftLegalTaxOwner(event.target.value)} autoComplete="organization" placeholder={messages.evidence.ownerPlaceholder} required /></label>
                <label><span>{messages.evidence.reviewedOn}</span><input type="datetime-local" value={draftReviewedAt} onChange={(event) => setDraftReviewedAt(event.target.value)} required /></label>
                <label className="full-field"><span>{messages.evidence.reviewRecord}</span><input value={draftReviewReference} onChange={(event) => setDraftReviewReference(event.target.value)} autoComplete="off" placeholder={messages.evidence.reviewRecordPlaceholder} required /></label>
              </div>
            </fieldset>
            <button type="submit" disabled={!serviceOnline || isCreating || !accessToken.trim()}>{isCreating ? messages.draft.creating : messages.draft.create}</button>
          </form>
        </details>
        <form onSubmit={loadGovernance} className="form-grid">
          <label className="full-field"><span>{messages.lookup.savedReference}</span><input value={ruleSetId} onChange={(event) => setRuleSetId(event.target.value)} placeholder={messages.lookup.placeholder} autoComplete="off" spellCheck={false} required /></label>
          <button type="submit" disabled={!serviceOnline || isLoading || !ruleSetId.trim() || !accessToken.trim()}>{isLoading ? messages.lookup.loading : messages.lookup.load}</button>
        </form>
        {!serviceOnline && <p className="inline-warning" role="alert">{messages.common.unavailable}</p>}
        {error && <p className="inline-error" role="alert">{error}</p>}
        {notice && <p className="inline-success" role="status">{notice}</p>}
        {state && <form onSubmit={submitTransition} className="governance-action-form">
          <div className="governance-current-state"><span>{messages.transition.currentStatus}</span><strong>{messages.statusLabels[state.status] ?? state.status}</strong><small>{messages.roleLabels[state.status] ?? state.status} · {messages.transition.change} {state.governance_revision}</small></div>
          {nextStatuses.length > 0 ? <>
            <label><span>{messages.transition.nextAction}</span><select value={targetStatus} onChange={(event) => setTargetStatus(event.target.value as GovernanceState["status"])}>{nextStatuses.map((status) => <option key={status} value={status}>{messages.transitionLabel(messages.statusLabels[state.status] ?? state.status, messages.statusLabels[status] ?? status)}</option>)}</select></label>
            <label><span>{messages.transition.reason}</span><textarea value={evidence} onChange={(event) => setEvidence(event.target.value)} aria-describedby="decision-evidence-help" rows={6} spellCheck={false} /><small id="decision-evidence-help" className="field-help">{messages.transition.reasonHelp}</small></label>
            <button type="submit" disabled={isSubmitting || !targetStatus || !accessToken.trim() || !serviceOnline}>{isSubmitting ? messages.transition.submitting : targetStatus ? messages.transitionLabel(messages.statusLabels[state.status] ?? state.status, messages.statusLabels[targetStatus] ?? targetStatus) : messages.transition.advance}</button>
          </> : <p className="inline-warning">{messages.transition.retired}</p>}
        </form>}
      </article>

      <article className="panel governance-evidence-panel" aria-live="polite">
        <div className="panel-heading"><div><p className="eyebrow">{messages.history.eyebrow}</p><h2>{messages.history.title}</h2></div>{state && <span className="status-pill status-pill-warning">{messages.statusLabels[state.status] ?? state.status}</span>}</div>
        <details className="authoring-section simulation-section">
          <summary>{messages.simulation.summary}</summary>
          <p className="evidence-note">{messages.history.simulationCopy}</p>
          <form onSubmit={simulatePersistedRule} className="form-grid">
            <label><span>{messages.simulation.gross}</span><input inputMode="numeric" min="0" value={simulationGross} onChange={(event) => setSimulationGross(event.target.value)} required /></label>
            <label><span>{messages.simulation.net}</span><input inputMode="numeric" min="0" value={simulationNet} onChange={(event) => setSimulationNet(event.target.value)} required /></label>
            <label><span>{messages.simulation.currency}</span><input value={simulationCurrency} onChange={(event) => setSimulationCurrency(event.target.value.toUpperCase())} maxLength={12} required /></label>
            <label><span>{assetScaleMessages.label}</span><input inputMode="numeric" min="0" max="9" value={simulationAtomicDecimals} onChange={(event) => setSimulationAtomicDecimals(event.target.value)} aria-describedby="governance-asset-precision-help" required /><small id="governance-asset-precision-help" className="field-help">{assetScaleMessages.help}</small></label>
            <p className="field-help full-field">{messages.simulation.jurisdictionBound(state?.jurisdiction ?? messages.common.loadRuleFirst)}</p>
            <button type="submit" disabled={!state || !serviceOnline || isSimulating || !accessToken.trim()}>{isSimulating ? messages.simulation.running : messages.simulation.run}</button>
          </form>
        {simulation && <div className="simulation-result" role="status">
            <div className="governance-current-state"><span>{messages.simulation.controlledResult}</span><strong>{formatAtomicValue(simulation.result.total_tax_atomic, simulation.result.currency, locale, simulation.result.atomic_decimals)} {messages.common.tax}</strong><small>{messages.history.calculationReference} {simulation.calculation_id} · {messages.history.rule} {simulation.result.rule_set_id} v{simulation.result.rule_version}</small></div>
            <TaxTreatmentOutcome outcome={simulation.result.treatment_outcome} currency={simulation.result.currency} atomicDecimals={simulation.result.atomic_decimals} locale={locale} formatAtomic={(value, currency, atomicDecimals) => formatAtomicValue(value, currency, locale, atomicDecimals)} />
            <dl className="summary-list"><div><dt>{messages.history.netAfterTax}</dt><dd>{formatAtomicValue(simulation.result.total_net_atomic, simulation.result.currency, locale, simulation.result.atomic_decimals)}</dd></div><div><dt>{messages.history.engine}</dt><dd>{simulation.result.engine_version}</dd></div><div><dt>{messages.history.ruleDigest}</dt><dd><code>{simulation.result.rule_digest.slice(0, 8).map((byte) => byte.toString(16).padStart(2, "0")).join("")}…</code></dd></div></dl>
            <div className="trace-list">{simulation.result.components.map((component) => <div className="trace-row" key={component.id}><div><strong>{component.label}</strong><small>{component.explanation}</small></div><strong>{formatAtomicValue(component.tax_atomic, simulation.result.currency, locale, simulation.result.atomic_decimals)}</strong></div>)}</div>
          </div>}
        </details>
        {!state ? <div className="empty-state"><span className="empty-icon">◎</span><strong>{messages.history.noRule}</strong><p>{messages.history.noRuleCopy}</p></div> : <div className="governance-evidence-content">
          <dl className="summary-list"><div><dt>{messages.history.identity}</dt><dd>{state.external_id} v{state.version}</dd></div><div><dt>{messages.history.jurisdictionRegime}</dt><dd>{state.jurisdiction} / {state.regime}</dd></div><div><dt>{messages.history.ruleDigest}</dt><dd><code>{state.rule_digest_hex.slice(0, 16)}…</code></dd></div><div><dt>{messages.history.effectiveFrom}</dt><dd>{formatTimestampValue(state.effective_from, locale, timeZone)}</dd></div><div><dt>{messages.history.author}</dt><dd>{state.author_subject ?? messages.common.notAssigned}</dd></div><div><dt>{messages.history.reviewer}</dt><dd>{state.reviewer_subject ?? messages.common.notAssigned}</dd></div><div><dt>{messages.history.approver}</dt><dd>{state.approver_subject ?? messages.common.notAssigned}</dd></div></dl>
          <div><h3 className="subheading">{messages.history.decisionTrail} <span>{state.events.length}</span></h3>{state.events.length === 0 ? <p className="evidence-note">{messages.history.noEvents}</p> : <ol className="governance-timeline">{state.events.map((event) => <li key={event.id}><div className="timeline-marker" /><div><strong>{event.from_status in messages.statusLabels ? messages.statusLabels[event.from_status as keyof typeof messages.statusLabels] : event.from_status} → {event.to_status in messages.statusLabels ? messages.statusLabels[event.to_status as keyof typeof messages.statusLabels] : event.to_status}</strong><small>{messages.history.actor(event.action, event.actor_subject)} · {formatTimestampValue(event.occurred_at, locale, timeZone)}</small><details><summary>{messages.history.evidence}</summary><pre>{JSON.stringify(event.evidence, null, 2)}</pre></details></div></li>)}</ol>}</div>
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
