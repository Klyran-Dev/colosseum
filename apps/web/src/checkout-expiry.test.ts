import { describe, expect, it } from "vitest";
import { settlementQuoteExpired } from "./checkout";

describe("settlement quote expiry boundary", () => {
  it("does not invent expiry when the server omits it", () => {
    expect(settlementQuoteExpired(undefined, 1_000)).toBe(false);
  });

  it("keeps a quote valid before its expiry second", () => {
    expect(settlementQuoteExpired(1_001, 1_000)).toBe(false);
  });

  it("expires at the exact server-declared second", () => {
    expect(settlementQuoteExpired(1_000, 1_000)).toBe(true);
  });

  it("remains expired after the server-declared second", () => {
    expect(settlementQuoteExpired(999, 1_000)).toBe(true);
  });
});

