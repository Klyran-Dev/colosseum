import { describe, expect, it } from "vitest";
import { SUPPORTED_LOCALES } from "./formatters";
import { refundClaimMessagesFor } from "./refundClaimMessages";

describe("refund claim localization catalog", () => {
  it("provides the complete bounded catalog and deterministic fallback", () => {
    for (const locale of SUPPORTED_LOCALES) {
      const messages = refundClaimMessagesFor(locale);
      expect(messages.proposal.title).toBeTruthy();
      expect(messages.review.title).toBeTruthy();
      expect(messages.wallet.signProposer).toBeTruthy();
      expect(messages.evidence.reconcile).toBeTruthy();
      expect(messages.errors.authorizationRequired).toBeTruthy();
      expect(messages.errors.preparationRequestFailed(422)).toContain("422");
      expect(messages.errors.preparationReady).toBeTruthy();
      expect(messages.errors.reconciliationReady).toBeTruthy();
    }
    expect(refundClaimMessagesFor("xx-XX").proposal.title).toBe(refundClaimMessagesFor("en-US").proposal.title);
  });

  it("bounds transport status before it reaches operator copy", () => {
    const messages = refundClaimMessagesFor("en-US");
    expect(messages.errors.recoveryRequestFailed(700)).toContain("unknown status");
  });

  it("does not put protocol or financial values in localized copy", () => {
    const messages = refundClaimMessagesFor("de-DE");
    const renderedCopy = JSON.stringify(messages);
    expect(renderedCopy).not.toMatch(/01992f1b|Tokenkeg|250000|111111111111/);
  });
});
