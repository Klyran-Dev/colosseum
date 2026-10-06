import { taxTreatmentMessagesFor } from "./taxTreatmentMessages";
import type { TreatmentOutcome } from "./taxResponses";

type Props = {
  outcome?: TreatmentOutcome;
  currency: string;
  atomicDecimals: number;
  locale: string;
  formatAtomic: (value: string, currency: string, atomicDecimals: number) => string;
};

export default function TaxTreatmentOutcome({ outcome, currency, atomicDecimals, locale, formatAtomic }: Props) {
  if (!outcome) return null;
  const messages = taxTreatmentMessagesFor(locale);
  const amount = (value: string) => formatAtomic(value, currency, atomicDecimals);
  return (
    <section className="treatment-outcome" aria-label={messages.ariaLabel}>
      <div className="treatment-outcome-heading">
        <div>
          <p className="eyebrow">{messages.title}</p>
          <strong>{messages.treatment}: {messages.names[outcome.treatment]}</strong>
        </div>
        <span className="status-pill">{messages.names[outcome.treatment]}</span>
      </div>
      <dl className="treatment-channels">
        <div><dt>{messages.collected}</dt><dd>{amount(outcome.collected_tax_atomic)}</dd></div>
        <div><dt>{messages.selfAssessed}</dt><dd>{amount(outcome.self_assessed_tax_atomic)}</dd></div>
        <div><dt>{messages.withheld}</dt><dd>{amount(outcome.withheld_tax_atomic)}</dd></div>
      </dl>
      <details className="technical-details">
        <summary>{messages.arithmetic}</summary>
        <dl className="summary-list">
          <div><dt>{messages.arithmetic}</dt><dd>{amount(outcome.arithmetic_tax_atomic)}</dd></div>
          <div><dt>{messages.taxableBase}</dt><dd>{amount(outcome.taxable_base_atomic)}</dd></div>
        </dl>
      </details>
      <p className="evidence-note">{messages.limitation}</p>
    </section>
  );
}
