import { describe, expect, it } from "vitest";
import { ruleWorkbenchMessagesFor } from "./ruleWorkbenchMessages";

describe("rule workbench localization catalog", () => {
  it("provides complete authoring guidance for every supported locale", () => {
    for (const locale of ["en-US", "de-DE", "ar-EG", "ja-JP"]) {
      const messages = ruleWorkbenchMessagesFor(locale);
      expect(messages.heading.title).toBeTruthy();
      expect(messages.component.internalName).toBeTruthy();
      expect(messages.rate.progressive).toBeTruthy();
      expect(messages.component.quantum).toBeTruthy();
      expect(messages.threshold.title).toBeTruthy();
      expect(messages.actions.addComponent).toBeTruthy();
    }
  });

  it("keeps serialized enum values canonical", () => {
    const messages = ruleWorkbenchMessagesFor("ar-EG");
    expect(messages.enumValues.priceMode).toEqual({ Exclusive: "Exclusive", Inclusive: "Inclusive" });
    expect(messages.enumValues.roundingScope.Component).toBe("Component");
  });
});
