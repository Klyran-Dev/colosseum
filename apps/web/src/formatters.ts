export const DEFAULT_LOCALE = "en-US";
export const DEFAULT_TIME_ZONE = "UTC";
export const SUPPORTED_LOCALES = ["en-US", "de-DE", "ar-EG", "ja-JP"] as const;
export type SupportedLocale = (typeof SUPPORTED_LOCALES)[number];

const RTL_LANGUAGES = new Set(["ar", "fa", "he", "ur"]);

function decimalSeparator(locale: string): string {
  return new Intl.NumberFormat(locale, { minimumFractionDigits: 1, maximumFractionDigits: 1 })
    .formatToParts(1.1)
    .find((part) => part.type === "decimal")?.value ?? ".";
}

function boundedAtomicDecimals(decimals: number): number {
  if (!Number.isInteger(decimals) || decimals < 0 || decimals > 9) {
    throw new Error("atomic decimals must be an integer between 0 and 9");
  }
  return decimals;
}

function currencyPlacement(locale: string): "prefix" | "suffix" {
  const parts = new Intl.NumberFormat(locale, {
    style: "currency",
    currency: "USD",
    currencyDisplay: "code",
  }).formatToParts(0);
  const currencyIndex = parts.findIndex((part) => part.type === "currency");
  const integerIndex = parts.findIndex((part) => part.type === "integer");
  return currencyIndex >= 0 && integerIndex >= 0 && currencyIndex < integerIndex ? "prefix" : "suffix";
}

function currencyJoiner(locale: string): string {
  const parts = new Intl.NumberFormat(locale, {
    style: "currency",
    currency: "USD",
    currencyDisplay: "code",
  }).formatToParts(0);
  const currencyIndex = parts.findIndex((part) => part.type === "currency");
  const integerIndex = parts.findIndex((part) => part.type === "integer");
  if (currencyIndex < 0 || integerIndex < 0) return " ";
  const start = Math.min(currencyIndex, integerIndex);
  const end = Math.max(currencyIndex, integerIndex);
  const literal = parts.slice(start + 1, end).find((part) => part.type === "literal")?.value;
  return literal ?? " ";
}

/** Formats protocol atomic units without converting the amount through Number. */
export function formatAtomic(
  value: string,
  currency: string,
  locale = DEFAULT_LOCALE,
  atomicDecimals: number,
): string {
  let atomic: bigint;
  try {
    atomic = BigInt(value);
  } catch {
    throw new Error("atomic amount is not an integer");
  }
  if (!currency.trim()) throw new Error("currency is required");
  if (atomicDecimals === undefined) throw new Error("atomic display scale is required");
  const decimals = boundedAtomicDecimals(atomicDecimals);

  const negative = atomic < 0n;
  const absolute = negative ? -atomic : atomic;
  const scale = 10n ** BigInt(decimals);
  const whole = decimals === 0 ? absolute : absolute / scale;
  const fraction = decimals === 0 ? 0n : absolute % scale;
  const integerText = new Intl.NumberFormat(locale, { useGrouping: true, maximumFractionDigits: 0 }).format(whole);
  const numericText = decimals === 0
    ? integerText
    : `${integerText}${decimalSeparator(locale)}${new Intl.NumberFormat(locale, {
      useGrouping: false,
      minimumIntegerDigits: decimals,
      maximumFractionDigits: 0,
    }).format(fraction)}`;
  const joiner = currencyJoiner(locale);
  const display = currencyPlacement(locale) === "prefix"
    ? `${currency}${joiner}${numericText}`
    : `${numericText}${joiner}${currency}`;
  return negative ? `-${display}` : display;
}

export function formatInteger(value: string, locale = DEFAULT_LOCALE): string {
  let integer: bigint;
  try {
    integer = BigInt(value);
  } catch {
    throw new Error("integer value is invalid");
  }
  return new Intl.NumberFormat(locale, { useGrouping: true, maximumFractionDigits: 0 }).format(integer);
}

export function isValidTimeZone(timeZone: string): boolean {
  if (!timeZone.trim()) return false;
  try {
    new Intl.DateTimeFormat(DEFAULT_LOCALE, { timeZone }).format();
    return true;
  } catch {
    return false;
  }
}

export function formatTimestamp(value: string, locale = DEFAULT_LOCALE, timeZone = DEFAULT_TIME_ZONE): string {
  const timestamp = Date.parse(value);
  if (Number.isNaN(timestamp) || !isValidTimeZone(timeZone)) return value;
  return new Intl.DateTimeFormat(locale, {
    dateStyle: "medium",
    timeStyle: "short",
    timeZone,
  }).format(timestamp);
}

export function formatEpoch(value: number, locale = DEFAULT_LOCALE, timeZone = DEFAULT_TIME_ZONE): string {
  if (!Number.isFinite(value) || !isValidTimeZone(timeZone)) return String(value);
  return new Intl.DateTimeFormat(locale, {
    dateStyle: "medium",
    timeStyle: "short",
    timeZone,
  }).format(new Date(value * 1_000));
}

export function directionForLocale(locale: string): "ltr" | "rtl" {
  const language = locale.toLowerCase().split("-")[0];
  return RTL_LANGUAGES.has(language) ? "rtl" : "ltr";
}
