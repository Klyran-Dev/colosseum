import { describe, expect, it } from "vitest";
import {
  directionForLocale,
  formatAtomic,
  formatEpoch,
  formatTimestamp,
  isValidTimeZone,
} from "./formatters";

describe("locale-aware financial and evidence formatting", () => {
  it("preserves large atomic values without Number conversion", () => {
    expect(formatAtomic("900719925474099312345", "USD", "en-US")).toBe("USD\u00a09,007,199,254,740,993,123.45");
  });

  it("formats zero, six, and nine decimal Solana assets from BigInt atomic units", () => {
    expect(formatAtomic("1234", "TOKEN", "en-US", 0)).toBe("TOKEN\u00a01,234");
    expect(formatAtomic("123456", "USDC", "en-US", 6)).toBe("USDC\u00a00.123456");
    expect(formatAtomic("1", "SOL", "en-US", 9)).toBe("SOL\u00a00.000000001");
  });

  it("rejects an unbounded or fractional asset precision", () => {
    expect(() => formatAtomic("1", "TOKEN", "en-US", -1)).toThrow(/between 0 and 9/);
    expect(() => formatAtomic("1", "TOKEN", "en-US", 10)).toThrow(/between 0 and 9/);
    expect(() => formatAtomic("1", "TOKEN", "en-US", 2.5)).toThrow(/between 0 and 9/);
  });

  it("uses locale grouping and decimal conventions", () => {
    expect(formatAtomic("123456", "EUR", "de-DE")).toBe("1.234,56 EUR");
  });

  it("formats the same timestamp in the requested time zone", () => {
    const value = "2026-01-02T03:04:05.000Z";
    expect(formatTimestamp(value, "en-US", "UTC")).toContain("3:04");
    expect(formatTimestamp(value, "en-US", "Asia/Kolkata")).toContain("8:34");
    expect(formatEpoch(0, "en-US", "UTC")).toContain("12:00");
  });

  it("fails safely for invalid dates and time zones", () => {
    expect(isValidTimeZone("Not/AZone")).toBe(false);
    expect(formatTimestamp("not-a-date", "en-US", "UTC")).toBe("not-a-date");
    expect(formatEpoch(Number.NaN, "en-US", "UTC")).toBe("NaN");
  });

  it("derives RTL only for the bounded RTL language set", () => {
    expect(directionForLocale("ar-EG")).toBe("rtl");
    expect(directionForLocale("he-IL")).toBe("rtl");
    expect(directionForLocale("de-DE")).toBe("ltr");
  });
});
