import {
  DEFAULT_LOCALE,
  DEFAULT_TIME_ZONE,
  isValidTimeZone,
  SUPPORTED_LOCALES,
  SupportedLocale,
} from "./formatters";

export type DisplayTheme = "system" | "light" | "dark";

export interface DisplayPreferences {
  locale: SupportedLocale;
  timeZone: string;
  theme: DisplayTheme;
}

export const DISPLAY_PREFERENCES_STORAGE_KEY = "solana-tax-protocol.display-preferences";
export const DEFAULT_DISPLAY_PREFERENCES: DisplayPreferences = {
  locale: DEFAULT_LOCALE,
  timeZone: DEFAULT_TIME_ZONE,
  theme: "system",
};

const DISPLAY_PREFERENCES_VERSION = 1;
const MAX_SERIALIZED_PREFERENCES_BYTES = 4096;

function browserStorage(): Storage | undefined {
  try {
    return typeof window === "undefined" ? undefined : window.localStorage;
  } catch {
    return undefined;
  }
}

function isSupportedLocale(value: unknown): value is SupportedLocale {
  return typeof value === "string" && (SUPPORTED_LOCALES as readonly string[]).includes(value);
}

function isDisplayTheme(value: unknown): value is DisplayTheme {
  return value === "system" || value === "light" || value === "dark";
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function parsePreferences(value: unknown): DisplayPreferences | null {
  if (!isRecord(value) || value.version !== DISPLAY_PREFERENCES_VERSION) return null;
  if (!isSupportedLocale(value.locale) || typeof value.timeZone !== "string" || !isValidTimeZone(value.timeZone)) return null;
  if (!isDisplayTheme(value.theme)) return null;
  return { locale: value.locale, timeZone: value.timeZone, theme: value.theme };
}

export function loadDisplayPreferences(storage: Storage | undefined = browserStorage()): DisplayPreferences {
  if (!storage) return DEFAULT_DISPLAY_PREFERENCES;
  try {
    const serialized = storage.getItem(DISPLAY_PREFERENCES_STORAGE_KEY);
    if (!serialized || new TextEncoder().encode(serialized).byteLength > MAX_SERIALIZED_PREFERENCES_BYTES) {
      return DEFAULT_DISPLAY_PREFERENCES;
    }
    return parsePreferences(JSON.parse(serialized)) ?? DEFAULT_DISPLAY_PREFERENCES;
  } catch {
    return DEFAULT_DISPLAY_PREFERENCES;
  }
}

export function saveDisplayPreferences(
  storage: Storage | undefined,
  preferences: DisplayPreferences,
): void {
  if (!storage) return;
  try {
    const candidate = {
      version: DISPLAY_PREFERENCES_VERSION,
      locale: preferences.locale,
      timeZone: preferences.timeZone,
      theme: preferences.theme,
    };
    if (!parsePreferences(candidate)) return;
    const serialized = JSON.stringify(candidate);
    if (serialized.length > MAX_SERIALIZED_PREFERENCES_BYTES) return;
    storage.setItem(DISPLAY_PREFERENCES_STORAGE_KEY, serialized);
  } catch {
    // Browser persistence is an enhancement; never block the product shell.
  }
}
