import { SupportedLocale } from "./formatters";

export interface AssetScaleMessages {
  label: string;
  help: string;
  invalid: string;
  decimalPlaces: (value: number) => string;
}

const ENGLISH: AssetScaleMessages = {
  label: "Asset precision",
  help: "Enter how many decimal places this asset uses (0–9). The service uses this value for exact amounts.",
  invalid: "Asset precision must be a whole number from 0 to 9.",
  decimalPlaces: (value) => `${value} decimal places`,
};

const CATALOG: Record<SupportedLocale, AssetScaleMessages> = {
  "en-US": ENGLISH,
  "de-DE": {
    label: "Asset-Präzision",
    help: "Geben Sie an, wie viele Dezimalstellen dieses Asset verwendet (0–9). Der Dienst nutzt diesen Wert für exakte Beträge.",
    invalid: "Die Asset-Präzision muss eine ganze Zahl von 0 bis 9 sein.",
    decimalPlaces: (value) => `${value} Dezimalstellen`,
  },
  "ar-EG": {
    label: "دقة الأصل",
    help: "أدخل عدد المنازل العشرية التي يستخدمها هذا الأصل (من 0 إلى 9). يستخدم النظام هذه القيمة للمبالغ الدقيقة.",
    invalid: "يجب أن تكون دقة الأصل عدداً صحيحاً من 0 إلى 9.",
    decimalPlaces: (value) => `${value} منازل عشرية`,
  },
  "ja-JP": {
    label: "アセットの精度",
    help: "このアセットが使用する小数点以下の桁数（0～9）を入力します。サービスはこの値で金額を正確に扱います。",
    invalid: "アセットの精度は0～9の整数で指定してください。",
    decimalPlaces: (value) => `${value}桁`,
  },
};

export function assetScaleMessagesFor(locale: string): AssetScaleMessages {
  return locale in CATALOG ? CATALOG[locale as SupportedLocale] : ENGLISH;
}

/** Parse the operator-owned precision without allowing an empty input to become zero. */
export function parseAssetScale(value: string, invalidMessage = ENGLISH.invalid): number {
  if (!/^[0-9]$/.test(value)) throw new Error(invalidMessage);
  return Number(value);
}
