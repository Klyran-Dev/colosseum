import { useMemo } from "react";
import {
  buildRuleSet,
  defaultComponentDraft,
  RuleComponentDraft,
  RuleWorkbenchDraft,
  validateRuleWorkbench,
} from "./ruleWorkbench";

type RuleWorkbenchProps = {
  draft: RuleWorkbenchDraft;
  onChange: (draft: RuleWorkbenchDraft) => void;
};

function updateComponent(
  draft: RuleWorkbenchDraft,
  index: number,
  patch: Partial<RuleComponentDraft>,
): RuleWorkbenchDraft {
  return {
    ...draft,
    components: draft.components.map((component, componentIndex) =>
      componentIndex === index ? { ...component, ...patch } : component,
    ),
  };
}

function updateBracket(
  draft: RuleWorkbenchDraft,
  componentIndex: number,
  bracketIndex: number,
  patch: Partial<RuleComponentDraft["brackets"][number]>,
): RuleWorkbenchDraft {
  const component = draft.components[componentIndex];
  return updateComponent(draft, componentIndex, {
    brackets: component.brackets.map((bracket, index) => index === bracketIndex ? { ...bracket, ...patch } : bracket),
  });
}

function componentChoices(draft: RuleWorkbenchDraft, currentId: string): string[] {
  return ["Gross", "Net", ...draft.components.map((component) => component.id.trim()).filter((id) => id && id !== currentId).map((id) => `Component:${id}`)];
}

