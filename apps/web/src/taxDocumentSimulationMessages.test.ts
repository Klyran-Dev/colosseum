import { describe, expect, it } from "vitest";
import { taxDocumentSimulationMessagesFor } from "./taxDocumentSimulationMessages";

describe("document simulation presentation catalogs", () => {
  it("covers every supported locale without translating protocol values", () => {
    const english = taxDocumentSimulationMessagesFor("en-US");
    for (const locale of ["en-US", "de-DE", "ar-EG", "ja-JP"]) {
      const messages = taxDocumentSimulationMessagesFor(locale);
      if (locale !== "en-US") expect(messages.panel.summary).not.toBe(english.panel.summary);
      expect(messages.result.calculationReference).toBeDefined();
      expect(messages.form.line(2)).toBeTruthy();
    }
    expect(taxDocumentSimulationMessagesFor("ar-EG").form.addLine).toBe("إضافة سطر");
    expect(taxDocumentSimulationMessagesFor("ja-JP").result.unattestedTax).toBe("未認証の文書税");
  });

  it("falls back deterministically for an unknown locale", () => {
    expect(taxDocumentSimulationMessagesFor("xx-XX").panel.summary).toBe("Line and invoice tax simulation");
  });
});
