import { describe, expect, it } from "vitest";
import { SUPPORTED_LOCALES } from "./formatters";
import { refundCancellationMessagesFor } from "./refundCancellationMessages";

describe("refund cancellation localization catalog", () => {
  it("provides bounded HTTP recovery text for every supported locale", () => {
    for (const locale of SUPPORTED_LOCALES) {
      const messages = refundCancellationMessagesFor(locale);
      expect(messages.errors.httpError(messages.errors.prepare, 409)).toContain("409");
      expect(messages.errors.httpError(messages.errors.prepare, 99)).toContain("unknown status");
      expect(messages.errors.httpError(messages.errors.prepare, 600)).toContain("unknown status");
    }
  });

  it("has a bounded operator vocabulary for every supported locale", () => {
    for (const locale of SUPPORTED_LOCALES) {
      const messages = refundCancellationMessagesFor(locale);
      expect(messages.proposal.title).toBeTruthy();
      expect(messages.review.title).toBeTruthy();
      expect(messages.wallet.sign).toBeTruthy();
      expect(messages.evidence.reconcile).toBeTruthy();
      expect(messages.evidence.providerNotProof).toBeTruthy();
    }
  });

  it("falls back deterministically without changing protocol values", () => {
    expect(refundCancellationMessagesFor("pt-BR").proposal.title).toBe("Cancel a refund request");
    expect(refundCancellationMessagesFor("pt-BR").evidence.providerNotProof).toContain("provider response");
  });
});