export default function RuleWorkbench({ draft, onChange }: RuleWorkbenchProps) {
  const errors = useMemo(() => validateRuleWorkbench(draft), [draft]);
  const generated = useMemo(() => {
    try {
      return JSON.stringify(buildRuleSet(draft), null, 2);
    } catch {
      return null;
    }
  }, [draft]);

  function addComponent() {
    if (draft.components.length >= 128) return;
    onChange({ ...draft, components: [...draft.components, defaultComponentDraft(draft.components.length + 1)] });
  }

  function removeComponent(index: number) {
    if (draft.components.length <= 1) return;
    onChange({ ...draft, components: draft.components.filter((_, componentIndex) => componentIndex !== index) });
  }

  return (
    <section className="rule-workbench" aria-labelledby="rule-workbench-heading">
      <div className="workbench-heading">
        <div>
          <p className="eyebrow">Structured Tier-1 authoring</p>
          <h3 id="rule-workbench-heading">Rule components and semantics</h3>
        </div>
        <span className="status-pill">Server validates</span>
      </div>
      <p className="evidence-note">Build the canonical rule document from explicit controls. Rates and money remain decimal strings; this browser does not calculate tax or publish policy.</p>

      <div className="workbench-components">
        {draft.components.map((component, componentIndex) => {
          const choices = componentChoices(draft, component.id);
          return (
            <fieldset className="workbench-component" key={`${component.id}-${componentIndex}`}>
              <legend>Component {componentIndex + 1}</legend>
              <div className="form-grid">
                <label><span>Component ID</span><input value={component.id} onChange={(event) => onChange(updateComponent(draft, componentIndex, { id: event.target.value }))} autoComplete="off" /></label>
                <label><span>Display label</span><input value={component.label} onChange={(event) => onChange(updateComponent(draft, componentIndex, { label: event.target.value }))} /></label>
                <div className="full-field">
                  <span className="field-label">Tax-base sources</span>
                  <div className="workbench-source-list">
                    {component.baseSources.map((source, sourceIndex) => (
                      <div className="workbench-inline-row" key={`${source}-${sourceIndex}`}>
                        <label className="inline-control"><span className="sr-only">Base source {sourceIndex + 1}</span><select value={source} onChange={(event) => onChange(updateComponent(draft, componentIndex, { baseSources: component.baseSources.map((item, index) => index === sourceIndex ? event.target.value : item) }))}>{choices.map((choice) => <option key={choice} value={choice}>{choice.startsWith("Component:") ? choice : choice}</option>)}</select></label>
                        <button type="button" className="secondary-button" onClick={() => onChange(updateComponent(draft, componentIndex, { baseSources: component.baseSources.filter((_, index) => index !== sourceIndex) }))} disabled={component.baseSources.length <= 1}>Remove source</button>
                      </div>
                    ))}
                  </div>
                  <button type="button" className="secondary-button" onClick={() => onChange(updateComponent(draft, componentIndex, { baseSources: [...component.baseSources, "Net"] }))} disabled={component.baseSources.length >= 128}>Add base source</button>
                  <small className="field-help">Choose another component to model tax-on-tax or cascading bases. The server resolves dependency order and rejects cycles.</small>
                </div>
                <label><span>Rate model</span><select value={component.rateMode} onChange={(event) => onChange(updateComponent(draft, componentIndex, { rateMode: event.target.value as RuleComponentDraft["rateMode"] }))}><option value="flat">Flat rate</option><option value="progressive">Progressive brackets</option></select></label>
                <label><span>Price mode</span><select value={component.priceMode} onChange={(event) => onChange(updateComponent(draft, componentIndex, { priceMode: event.target.value as RuleComponentDraft["priceMode"] }))}><option value="Exclusive">Exclusive</option><option value="Inclusive">Inclusive</option></select></label>
                {component.rateMode === "flat" ? <>
                  <label><span>Rate numerator</span><input inputMode="numeric" value={component.flatNumerator} onChange={(event) => onChange(updateComponent(draft, componentIndex, { flatNumerator: event.target.value }))} /></label>
                  <label><span>Rate denominator</span><input inputMode="numeric" value={component.flatDenominator} onChange={(event) => onChange(updateComponent(draft, componentIndex, { flatDenominator: event.target.value }))} /></label>
                </> : <div className="full-field">
                  <span className="field-label">Progressive brackets</span>
                  <div className="bracket-list">
                    {component.brackets.map((bracket, bracketIndex) => <div className="bracket-row" key={`${bracket.lowerAtomic}-${bracketIndex}`}>
                      <label><span>Lower atomic</span><input inputMode="numeric" value={bracket.lowerAtomic} onChange={(event) => onChange(updateBracket(draft, componentIndex, bracketIndex, { lowerAtomic: event.target.value }))} /></label>
                      <label><span>Upper atomic</span><input inputMode="numeric" value={bracket.upperAtomic} disabled={bracket.openEnded} onChange={(event) => onChange(updateBracket(draft, componentIndex, bracketIndex, { upperAtomic: event.target.value }))} placeholder={bracket.openEnded ? "Open-ended" : "Required"} /></label>
                      <label><span>Numerator</span><input inputMode="numeric" value={bracket.numerator} onChange={(event) => onChange(updateBracket(draft, componentIndex, bracketIndex, { numerator: event.target.value }))} /></label>
                      <label><span>Denominator</span><input inputMode="numeric" value={bracket.denominator} onChange={(event) => onChange(updateBracket(draft, componentIndex, bracketIndex, { denominator: event.target.value }))} /></label>
                      <label className="checkbox-control"><span>Open-ended</span><input type="checkbox" checked={bracket.openEnded} onChange={(event) => onChange(updateBracket(draft, componentIndex, bracketIndex, { openEnded: event.target.checked }))} /></label>
                      <button type="button" className="secondary-button" onClick={() => onChange(updateComponent(draft, componentIndex, { brackets: component.brackets.filter((_, index) => index !== bracketIndex) }))} disabled={component.brackets.length <= 1}>Remove bracket</button>
                    </div>)}
                  </div>
                  <button type="button" className="secondary-button" onClick={() => {
                    const previous = component.brackets[component.brackets.length - 1];
                    onChange(updateComponent(draft, componentIndex, { brackets: [...component.brackets, { lowerAtomic: previous?.openEnded ? "0" : previous?.upperAtomic ?? "0", upperAtomic: "", numerator: "10", denominator: "100", openEnded: true }] }));
                  }} disabled={component.brackets.length >= 128}>Add bracket</button>
                  <small className="field-help">Brackets must start at zero, remain contiguous, and have only a final open-ended row.</small>
                </div>}
                <label><span>Credit atomic units</span><input inputMode="numeric" value={component.creditAtomic} onChange={(event) => onChange(updateComponent(draft, componentIndex, { creditAtomic: event.target.value }))} /></label>
                <label><span>Floor atomic units (optional)</span><input inputMode="numeric" value={component.floorAtomic} onChange={(event) => onChange(updateComponent(draft, componentIndex, { floorAtomic: event.target.value }))} placeholder="No floor" /></label>
                <label><span>Cap atomic units (optional)</span><input inputMode="numeric" value={component.capAtomic} onChange={(event) => onChange(updateComponent(draft, componentIndex, { capAtomic: event.target.value }))} placeholder="No cap" /></label>
                <label><span>Rounding / truncation</span><select value={component.roundingMode} onChange={(event) => onChange(updateComponent(draft, componentIndex, { roundingMode: event.target.value as RuleComponentDraft["roundingMode"] }))}><option value="Floor">Floor / truncate</option><option value="Ceil">Ceil</option><option value="HalfUp">Half up</option><option value="HalfEven">Half even</option><option value="AwayFromZero">Away from zero</option></select></label>
                <div className="full-field workbench-threshold">
                  <label className="checkbox-control"><span>Enable threshold semantics</span><input type="checkbox" checked={component.thresholdEnabled} onChange={(event) => onChange(updateComponent(draft, componentIndex, { thresholdEnabled: event.target.checked }))} /></label>
                  {component.thresholdEnabled && <div className="form-grid">
                    <label><span>Threshold atomic units</span><input inputMode="numeric" value={component.thresholdAtomic} onChange={(event) => onChange(updateComponent(draft, componentIndex, { thresholdAtomic: event.target.value }))} /></label>
                    <label><span>Threshold basis</span><select value={component.thresholdBasis} onChange={(event) => onChange(updateComponent(draft, componentIndex, { thresholdBasis: event.target.value as RuleComponentDraft["thresholdBasis"] }))}><option value="PerTransaction">Per transaction</option><option value="CumulativePeriod">Cumulative period</option></select></label>
                    <label><span>Threshold behavior</span><select value={component.thresholdMode} onChange={(event) => onChange(updateComponent(draft, componentIndex, { thresholdMode: event.target.value as RuleComponentDraft["thresholdMode"] }))}><option value="ExemptUpTo">Exempt up to</option><option value="TriggerAt">Trigger at</option></select></label>
                  </div>}
                  <small className="field-help">Inclusive thresholds require a separately versioned equation and are rejected by DSL v1.</small>
                </div>
                <label className="full-field"><span>Exemption codes (one per line or comma-separated)</span><textarea value={component.exemptionCodes} onChange={(event) => onChange(updateComponent(draft, componentIndex, { exemptionCodes: event.target.value }))} rows={2} /></label>
              </div>
              <button type="button" className="secondary-button" onClick={() => removeComponent(componentIndex)} disabled={draft.components.length <= 1}>Remove component</button>
            </fieldset>
          );
        })}
      </div>
      <button type="button" className="secondary-button" onClick={addComponent} disabled={draft.components.length >= 128}>Add tax component</button>

      <div className="workbench-validation" aria-live="polite">
        {errors.length === 0 ? <p className="inline-success">The structured document is internally consistent and ready for server validation.</p> : <div className="inline-warning"><strong>Resolve before submitting</strong><ul>{errors.map((error) => <li key={error}>{error}</li>)}</ul></div>}
      </div>
      <details className="workbench-preview">
        <summary>Preview canonical RuleSet JSON</summary>
        {generated ? <pre>{generated}</pre> : <p className="inline-warning">The document preview is unavailable until the structured fields are valid.</p>}
      </details>
    </section>
  );
}
