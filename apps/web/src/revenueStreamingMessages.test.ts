import { describe, expect, it } from "vitest";
import { revenueStreamingMessagesFor } from "./revenueStreamingMessages";

describe("revenue-streaming localization catalog", () => {
  it("provides complete catalogs for every supported operator locale", () => {
    for (const locale of ["en-US", "de-DE", "ar-EG", "ja-JP"]) {
      const messages = revenueStreamingMessagesFor(locale);
      expect(messages.shell.title).toBeTruthy();
      expect(messages.status.proposed).toBeTruthy();
      expect(messages.actions.approve).toBeTruthy();
      expect(messages.setup.destination).toBeTruthy();
      expect(messages.setup.maxTotal).toBeTruthy();
      expect(messages.boundary.disclaimer).toBeTruthy();
      expect(messages.details.unavailable).toBeTruthy();
      expect(messages.details.destinationReference).toBeTruthy();
      expect(messages.details.splitReference).toBeTruthy();
      expect(messages.details.policyReference).toBeTruthy();
      expect(messages.actions.upTo("100")).toContain("100");
      expect(messages.errors.httpError(messages.errors.plansLoad, 503)).toContain("503");
    }
  });

  it("keeps machine status and capability values separate from labels", () => {
    const messages = revenueStreamingMessagesFor("de-DE");
    expect(messages.status.proposed).not.toBe("proposed");
    expect(messages.enumValues.proposed).toBe("proposed");
    expect(messages.enumValues.completed).toBe("completed");
    expect(messages.actionLabel("proposed", "approved")).toContain("→");
  });

  it("bounds untrusted HTTP status values in recovery copy", () => {
    const messages = revenueStreamingMessagesFor("en-US");
    expect(messages.errors.httpError(messages.errors.save, 700)).toContain("unknown status");
    expect(messages.errors.httpError(messages.errors.save, -1)).toContain("unknown status");
  });
});
