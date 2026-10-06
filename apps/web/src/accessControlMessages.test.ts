import { describe, expect, it } from "vitest";
import { accessControlMessagesFor } from "./accessControlMessages";

describe("access-control presentation catalogs", () => {
  it("covers every supported locale with the same policy identity set", () => {
    const english = accessControlMessagesFor("en-US");
    for (const locale of ["en-US", "de-DE", "ar-EG", "ja-JP"]) {
      const messages = accessControlMessagesFor(locale);
      expect(Object.keys(messages.policies).sort()).toEqual(Object.keys(english.policies).sort());
      expect(messages.overview.technicalCopy).toContain("RBAC");
    }
    expect(accessControlMessagesFor("ar-EG").policies["tax.quote.create"].label).toBe("إنشاء عرض ضريبي");
    expect(accessControlMessagesFor("ja-JP").policies["tax.quote.create"].label).toBe("税額見積を作成");
  });

  it("falls back deterministically for an unknown locale", () => {
    expect(accessControlMessagesFor("xx-XX").overview.title).toBe("Who can do what");
  });
});
