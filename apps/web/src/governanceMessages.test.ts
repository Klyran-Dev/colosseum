import { describe, expect, it } from "vitest";
import { governanceMessagesFor } from "./governanceMessages";
import { SUPPORTED_LOCALES } from "./formatters";

describe("governance localization catalog", () => {
  it("provides bounded HTTP recovery text for every supported locale", () => {
    for (const locale of SUPPORTED_LOCALES) {
      const messages = governanceMessagesFor(locale);
      expect(messages.common.httpError(messages.errors.load, 503)).toContain("503");
      expect(messages.common.httpError(messages.errors.load, 99)).toContain("unknown status");
      expect(messages.common.httpError(messages.errors.load, 600)).toContain("unknown status");
    }
  });

  it("provides complete primary governance copy for every supported locale", () => {
    for (const locale of ["en-US", "de-DE", "ar-EG", "ja-JP"]) {
      const messages = governanceMessagesFor(locale);
      expect(messages.section.title).toBeTruthy();
      expect(messages.draft.summary).toBeTruthy();
      expect(messages.evidence.legend).toBeTruthy();
      expect(messages.transition.reason).toBeTruthy();
      expect(messages.simulation.run).toBeTruthy();
      expect(messages.history.decisionTrail).toBeTruthy();
    }
  });

  it("keeps protocol values separate from presentation labels", () => {
    const messages = governanceMessagesFor("de-DE");
    expect(messages.editor.modeValues).toEqual({ structured: "structured", json: "json" });
    expect(messages.statusLabels.draft).not.toBe("draft");
    expect(messages.transitionLabel("draft", "simulated")).toBeTruthy();
  });
});
