import { describe, expect, it } from "vitest";
import { SUPPORTED_LOCALES } from "./formatters";
import { settlementAdjustmentMessagesFor } from "./settlementAdjustmentMessages";

describe("settlement adjustment localization catalog", () => {
  it("has bounded correction and wallet evidence language for every locale", () => {
    for (const locale of SUPPORTED_LOCALES) {
      const messages = settlementAdjustmentMessagesFor(locale);
      expect(messages.proposal.title).toBeTruthy();
      expect(messages.review.title).toBeTruthy();
      expect(messages.chain.prepare).toBeTruthy();
      expect(messages.status.providerAmbiguous).toBeTruthy();
      expect(messages.errors.bytesJsonInvalid).toBeTruthy();
      expect(messages.errors.evidenceJsonInvalid).toBeTruthy();
      expect(messages.errors.evidenceObjectRequired).toBeTruthy();
      expect(messages.errors.reasonCodeInvalid).toBeTruthy();
      expect(messages.errors.idempotencyKeyRequired).toBeTruthy();
      expect(messages.errors.httpError(messages.errors.proposal, 422)).toContain("422");
    }
  });

  it("falls back deterministically to English", () => {
    expect(settlementAdjustmentMessagesFor("pt-BR").proposal.title).toBe("Create an adjustment request");
    expect(settlementAdjustmentMessagesFor("pt-BR").status.providerAmbiguous).toContain("Provider outcome");
  });

  it("bounds untrusted HTTP statuses before they reach operator copy", () => {
    const messages = settlementAdjustmentMessagesFor("en-US");
    expect(messages.errors.httpError(messages.errors.proposal, 700)).toContain("unknown status");
    expect(messages.errors.httpError(messages.errors.proposal, -1)).toContain("unknown status");
    expect(messages.errors.httpError(messages.errors.proposal, 503)).toContain("503");
  });
});
