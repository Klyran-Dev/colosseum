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
          <p className="eyebrow">Tax rule builder</p>
          <h3 id="rule-workbench-heading">How the tax is calculated</h3>
        </div>
        <span className="status-pill">Checked before publishing</span>
      </div>
      <p className="evidence-note">Set the tax base, rate, bands, credits, limits, exemptions, and rounding rules. The service checks the complete rule before it can be reviewed or published.</p>

      <div className="workbench-components">
        {draft.components.map((component, componentIndex) => {
          const choices = componentChoices(draft, component.id);
          return (
            <fieldset className="workbench-component" key={`${component.id}-${componentIndex}`}>
              <legend>Component {componentIndex + 1}</legend>
              <div className="form-grid">
                <label><span>Internal component name</span><input value={component.id} onChange={(event) => onChange(updateComponent(draft, componentIndex, { id: event.target.value }))} autoComplete="off" /></label>
                <label><span>Customer-facing label</span><input value={component.label} onChange={(event) => onChange(updateComponent(draft, componentIndex, { label: event.target.value }))} /></label>
                <div className="full-field">
                  <span className="field-label">What amount is taxed?</span>
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
                <label><span>How the rate changes</span><select value={component.rateMode} onChange={(event) => onChange(updateComponent(draft, componentIndex, { rateMode: event.target.value as RuleComponentDraft["rateMode"] }))}><option value="flat">One rate</option><option value="progressive">Progressive rate bands</option></select></label>
                <label><span>How the price is shown</span><select value={component.priceMode} onChange={(event) => onChange(updateComponent(draft, componentIndex, { priceMode: event.target.value as RuleComponentDraft["priceMode"] }))}><option value="Exclusive">Tax added on top</option><option value="Inclusive">Tax included in price</option></select></label>
                {component.rateMode === "flat" ? <>
                  <label><span>Rate value</span><input inputMode="numeric" value={component.flatNumerator} onChange={(event) => onChange(updateComponent(draft, componentIndex, { flatNumerator: event.target.value }))} /></label>
                  <label><span>Rate scale</span><input inputMode="numeric" value={component.flatDenominator} onChange={(event) => onChange(updateComponent(draft, componentIndex, { flatDenominator: event.target.value }))} /></label>
                </> : <div className="full-field">
                  <span className="field-label">Progressive rate bands</span>
                  <div className="bracket-list">
                    {component.brackets.map((bracket, bracketIndex) => <div className="bracket-row" key={`${bracket.lowerAtomic}-${bracketIndex}`}>
                      <label><span>Lower amount</span><input inputMode="numeric" value={bracket.lowerAtomic} onChange={(event) => onChange(updateBracket(draft, componentIndex, bracketIndex, { lowerAtomic: event.target.value }))} /></label>
                      <label><span>Upper amount</span><input inputMode="numeric" value={bracket.upperAtomic} disabled={bracket.openEnded} onChange={(event) => onChange(updateBracket(draft, componentIndex, bracketIndex, { upperAtomic: event.target.value }))} placeholder={bracket.openEnded ? "Open-ended" : "Required"} /></label>
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
                <label><span>Tax credit</span><input inputMode="numeric" value={component.creditAtomic} onChange={(event) => onChange(updateComponent(draft, componentIndex, { creditAtomic: event.target.value }))} /></label>
                <label><span>Minimum tax (optional)</span><input inputMode="numeric" value={component.floorAtomic} onChange={(event) => onChange(updateComponent(draft, componentIndex, { floorAtomic: event.target.value }))} placeholder="No minimum" /></label>
                <label><span>Maximum tax (optional)</span><input inputMode="numeric" value={component.capAtomic} onChange={(event) => onChange(updateComponent(draft, componentIndex, { capAtomic: event.target.value }))} placeholder="No maximum" /></label>
                <label><span>How amounts are rounded</span><select value={component.roundingMode} onChange={(event) => onChange(updateComponent(draft, componentIndex, { roundingMode: event.target.value as RuleComponentDraft["roundingMode"] }))}><option value="Floor">Down / truncate</option><option value="Ceil">Up</option><option value="HalfUp">Nearest, ties up</option><option value="HalfEven">Nearest, ties to even</option><option value="AwayFromZero">Away from zero</option></select></label>
                <label><span>When rounding happens</span><select value={component.roundingScope} onChange={(event) => onChange(updateComponent(draft, componentIndex, { roundingScope: event.target.value as RuleComponentDraft["roundingScope"] }))}><option value="Component">For each tax</option><option value="Settlement">For the full payment</option><option value="Line">For each invoice line</option><option value="Invoice">For the full invoice</option></select></label>
                <label><span>Smallest tax unit</span><input inputMode="numeric" value={component.roundingQuantumAtomic} onChange={(event) => onChange(updateComponent(draft, componentIndex, { roundingQuantumAtomic: event.target.value }))} /></label>
                <small className="field-help full-field">Settlement rounds across one aggregate. Line rounds within each line. Invoice rounds across the complete document. These scopes are conserving and deterministic, but tax-on-tax dependencies must use each-component rounding.</small>
                <small className="field-help full-field">Use a positive atomic quantum to make decimal-scale truncation explicit; 1 preserves one-atomic-unit precision.</small>
                <div className="full-field workbench-threshold">
                  <label className="checkbox-control"><span>Use a threshold</span><input type="checkbox" checked={component.thresholdEnabled} onChange={(event) => onChange(updateComponent(draft, componentIndex, { thresholdEnabled: event.target.checked }))} /></label>
                  {component.thresholdEnabled && <div className="form-grid">
                    <label><span>Threshold amount</span><input inputMode="numeric" value={component.thresholdAtomic} onChange={(event) => onChange(updateComponent(draft, componentIndex, { thresholdAtomic: event.target.value }))} /></label>
                    <label><span>Threshold applies per</span><select value={component.thresholdBasis} onChange={(event) => onChange(updateComponent(draft, componentIndex, { thresholdBasis: event.target.value as RuleComponentDraft["thresholdBasis"] }))}><option value="PerTransaction">Each transaction</option><option value="CumulativePeriod">A running period total</option></select></label>
                    <label><span>What happens at the threshold</span><select value={component.thresholdMode} onChange={(event) => onChange(updateComponent(draft, componentIndex, { thresholdMode: event.target.value as RuleComponentDraft["thresholdMode"] }))}><option value="ExemptUpTo">Exempt up to this amount</option><option value="TriggerAt">Apply after reaching this amount</option></select></label>
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
        <summary>View the exact rule document</summary>
        {generated ? <pre>{generated}</pre> : <p className="inline-warning">The document preview is unavailable until the structured fields are valid.</p>}
      </details>
    </section>
  );
}
