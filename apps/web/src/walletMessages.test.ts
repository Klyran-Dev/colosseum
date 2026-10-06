import { describe, expect, it } from "vitest";
import { SUPPORTED_LOCALES } from "./formatters";
import { walletMessagesFor } from "./walletMessages";

describe("wallet message catalogs", () => {
  it("provide complete user-facing wallet controls for every supported locale", () => {
    for (const locale of SUPPORTED_LOCALES) {
      const messages = walletMessagesFor(locale);
      expect(messages.noWallet).not.toBe("");
      expect(messages.providerLabel).not.toBe("");
      expect(messages.accountLabel).not.toBe("");
      expect(messages.connect).not.toBe("");
      expect(messages.connectAfterArtifact).not.toBe("");
    }
  });

  it("falls back deterministically and never translates wallet protocol values", () => {
    expect(walletMessagesFor("xx-XX").connect).toBe(walletMessagesFor("en-US").connect);
    expect(JSON.stringify(walletMessagesFor("de-DE"))).not.toMatch(/Tokenkeg|0199|1000/);
  });
});
