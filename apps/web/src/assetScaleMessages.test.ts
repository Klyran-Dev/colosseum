import { describe, expect, it } from "vitest";
import { SUPPORTED_LOCALES, SupportedLocale } from "./formatters";
import { assetScaleMessagesFor, parseAssetScale } from "./assetScaleMessages";

describe("versioned asset-precision language", () => {
  it("provides complete copy for every supported locale", () => {
    for (const locale of SUPPORTED_LOCALES) {
      const messages = assetScaleMessagesFor(locale);
      expect(messages.label.length).toBeGreaterThan(0);
      expect(messages.help.length).toBeGreaterThan(0);
      expect(messages.invalid.length).toBeGreaterThan(0);
      expect(messages.decimalPlaces(2).length).toBeGreaterThan(0);
    }
  });

  it("uses the canonical English catalog for unknown locales", () => {
    expect(assetScaleMessagesFor("xx-XX" as SupportedLocale)).toEqual(assetScaleMessagesFor("en-US"));
  });

  it("rejects blank, fractional, signed, and out-of-range precision at the browser boundary", () => {
    for (const value of ["", "1.0", "-1", "10", " "]) {
      expect(() => parseAssetScale(value, "invalid precision")).toThrow("invalid precision");
    }
    expect(parseAssetScale("0", "invalid precision")).toBe(0);
    expect(parseAssetScale("9", "invalid precision")).toBe(9);
  });
});
