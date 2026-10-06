import { describe, expect, it } from "vitest";
import { SUPPORTED_LOCALES } from "./formatters";
import { refundExecuteMessagesFor } from "./refundExecuteMessages";

describe("refund execution localization catalog", () => {
  it("provides required keys and deterministic fallback", () => {
    for (const locale of SUPPORTED_LOCALES) {
      const messages = refundExecuteMessagesFor(locale);
      expect(messages.proposal.title).toBeTruthy();
      expect(messages.review.title).toBeTruthy();
      expect(messages.wallet.signApprover).toBeTruthy();
      expect(messages.status.proofPending).toBeTruthy();
      expect(messages.errors.authorizationRequired).toBeTruthy();
      expect(messages.errors.preparationRequestFailed(422)).toContain("422");
      expect(messages.errors.preparationReady).toBeTruthy();
      expect(messages.errors.submissionUnknown).toBeTruthy();
    }
    expect(refundExecuteMessagesFor("xx-XX").proposal.title).toBe(refundExecuteMessagesFor("en-US").proposal.title);
  });

  it("bounds transport status before it reaches operator copy", () => {
    const messages = refundExecuteMessagesFor("en-US");
    expect(messages.errors.recoveryRequestFailed(700)).toContain("unknown status");
  });
});
