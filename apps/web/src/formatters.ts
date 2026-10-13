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

function dateTimePart(parts: Intl.DateTimeFormatPart[], type: Intl.DateTimeFormatPartTypes): string {
  return parts.find((part) => part.type === type)?.value ?? "";
}

function timeZoneOffsetMilliseconds(value: Date, timeZone: string): number {
  const parts = new Intl.DateTimeFormat("en-CA", {
    timeZone,
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hourCycle: "h23",
  }).formatToParts(value);
  const wallClock = Date.UTC(
    Number(dateTimePart(parts, "year")),
    Number(dateTimePart(parts, "month")) - 1,
    Number(dateTimePart(parts, "day")),
    Number(dateTimePart(parts, "hour")),
    Number(dateTimePart(parts, "minute")),
    Number(dateTimePart(parts, "second")),
  );
  return wallClock - value.getTime();
}

/** Formats an epoch second as the exact value accepted by a datetime-local input in timeZone. */
export function formatDateTimeLocal(value: number, timeZone = DEFAULT_TIME_ZONE): string {
  if (!Number.isSafeInteger(value) || !isValidTimeZone(timeZone)) return "";
  const parts = new Intl.DateTimeFormat("en-CA", {
    timeZone,
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hourCycle: "h23",
  }).formatToParts(new Date(value * 1_000));
  return `${dateTimePart(parts, "year")}-${dateTimePart(parts, "month")}-${dateTimePart(parts, "day")}T${dateTimePart(parts, "hour")}:${dateTimePart(parts, "minute")}:${dateTimePart(parts, "second")}`;
}

/** Parses a datetime-local value in timeZone without converting through the browser's local zone. */
export function parseDateTimeLocal(value: string, timeZone = DEFAULT_TIME_ZONE): number | null {
  if (!isValidTimeZone(timeZone)) return null;
  const match = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2})(?::(\d{2}))?$/.exec(value.trim());
  if (!match) return null;
  const year = Number(match[1]);
  const month = Number(match[2]);
  const day = Number(match[3]);
  const hour = Number(match[4]);
  const minute = Number(match[5]);
  const second = Number(match[6] ?? "0");
  if (month < 1 || month > 12 || day < 1 || day > 31 || hour > 23 || minute > 59 || second > 59) return null;

  const wallDate = new Date(0);
  wallDate.setUTCFullYear(year, month - 1, day);
  wallDate.setUTCHours(hour, minute, second, 0);
  if (wallDate.getUTCFullYear() !== year || wallDate.getUTCMonth() !== month - 1 || wallDate.getUTCDate() !== day) return null;

  const wallClock = wallDate.getTime();
  let candidate = wallClock;
  for (let attempt = 0; attempt < 4; attempt += 1) {
    const next = wallClock - timeZoneOffsetMilliseconds(new Date(candidate), timeZone);
    if (next === candidate) break;
    candidate = next;
  }
  const epochSeconds = candidate / 1_000;
  if (!Number.isSafeInteger(epochSeconds)) return null;
  const normalized = formatDateTimeLocal(epochSeconds, timeZone);
  const requested = `${match[1]}-${match[2]}-${match[3]}T${match[4]}:${match[5]}:${match[6] ?? "00"}`;
  return normalized === requested ? epochSeconds : null;
}

export function directionForLocale(locale: string): "ltr" | "rtl" {
  const language = locale.toLowerCase().split("-")[0];
  return RTL_LANGUAGES.has(language) ? "rtl" : "ltr";
}
