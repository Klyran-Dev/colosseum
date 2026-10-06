import { describe, expect, it } from "vitest";
import { SUPPORTED_LOCALES, SupportedLocale } from "./formatters";
import { quoteMessagesFor } from "./quoteMessages";

describe("versioned quote workspace localization", () => {
  it("provides primary-flow copy for every supported locale", () => {
    for (const locale of SUPPORTED_LOCALES) {
      const messages = quoteMessagesFor(locale);
      expect(messages.quoteMode.title.length).toBeGreaterThan(0);
      expect(messages.previewMode.title.length).toBeGreaterThan(0);
      expect(messages.fields.grossLabel.length).toBeGreaterThan(0);
      expect(messages.fields.currencyPlaceholder.length).toBeGreaterThan(0);
      expect(messages.location.description.length).toBeGreaterThan(0);
      expect(messages.result.emptyTitle.length).toBeGreaterThan(0);
      expect(messages.result.whyTax.length).toBeGreaterThan(0);
      expect(messages.amountHelp.length).toBeGreaterThan(0);
      expect(messages.quoteMode.submit.length).toBeGreaterThan(0);
    }
  });

  it("uses the canonical English catalog for unknown locales", () => {
    const english = quoteMessagesFor("en-US");
    const unknown = quoteMessagesFor("xx-XX" as SupportedLocale);
    expect(unknown.quoteMode).toEqual(english.quoteMode);
    expect(unknown.fields).toEqual(english.fields);
    expect(unknown.result.emptyTitle).toBe(english.result.emptyTitle);
  });

  it("does not translate protocol identifiers or financial values", () => {
    for (const locale of SUPPORTED_LOCALES) {
      const messages = quoteMessagesFor(locale);
      expect(messages.result.calculationReference).not.toContain("UUIDv7");
      expect(messages.amountHelp).not.toContain("USD");
      expect(messages.fields.approvedRulePlaceholder).not.toContain("019");
    }
  });
});
