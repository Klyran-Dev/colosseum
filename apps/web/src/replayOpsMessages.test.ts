import { describe, expect, it } from "vitest";
import { replayOpsMessagesFor } from "./replayOpsMessages";

describe("replay operations message catalogs", () => {
  it("provides bounded operator copy for every supported locale", () => {
    for (const locale of ["en-US", "de-DE", "ar-EG", "ja-JP"]) {
      const messages = replayOpsMessagesFor(locale);
      expect(messages.control.title).toBeTruthy();
      expect(messages.route.title).toBeTruthy();
      expect(messages.targeted.title).toBeTruthy();
      expect(messages.queue.title).toBeTruthy();
      expect(messages.status.labels.dead_lettered).toBeTruthy();
      expect(messages.actions.acknowledge).toBeTruthy();
      expect(messages.actions.requeue).toBeTruthy();
      expect(messages.common.malformedRouteEvidence).toBeTruthy();
      expect(messages.common.httpError(messages.common.error, 422)).toContain("422");
      expect(messages.common.pagesOutOfRange(8)).toContain("8");
    }
  });

  it("falls back deterministically to English for unknown locales", () => {
    expect(replayOpsMessagesFor("xx-XX").control.title).toBe(replayOpsMessagesFor("en-US").control.title);
    expect(replayOpsMessagesFor("xx-XX").common.unavailable).toBe(replayOpsMessagesFor("en-US").common.unavailable);
  });

  it("bounds untrusted transport status before it reaches operator copy", () => {
    const messages = replayOpsMessagesFor("en-US");
    expect(messages.common.httpError(messages.common.error, 700)).toContain("unknown status");
  });
});
