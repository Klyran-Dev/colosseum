import { describe, expect, it } from "vitest";
import { SUPPORTED_LOCALES } from "./formatters";
import { documentCheckoutMessagesFor } from "./documentCheckoutMessages";

describe("document checkout message catalog", () => {
  it("provides complete wallet-review copy for every supported locale", () => {
    for (const locale of SUPPORTED_LOCALES) {
      const messages = documentCheckoutMessagesFor(locale);
      expect(messages.evidence.title).not.toBe("");
      expect(messages.review.title).not.toBe("");
      expect(messages.review.prepare).not.toBe("");
      expect(messages.status.walletBoundary).not.toBe("");
      expect(messages.status.legalBoundary).not.toBe("");
      expect(messages.errors.serviceUnavailable).not.toBe("");
      expect(messages.errors.compileRequestFailed(422)).toContain("422");
      expect(messages.errors.intentRecovery("pending")).not.toContain("undefined");
    }
  });

  it("bounds operator-visible status and intent values", () => {
    const messages = documentCheckoutMessagesFor("en-US");
    expect(messages.errors.routeRequestFailed(700)).toContain("unknown status");
    expect(messages.errors.intentRecovery("<script>alert(1)</script>")).toContain("unknown");
    expect(messages.errors.intentRecovery("finalized")).toContain("finalized");
  });

  it("falls back deterministically and excludes protocol values from copy", () => {
    expect(documentCheckoutMessagesFor("xx-XX").review.title).toBe(documentCheckoutMessagesFor("en-US").review.title);
    const serialized = JSON.stringify(documentCheckoutMessagesFor("en-US"));
    expect(serialized).not.toContain("Tokenkeg");
    expect(serialized).not.toContain("0199");
    expect(serialized).not.toContain("1000");
  });
});
