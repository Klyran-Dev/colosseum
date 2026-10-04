// @vitest-environment happy-dom

import { afterEach, describe, expect, it } from "vitest";
import {
  DISPLAY_PREFERENCES_STORAGE_KEY,
  DEFAULT_DISPLAY_PREFERENCES,
  loadDisplayPreferences,
  saveDisplayPreferences,
} from "./displayPreferences";

afterEach(() => {
  window.localStorage.clear();
});

describe("versioned local display preferences", () => {
  it("uses safe defaults when no preference has been stored", () => {
    expect(loadDisplayPreferences(window.localStorage)).toEqual(DEFAULT_DISPLAY_PREFERENCES);
  });

  it("round-trips only the supported presentation fields", () => {
    saveDisplayPreferences(window.localStorage, {
      locale: "ar-EG",
      timeZone: "Asia/Kolkata",
      theme: "light",
    });

    expect(loadDisplayPreferences(window.localStorage)).toEqual({
      locale: "ar-EG",
      timeZone: "Asia/Kolkata",
      theme: "light",
    });
    expect(JSON.parse(window.localStorage.getItem(DISPLAY_PREFERENCES_STORAGE_KEY) ?? "{}"))
      .toEqual({ version: 1, locale: "ar-EG", timeZone: "Asia/Kolkata", theme: "light" });
  });

  it("rejects malformed, unknown, and oversized persisted values", () => {
    window.localStorage.setItem(DISPLAY_PREFERENCES_STORAGE_KEY, JSON.stringify({
      version: 1,
      locale: "xx-XX",
      timeZone: "Not/AZone",
      theme: "neon",
    }));
    expect(loadDisplayPreferences(window.localStorage)).toEqual(DEFAULT_DISPLAY_PREFERENCES);

    window.localStorage.setItem(DISPLAY_PREFERENCES_STORAGE_KEY, "{" );
    expect(loadDisplayPreferences(window.localStorage)).toEqual(DEFAULT_DISPLAY_PREFERENCES);

    window.localStorage.setItem(DISPLAY_PREFERENCES_STORAGE_KEY, "x".repeat(4097));
    expect(loadDisplayPreferences(window.localStorage)).toEqual(DEFAULT_DISPLAY_PREFERENCES);
  });

  it("does not write an invalid preference object", () => {
    saveDisplayPreferences(window.localStorage, {
      locale: "xx-XX" as "en-US",
      timeZone: "Not/AZone",
      theme: "neon" as "dark",
    });

    expect(window.localStorage.getItem(DISPLAY_PREFERENCES_STORAGE_KEY)).toBeNull();
  });

  it("fails open when browser storage throws and never persists sensitive state", () => {
    const throwingStorage = {
      getItem: () => { throw new Error("storage unavailable"); },
      setItem: () => { throw new Error("storage unavailable"); },
    } as unknown as Storage;

    expect(loadDisplayPreferences(throwingStorage)).toEqual(DEFAULT_DISPLAY_PREFERENCES);
    expect(() => saveDisplayPreferences(throwingStorage, {
      locale: "en-US",
      timeZone: "UTC",
      theme: "dark",
    })).not.toThrow();
    expect(JSON.stringify(DEFAULT_DISPLAY_PREFERENCES)).not.toContain("token");
  });
});
