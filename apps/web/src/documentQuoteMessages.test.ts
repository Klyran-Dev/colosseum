import { describe, expect, it } from "vitest";
import { SUPPORTED_LOCALES } from "./formatters";
import { documentQuoteMessagesFor } from "./documentQuoteMessages";

describe("document quote message catalog", () => {
  it("provides complete primary-flow copy for every supported locale", () => {
    for (const locale of SUPPORTED_LOCALES) {
      const messages = documentQuoteMessagesFor(locale);
      expect(messages.form.title).not.toBe("");
      expect(messages.form.fields.documentId).not.toBe("");
      expect(messages.form.lines.add).not.toBe("");
      expect(messages.result.emptyTitle).not.toBe("");
      expect(messages.result.openCheckout).not.toBe("");
      expect(messages.errors.serviceUnavailable).not.toBe("");
      expect(messages.errors.quoteFailed).not.toBe("");
      expect(messages.errors.invalidFields).not.toBe("");
    }
  });

  it("falls back deterministically and keeps protocol values out of catalog copy", () => {
    expect(documentQuoteMessagesFor("xx-XX").form.title).toBe(documentQuoteMessagesFor("en-US").form.title);
    const serialized = JSON.stringify(documentQuoteMessagesFor("en-US"));
    expect(serialized).not.toContain("USD");
    expect(serialized).not.toContain("0199");
    expect(serialized).not.toContain("rule-1");
  });

  it("keeps dynamic error values typed and bounded", () => {
    const messages = documentQuoteMessagesFor("en-US");
    expect(messages.errors.quoteRequestFailed(503)).toContain("503");
    expect(messages.errors.quoteRequestFailed(99999)).not.toContain("99999");
    expect(messages.errors.quoteRequestFailed(-1)).not.toContain("-1");
  });
});
