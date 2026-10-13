import { describe, expect, it } from "vitest";
import { shellOperationalMessagesFor } from "./shellOperationalMessages";

describe("shell operational presentation catalogs", () => {
  it("provides localized status and protection copy for every supported locale", () => {
    const english = shellOperationalMessagesFor("en-US");
    for (const locale of ["en-US", "de-DE", "ar-EG", "ja-JP"]) {
      const messages = shellOperationalMessagesFor(locale);
      expect(messages.system.label).not.toHaveLength(0);
      expect(messages.account.openMenu).not.toHaveLength(0);
      expect(messages.authorization.unavailable).not.toHaveLength(0);
      expect(messages.authorization.expired).not.toHaveLength(0);
      expect(messages.authorization.missingExpiry).not.toHaveLength(0);
      expect(messages.protection.label).not.toHaveLength(0);
      expect(messages.footer.traceable).not.toHaveLength(0);
    }
    expect(shellOperationalMessagesFor("xx-XX").system.label).toBe(english.system.label);
    expect(shellOperationalMessagesFor("de-DE").account.openMenu).toBe("Kontomenü öffnen");
    expect(shellOperationalMessagesFor("de-DE").authorization.expired).toBe("Ihre Zugriffsprüfung ist nicht mehr aktuell. Der Zugriff wird aktualisiert, bevor Sie fortfahren.");
    expect(shellOperationalMessagesFor("ar-EG").protection.label).toBe("تفاصيل الحماية");
    expect(shellOperationalMessagesFor("de-DE").error.reload).toBe("Anwendung neu laden");
    expect(shellOperationalMessagesFor("ja-JP").loading.title("税ルール")).toBe("税ルールを読み込み中…");
    expect(shellOperationalMessagesFor("en-US").account.roleLabel("tax_rule_author")).toBe("Tax rule author");
  });
});
