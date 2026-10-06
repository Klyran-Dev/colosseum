import { SupportedLocale } from "./formatters";
import type { TaxTreatment } from "./taxResponses";

export type TaxTreatmentMessages = {
  ariaLabel: string;
  title: string;
  treatment: string;
  collected: string;
  selfAssessed: string;
  withheld: string;
  taxableBase: string;
  arithmetic: string;
  limitation: string;
  names: Record<TaxTreatment, string>;
};

const ENGLISH: TaxTreatmentMessages = {
  ariaLabel: "Tax treatment outcome",
  title: "How this amount is treated",
  treatment: "Treatment",
  collected: "Collected from the payer",
  selfAssessed: "Self-assessed by the recipient",
  withheld: "Withheld from the seller",
  taxableBase: "Taxable base",
  arithmetic: "Calculated amount before treatment",
  limitation: "This classification supports calculation and settlement review. It is not a filing, legal conclusion, or tax advice.",
  names: {
    Taxable: "Taxable",
    ZeroRated: "Zero-rated",
    Exempt: "Exempt",
    ReverseCharge: "Reverse charge",
    Withholding: "Withholding",
    OutOfScope: "Outside scope",
  },
};

const GERMAN: TaxTreatmentMessages = {
  ...ENGLISH,
  ariaLabel: "Ergebnis der steuerlichen Behandlung",
  title: "Wie dieser Betrag behandelt wird",
  treatment: "Behandlung",
  collected: "Vom Zahler erhoben",
  selfAssessed: "Vom Empfänger selbst berechnet",
  withheld: "Vom Verkäufer einbehalten",
  taxableBase: "Steuerbemessungsgrundlage",
  arithmetic: "Berechneter Betrag vor Behandlung",
  limitation: "Diese Einstufung unterstützt die Berechnungs- und Abwicklungsprüfung. Sie ist keine Erklärung, rechtliche Feststellung oder Steuerberatung.",
  names: { Taxable: "Steuerpflichtig", ZeroRated: "Nullsatz", Exempt: "Steuerbefreit", ReverseCharge: "Umkehr der Steuerschuld", Withholding: "Einbehalt", OutOfScope: "Außerhalb des Anwendungsbereichs" },
};

const ARABIC: TaxTreatmentMessages = {
  ...ENGLISH,
  ariaLabel: "نتيجة المعاملة الضريبية",
  title: "كيفية معاملة هذا المبلغ",
  treatment: "المعاملة",
  collected: "المحصّل من الدافع",
  selfAssessed: "المحتسب ذاتياً من المستلم",
  withheld: "المقتطع من البائع",
  taxableBase: "الوعاء الخاضع للضريبة",
  arithmetic: "المبلغ المحسوب قبل المعاملة",
  limitation: "تدعم هذه الفئة مراجعة الحساب والتسوية. وهي ليست إقراراً أو استنتاجاً قانونياً أو نصيحة ضريبية.",
  names: { Taxable: "خاضع للضريبة", ZeroRated: "بنسبة صفرية", Exempt: "معفى", ReverseCharge: "الاحتساب العكسي", Withholding: "اقتطاع", OutOfScope: "خارج النطاق" },
};

const JAPANESE: TaxTreatmentMessages = {
  ...ENGLISH,
  ariaLabel: "税務処理の結果",
  title: "この金額の処理方法",
  treatment: "処理",
  collected: "支払者から徴収",
  selfAssessed: "受取人による自己申告計算",
  withheld: "販売者から源泉徴収",
  taxableBase: "課税標準",
  arithmetic: "処理前の計算額",
  limitation: "この分類は計算と決済の確認を支援します。申告、法的判断、税務助言ではありません。",
  names: { Taxable: "課税", ZeroRated: "ゼロ税率", Exempt: "免税", ReverseCharge: "リバースチャージ", Withholding: "源泉徴収", OutOfScope: "対象外" },
};

const CATALOG: Record<SupportedLocale, TaxTreatmentMessages> = {
  "en-US": ENGLISH,
  "de-DE": GERMAN,
  "ar-EG": ARABIC,
  "ja-JP": JAPANESE,
};

export function taxTreatmentMessagesFor(locale: string): TaxTreatmentMessages {
  return CATALOG[locale as SupportedLocale] ?? ENGLISH;
}
