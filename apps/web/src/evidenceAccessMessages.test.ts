import { describe, expect, it } from "vitest";
import { evidenceAccessMessagesFor } from "./evidenceAccessMessages";
import { SUPPORTED_LOCALES } from "./formatters";

describe("evidence-access presentation catalogs", () => {
  it("provides bounded HTTP recovery text for every supported locale", () => {
    for (const locale of SUPPORTED_LOCALES) {
      const messages = evidenceAccessMessagesFor(locale);
      expect(messages.common.httpError(messages.common.error, 502)).toContain("502");
      expect(messages.common.httpError(messages.common.error, 99)).toContain("unknown status");
      expect(messages.common.httpError(messages.common.error, 600)).toContain("unknown status");
    }
  });

  it("covers every supported locale with complete primary and authority copy", () => {
    for (const locale of SUPPORTED_LOCALES) {
      const messages = evidenceAccessMessagesFor(locale);
      expect(messages.control.title.length).toBeGreaterThan(0);
      expect(messages.control.load.length).toBeGreaterThan(0);
      expect(messages.ledger.authorityTitle.length).toBeGreaterThan(0);
      expect(messages.ledger.authorityCopy.length).toBeGreaterThan(0);
      expect(messages.common.error.length).toBeGreaterThan(0);
    }
  });

  it("falls back deterministically for an unknown locale", () => {
    expect(evidenceAccessMessagesFor("xx-XX").control.title).toBe("Review evidence access");
  });
});
