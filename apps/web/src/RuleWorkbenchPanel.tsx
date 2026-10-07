import { useMemo } from "react";
import {
  buildRuleSet,
  defaultComponentDraft,
  RuleComponentDraft,
  RuleWorkbenchDraft,
  validateRuleWorkbench,
} from "./ruleWorkbench";
import { DEFAULT_LOCALE } from "./formatters";
import { ruleWorkbenchMessagesFor } from "./ruleWorkbenchMessages";

type RuleWorkbenchProps = {
  draft: RuleWorkbenchDraft;
  onChange: (draft: RuleWorkbenchDraft) => void;
  locale?: string;
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

export default function RuleWorkbench({ draft, onChange, locale = DEFAULT_LOCALE }: RuleWorkbenchProps) {
  const messages = ruleWorkbenchMessagesFor(locale);
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
          <p className="eyebrow">{messages.heading.eyebrow}</p>
          <h3 id="rule-workbench-heading">{messages.heading.title}</h3>
        </div>
        <span className="status-pill">{messages.heading.badge}</span>
      </div>
      <p className="evidence-note">{messages.heading.copy}</p>

      <div className="workbench-components">
        {draft.components.map((component, componentIndex) => {
          const choices = componentChoices(draft, component.id);
          return (
            <fieldset className="workbench-component" key={`${component.id}-${componentIndex}`}>
              <legend>{messages.component.label(componentIndex + 1)}</legend>
              <div className="form-grid">
                <label><span>{messages.component.internalName}</span><input value={component.id} onChange={(event) => onChange(updateComponent(draft, componentIndex, { id: event.target.value }))} autoComplete="off" /></label>
                <label><span>{messages.component.customerLabel}</span><input value={component.label} onChange={(event) => onChange(updateComponent(draft, componentIndex, { label: event.target.value }))} /></label>
                <div className="full-field">
                  <span className="field-label">{messages.component.taxedAmount}</span>
                  <div className="workbench-source-list">
                    {component.baseSources.map((source, sourceIndex) => (
                      <div className="workbench-inline-row" key={`${source}-${sourceIndex}`}>
                        <label className="inline-control"><span className="sr-only">{messages.component.baseSource(sourceIndex + 1)}</span><select value={source} onChange={(event) => onChange(updateComponent(draft, componentIndex, { baseSources: component.baseSources.map((item, index) => index === sourceIndex ? event.target.value : item) }))}>{choices.map((choice) => <option key={choice} value={choice}>{choice}</option>)}</select></label>
                        <div className="base-share-fraction">
                          <label className="inline-control"><span>{messages.component.baseNumerator}</span><input inputMode="numeric" value={component.baseNumerators?.[sourceIndex] ?? "1"} onChange={(event) => onChange(updateComponent(draft, componentIndex, { baseNumerators: component.baseSources.map((_, index) => index === sourceIndex ? event.target.value : component.baseNumerators?.[index] ?? "1") }))} /></label>
                          <span className="base-share-divider" aria-hidden="true">/</span>
                          <label className="inline-control"><span>{messages.component.baseDenominator}</span><input inputMode="numeric" value={component.baseDenominators?.[sourceIndex] ?? "1"} onChange={(event) => onChange(updateComponent(draft, componentIndex, { baseDenominators: component.baseSources.map((_, index) => index === sourceIndex ? event.target.value : component.baseDenominators?.[index] ?? "1") }))} /></label>
                        </div>
                        <button type="button" className="secondary-button" onClick={() => onChange(updateComponent(draft, componentIndex, { baseSources: component.baseSources.filter((_, index) => index !== sourceIndex), baseNumerators: (component.baseNumerators ?? component.baseSources.map(() => "1")).filter((_, index) => index !== sourceIndex), baseDenominators: (component.baseDenominators ?? component.baseSources.map(() => "1")).filter((_, index) => index !== sourceIndex) }))} disabled={component.baseSources.length <= 1}>{messages.component.removeSource}</button>
                      </div>
                    ))}
                  </div>
                  <button type="button" className="secondary-button" onClick={() => onChange(updateComponent(draft, componentIndex, { baseSources: [...component.baseSources, "Net"], baseNumerators: [...(component.baseNumerators ?? component.baseSources.map(() => "1")), "1"], baseDenominators: [...(component.baseDenominators ?? component.baseSources.map(() => "1")), "1"] }))} disabled={component.baseSources.length >= 128}>{messages.component.addSource}</button>
                  <small className="field-help">{messages.component.sourceHelp}</small>
                </div>
                <label><span>{messages.component.baseMinimum}</span><input inputMode="numeric" value={component.baseFloorAtomic} onChange={(event) => onChange(updateComponent(draft, componentIndex, { baseFloorAtomic: event.target.value }))} placeholder={messages.component.exemptionPlaceholder} /></label>
                <label><span>{messages.component.baseMaximum}</span><input inputMode="numeric" value={component.baseCapAtomic} onChange={(event) => onChange(updateComponent(draft, componentIndex, { baseCapAtomic: event.target.value }))} placeholder={messages.component.exemptionPlaceholder} /></label>
                <small className="field-help full-field">{messages.component.baseBoundsHelp}</small>
                <label><span>{messages.component.rateChanges}</span><select value={component.rateMode} onChange={(event) => onChange(updateComponent(draft, componentIndex, { rateMode: event.target.value as RuleComponentDraft["rateMode"] }))}><option value="flat">{messages.rate.flat}</option><option value="progressive">{messages.rate.progressive}</option></select></label>
                <label><span>{messages.component.priceMode}</span><select value={component.priceMode} onChange={(event) => { const priceMode = event.target.value as RuleComponentDraft["priceMode"]; onChange(updateComponent(draft, componentIndex, { priceMode, inclusiveThresholdEquation: priceMode === "Inclusive" ? component.inclusiveThresholdEquation : null })); }}><option value="Exclusive">{messages.enumLabels.priceMode.Exclusive}</option><option value="Inclusive">{messages.enumLabels.priceMode.Inclusive}</option></select></label>
                {component.rateMode === "flat" ? <>
                  <label><span>{messages.component.rateValue}</span><input inputMode="numeric" value={component.flatNumerator} onChange={(event) => onChange(updateComponent(draft, componentIndex, { flatNumerator: event.target.value }))} /></label>
                  <label><span>{messages.component.rateScale}</span><input inputMode="numeric" value={component.flatDenominator} onChange={(event) => onChange(updateComponent(draft, componentIndex, { flatDenominator: event.target.value }))} /></label>
                </> : <div className="full-field">
                  <span className="field-label">{messages.rate.bands}</span>
                  <div className="bracket-list">
                    {component.brackets.map((bracket, bracketIndex) => <div className="bracket-row" key={`${bracket.lowerAtomic}-${bracketIndex}`}>
                      <label><span>{messages.rate.lower}</span><input inputMode="numeric" value={bracket.lowerAtomic} onChange={(event) => onChange(updateBracket(draft, componentIndex, bracketIndex, { lowerAtomic: event.target.value }))} /></label>
                      <label><span>{messages.rate.upper}</span><input inputMode="numeric" value={bracket.upperAtomic} disabled={bracket.openEnded} onChange={(event) => onChange(updateBracket(draft, componentIndex, bracketIndex, { upperAtomic: event.target.value }))} placeholder={bracket.openEnded ? messages.rate.openEndedPlaceholder : messages.rate.requiredPlaceholder} /></label>
                      <label><span>{messages.rate.numerator}</span><input inputMode="numeric" value={bracket.numerator} onChange={(event) => onChange(updateBracket(draft, componentIndex, bracketIndex, { numerator: event.target.value }))} /></label>
                      <label><span>{messages.rate.denominator}</span><input inputMode="numeric" value={bracket.denominator} onChange={(event) => onChange(updateBracket(draft, componentIndex, bracketIndex, { denominator: event.target.value }))} /></label>
                      <label className="checkbox-control"><span>{messages.rate.openEnded}</span><input type="checkbox" checked={bracket.openEnded} onChange={(event) => onChange(updateBracket(draft, componentIndex, bracketIndex, { openEnded: event.target.checked }))} /></label>
                      <button type="button" className="secondary-button" onClick={() => onChange(updateComponent(draft, componentIndex, { brackets: component.brackets.filter((_, index) => index !== bracketIndex) }))} disabled={component.brackets.length <= 1}>{messages.rate.removeBracket}</button>
                    </div>)}
                  </div>
                  <button type="button" className="secondary-button" onClick={() => {
                    const previous = component.brackets[component.brackets.length - 1];
                    onChange(updateComponent(draft, componentIndex, { brackets: [...component.brackets, { lowerAtomic: previous?.openEnded ? "0" : previous?.upperAtomic ?? "0", upperAtomic: "", numerator: "10", denominator: "100", openEnded: true }] }));
                  }} disabled={component.brackets.length >= 128}>{messages.rate.addBracket}</button>
                  <small className="field-help">{messages.rate.bracketHelp}</small>
                </div>}
                <label><span>{messages.component.credit}</span><input inputMode="numeric" value={component.creditAtomic} onChange={(event) => onChange(updateComponent(draft, componentIndex, { creditAtomic: event.target.value }))} /></label>
                <label><span>{messages.component.minimum}</span><input inputMode="numeric" value={component.floorAtomic} onChange={(event) => onChange(updateComponent(draft, componentIndex, { floorAtomic: event.target.value }))} placeholder={messages.component.exemptionPlaceholder} /></label>
                <label><span>{messages.component.maximum}</span><input inputMode="numeric" value={component.capAtomic} onChange={(event) => onChange(updateComponent(draft, componentIndex, { capAtomic: event.target.value }))} placeholder={messages.component.exemptionPlaceholder} /></label>
                <label><span>{messages.component.roundingMode}</span><select value={component.roundingMode} onChange={(event) => onChange(updateComponent(draft, componentIndex, { roundingMode: event.target.value as RuleComponentDraft["roundingMode"] }))}><option value="Floor">{messages.enumLabels.roundingMode.Floor}</option><option value="Ceil">{messages.enumLabels.roundingMode.Ceil}</option><option value="HalfUp">{messages.enumLabels.roundingMode.HalfUp}</option><option value="HalfEven">{messages.enumLabels.roundingMode.HalfEven}</option><option value="AwayFromZero">{messages.enumLabels.roundingMode.AwayFromZero}</option></select></label>
                <label><span>{messages.component.roundingScope}</span><select value={component.roundingScope} onChange={(event) => onChange(updateComponent(draft, componentIndex, { roundingScope: event.target.value as RuleComponentDraft["roundingScope"] }))}><option value="Component">{messages.enumLabels.roundingScope.Component}</option><option value="Settlement">{messages.enumLabels.roundingScope.Settlement}</option><option value="Line">{messages.enumLabels.roundingScope.Line}</option><option value="Invoice">{messages.enumLabels.roundingScope.Invoice}</option></select></label>
                <label><span>{messages.component.quantum}</span><input inputMode="numeric" value={component.roundingQuantumAtomic} onChange={(event) => onChange(updateComponent(draft, componentIndex, { roundingQuantumAtomic: event.target.value }))} /></label>
                <small className="field-help full-field">{messages.component.scopeHelp}</small>
                <small className="field-help full-field">{messages.component.quantumHelp}</small>
                <div className="full-field workbench-threshold">
                  <label className="checkbox-control"><span>{messages.threshold.title}</span><input type="checkbox" checked={component.thresholdEnabled} onChange={(event) => onChange(updateComponent(draft, componentIndex, { thresholdEnabled: event.target.checked }))} /></label>
                  {component.thresholdEnabled && <div className="form-grid">
                    <label><span>{messages.threshold.amount}</span><input inputMode="numeric" value={component.thresholdAtomic} onChange={(event) => onChange(updateComponent(draft, componentIndex, { thresholdAtomic: event.target.value }))} /></label>
                    <label><span>{messages.threshold.basis}</span><select value={component.thresholdBasis} onChange={(event) => onChange(updateComponent(draft, componentIndex, { thresholdBasis: event.target.value as RuleComponentDraft["thresholdBasis"] }))}><option value="PerTransaction">{messages.enumLabels.thresholdBasis.PerTransaction}</option><option value="CumulativePeriod">{messages.enumLabels.thresholdBasis.CumulativePeriod}</option></select></label>
                    <label><span>{messages.threshold.mode}</span><select value={component.thresholdMode} onChange={(event) => onChange(updateComponent(draft, componentIndex, { thresholdMode: event.target.value as RuleComponentDraft["thresholdMode"] }))}><option value="ExemptUpTo">{messages.enumLabels.thresholdMode.ExemptUpTo}</option><option value="TriggerAt">{messages.enumLabels.thresholdMode.TriggerAt}</option></select></label>
                    {component.priceMode === "Inclusive" && <label><span>{messages.threshold.equation}</span><select value={component.inclusiveThresholdEquation ?? ""} onChange={(event) => onChange(updateComponent(draft, componentIndex, { inclusiveThresholdEquation: event.target.value === "" ? null : event.target.value as RuleComponentDraft["inclusiveThresholdEquation"] }))}><option value="">{messages.threshold.chooseEquation}</option><option value="GrossExcess">{messages.threshold.grossExcess}</option></select></label>}
                  </div>}
                  <small className="field-help">{messages.threshold.help}</small>
                </div>
                <label className="full-field"><span>{messages.component.exemptions}</span><textarea value={component.exemptionCodes} onChange={(event) => onChange(updateComponent(draft, componentIndex, { exemptionCodes: event.target.value }))} rows={2} /></label>
              </div>
              <button type="button" className="secondary-button" onClick={() => removeComponent(componentIndex)} disabled={draft.components.length <= 1}>{messages.actions.removeComponent}</button>
            </fieldset>
          );
        })}
      </div>
      <button type="button" className="secondary-button" onClick={addComponent} disabled={draft.components.length >= 128}>{messages.actions.addComponent}</button>

      <div className="workbench-validation" aria-live="polite">
        {errors.length === 0 ? <p className="inline-success">{messages.actions.ready}</p> : <div className="inline-warning"><strong>{messages.actions.resolve}</strong><ul>{errors.map((error) => <li key={error}>{error}</li>)}</ul></div>}
      </div>
      <details className="workbench-preview">
        <summary>{messages.actions.preview}</summary>
        {generated ? <pre>{generated}</pre> : <p className="inline-warning">{messages.actions.previewUnavailable}</p>}
      </details>
    </section>
  );
}
