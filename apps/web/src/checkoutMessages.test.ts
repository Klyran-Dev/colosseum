import { describe, expect, it } from "vitest";
import { SUPPORTED_LOCALES } from "./formatters";
import { checkoutMessagesFor } from "./checkoutMessages";

describe("aggregate checkout message catalogs", () => {
  it("provide complete primary-flow copy for every supported locale", () => {
    for (const locale of SUPPORTED_LOCALES) {
      const messages = checkoutMessagesFor(locale);
      expect(messages.review.title).not.toBe("");
      expect(messages.route.paymentRoute).not.toBe("");
      expect(messages.split.sharing).not.toBe("");
      expect(messages.safeguards.title).not.toBe("");
      expect(messages.status.walletBoundary).not.toBe("");
      expect(messages.errors.serviceUnavailable).not.toBe("");
      expect(messages.errors.quoteExpired).not.toBe("");
      expect(messages.errors.submissionUnknown).not.toBe("");
    }
  });

  it("falls back deterministically and leaves protocol vocabulary untouched", () => {
    expect(checkoutMessagesFor("xx-XX").review.title).toBe(checkoutMessagesFor("en-US").review.title);
    expect(JSON.stringify(checkoutMessagesFor("de-DE"))).not.toMatch(/Tokenkeg|0199|1000/);
  });

  it("bounds dynamic recovery values before rendering them", () => {
    const messages = checkoutMessagesFor("en-US");
    expect(messages.errors.routeRequestFailed(502)).toContain("502");
    expect(messages.errors.routeRequestFailed(100000)).not.toContain("100000");
    expect(messages.errors.intentRecovery("pending")).toContain("pending");
    expect(messages.errors.intentRecovery("<provider-detail>")).not.toContain("provider-detail");
  });
});
