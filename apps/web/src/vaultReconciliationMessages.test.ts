import { describe, expect, it } from "vitest";
import { vaultReconciliationMessagesFor } from "./vaultReconciliationMessages";

describe("vault reconciliation message catalogs", () => {
  it("provides bounded balance-control copy for every supported locale", () => {
    for (const locale of ["en-US", "de-DE", "ar-EG", "ja-JP"]) {
      const messages = vaultReconciliationMessagesFor(locale);
      expect(messages.control.title).toBeTruthy();
      expect(messages.targets.title).toBeTruthy();
      expect(messages.detail.title).toBeTruthy();
      expect(messages.status.matched).toBeTruthy();
      expect(messages.common.httpError(messages.common.lookupError, 503)).toContain("503");
    }
  });

  it("falls back deterministically to English for unknown locales", () => {
    expect(vaultReconciliationMessagesFor("xx-XX").control.title).toBe(vaultReconciliationMessagesFor("en-US").control.title);
    expect(vaultReconciliationMessagesFor("xx-XX").common.unavailable).toBe(vaultReconciliationMessagesFor("en-US").common.unavailable);
  });

  it("bounds untrusted HTTP status values in recovery copy", () => {
    const messages = vaultReconciliationMessagesFor("en-US");
    expect(messages.common.httpError(messages.common.historyError, 700)).toContain("unknown status");
    expect(messages.common.httpError(messages.common.historyError, -1)).toContain("unknown status");
  });
});
