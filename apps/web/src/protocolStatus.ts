import { settlementAdjustmentMessagesFor } from "./settlementAdjustmentMessages";

/**
 * Converts a validated protocol lifecycle value into operator-facing copy.
 * Backend values remain locale-neutral; unknown values never leak into UI.
 */
export function formatProtocolStatus(status: string, locale: string): string {
  const labels = settlementAdjustmentMessagesFor(locale).status.labels;
  return labels[status] ?? labels.unknown;
}
