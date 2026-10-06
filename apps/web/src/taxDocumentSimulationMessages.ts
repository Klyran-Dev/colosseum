import { DEFAULT_LOCALE } from "./formatters";

export interface TaxDocumentSimulationMessages {
  panel: { summary: string; evidenceNote: string };
  form: {
    documentId: string;
    currency: string;
    jurisdiction: (value: string) => string;
    notSelected: string;
    linesHeading: string;
    linesDescription: string;
    addLine: string;
    line: (number: number) => string;
    lineId: string;
    gross: string;
    net: string;
    exemptions: string;
    exemptionsPlaceholder: string;
    remove: string;
    calculating: string;
    submit: string;
  };
  state: {
    unavailable: string;
    invalidRule: string;
    noLocalTotal: string;
    resultRegion: string;
    notice: string;
    failed: (status?: number) => string;
  };
  result: {
    unattestedTax: string;
    gross: string;
    netAfterTax: string;
    factsDigest: string;
    fxEvidence: string;
    canonicalTrace: string;
    tax: string;
    calculationReference: string;
  };
}

const ENGLISH: TaxDocumentSimulationMessages = {
  panel: {
    summary: "Line and invoice tax simulation",
    evidenceNote: "This is an explicit document-facts workbench. The Rust service validates bounds, canonicalizes line identity, applies the rule's Line or Invoice scope, and returns an unattested trace. No browser arithmetic or settlement authority is used.",
  },
  form: {
    documentId: "Document ID",
    currency: "Currency",
    jurisdiction: (value) => `Jurisdiction follows the loaded rule: ${value || "not selected"}.`,
    notSelected: "not selected",
    linesHeading: "Tax lines",
    linesDescription: "Canonical results are sorted by line ID. Maximum 256 lines.",
    addLine: "Add line",
    line: (number) => `Line ${number}`,
    lineId: "Line ID",
    gross: "Amount before tax",
    net: "Amount after tax",
    exemptions: "Exemption codes",
    exemptionsPlaceholder: "One per line or comma-separated",
    remove: "Remove",
    calculating: "Calculating bounded document trace…",
    submit: "Simulate document tax",
  },
  state: {
    unavailable: "The API is unavailable; no local tax result is substituted.",
    invalidRule: "The current rule document is not valid. Fix the rule workbench before simulating a document.",
    noLocalTotal: "The service is unavailable. This workbench does not display a locally invented total.",
    resultRegion: "Document simulation result",
    notice: "The server returned a deterministic, unattested document trace. It cannot create a quote or authorize settlement.",
    failed: (status) => status === undefined ? "Document simulation failed" : `Document simulation failed (${status})`,
  },
  result: {
    unattestedTax: "Unattested document tax",
    gross: "Gross",
    netAfterTax: "Net after tax",
    factsDigest: "Facts digest",
    fxEvidence: "FX evidence",
    canonicalTrace: "Canonical line trace",
    tax: "tax",
    calculationReference: "calculation",
  },
};

const GERMAN: TaxDocumentSimulationMessages = {
  panel: { summary: "Steuersimulation für Zeilen und Rechnung", evidenceNote: "Dies ist eine Arbeitsfläche für dokumentierte Fakten. Der Rust-Dienst prüft Grenzen, kanonisiert die Zeilenidentität, wendet den Zeilen- oder Rechnungsumfang der Regel an und liefert einen nicht attestierten Nachweis. Es gibt keine Berechnung im Browser und keine Abwicklungsberechtigung." },
  form: { documentId: "Dokument-ID", currency: "Währung", jurisdiction: (value) => `Die Jurisdiktion folgt der geladenen Regel: ${value || "nicht ausgewählt"}.`, notSelected: "nicht ausgewählt", linesHeading: "Steuerzeilen", linesDescription: "Kanonische Ergebnisse werden nach Zeilen-ID sortiert. Maximal 256 Zeilen.", addLine: "Zeile hinzufügen", line: (number) => `Zeile ${number}`, lineId: "Zeilen-ID", gross: "Betrag vor Steuer", net: "Betrag nach Steuer", exemptions: "Befreiungscodes", exemptionsPlaceholder: "Eine pro Zeile oder durch Komma getrennt", remove: "Entfernen", calculating: "Begrenzten Dokumentnachweis berechnen…", submit: "Dokumentsteuer simulieren" },
  state: { unavailable: "Die API ist nicht verfügbar; es wird kein lokales Steuerergebnis ersetzt.", invalidRule: "Das aktuelle Regeldokument ist ungültig. Korrigieren Sie die Regelarbeitsfläche vor der Dokumentensimulation.", noLocalTotal: "Der Dienst ist nicht verfügbar. Diese Arbeitsfläche zeigt keine erfundene lokale Summe an.", resultRegion: "Ergebnis der Dokumentensimulation", notice: "Der Dienst hat einen deterministischen, nicht attestierten Dokumentnachweis zurückgegeben. Er kann kein Angebot erstellen oder eine Abwicklung autorisieren.", failed: (status) => status === undefined ? "Dokumentensimulation fehlgeschlagen" : `Dokumentensimulation fehlgeschlagen (${status})` },
  result: { unattestedTax: "Nicht attestierte Dokumentsteuer", gross: "Brutto", netAfterTax: "Netto nach Steuer", factsDigest: "Fakten-Digest", fxEvidence: "FX-Nachweis", canonicalTrace: "Kanonischer Zeilennachweis", tax: "Steuer", calculationReference: "Berechnung" },
};

const ARABIC: TaxDocumentSimulationMessages = {
  panel: { summary: "محاكاة ضريبة السطور والفاتورة", evidenceNote: "هذه مساحة عمل لحقائق المستند. تتحقق خدمة Rust من الحدود، وتوحّد هوية السطور، وتطبق نطاق السطر أو الفاتورة في القاعدة، وتعيد دليلاً غير موثّق. لا توجد حسابات ضريبية في المتصفح ولا صلاحية للتسوية." },
  form: { documentId: "معرّف المستند", currency: "العملة", jurisdiction: (value) => `تتبع الولاية القاعدة المحمّلة: ${value || "غير محددة"}.`, notSelected: "غير محددة", linesHeading: "سطور الضريبة", linesDescription: "تُرتب النتائج المعيارية حسب معرّف السطر. الحد الأقصى 256 سطراً.", addLine: "إضافة سطر", line: (number) => `السطر ${number}`, lineId: "معرّف السطر", gross: "المبلغ قبل الضريبة", net: "المبلغ بعد الضريبة", exemptions: "رموز الإعفاء", exemptionsPlaceholder: "رمز واحد لكل سطر أو مفصولة بفواصل", remove: "إزالة", calculating: "جارٍ حساب دليل المستند المحدود…", submit: "محاكاة ضريبة المستند" },
  state: { unavailable: "واجهة API غير متاحة؛ لا يتم استبدال نتيجة ضريبية محلية.", invalidRule: "مستند القاعدة الحالي غير صالح. أصلح مساحة القواعد قبل محاكاة المستند.", noLocalTotal: "الخدمة غير متاحة. لا تعرض مساحة العمل هذه إجمالياً محلياً مختلقاً.", resultRegion: "نتيجة محاكاة المستند", notice: "أعادت الخدمة دليلاً حتمياً وغير موثّق للمستند. لا يمكنه إنشاء عرض أو تفويض تسوية.", failed: (status) => status === undefined ? "فشلت محاكاة المستند" : `فشلت محاكاة المستند (${status})` },
  result: { unattestedTax: "ضريبة مستند غير موثّقة", gross: "الإجمالي", netAfterTax: "الصافي بعد الضريبة", factsDigest: "ملخص الحقائق", fxEvidence: "دليل سعر الصرف", canonicalTrace: "دليل السطر المعياري", tax: "ضريبة", calculationReference: "الحساب" },
};

const JAPANESE: TaxDocumentSimulationMessages = {
  panel: { summary: "明細・請求書の税額シミュレーション", evidenceNote: "これは文書事実を扱うワークベンチです。Rustサービスが上限を検証し、明細IDを正規化し、ルールの明細または請求書スコープを適用して未認証のトレースを返します。ブラウザーでの税計算や決済権限はありません。" },
  form: { documentId: "ドキュメントID", currency: "通貨", jurisdiction: (value) => `管轄は読み込まれたルールに従います: ${value || "未選択"}。`, notSelected: "未選択", linesHeading: "税明細", linesDescription: "正規化された結果は明細ID順に並びます。最大256明細です。", addLine: "明細を追加", line: (number) => `明細 ${number}`, lineId: "明細ID", gross: "税引前金額", net: "税引後金額", exemptions: "免税コード", exemptionsPlaceholder: "明細ごとに1つ、またはカンマ区切り", remove: "削除", calculating: "制限付き文書トレースを計算中…", submit: "文書税をシミュレーション" },
  state: { unavailable: "APIを利用できません。ローカルの税額で代替しません。", invalidRule: "現在のルールドキュメントが無効です。文書をシミュレーションする前にルールワークベンチを修正してください。", noLocalTotal: "サービスを利用できません。このワークベンチは作成したローカル合計を表示しません。", resultRegion: "文書シミュレーション結果", notice: "サービスが決定的な未認証の文書トレースを返しました。見積の作成や決済の承認はできません。", failed: (status) => status === undefined ? "文書シミュレーションに失敗しました" : `文書シミュレーションに失敗しました (${status})` },
  result: { unattestedTax: "未認証の文書税", gross: "総額", netAfterTax: "税引後純額", factsDigest: "事実ダイジェスト", fxEvidence: "FX証拠", canonicalTrace: "正規化された明細トレース", tax: "税", calculationReference: "計算" },
};

const CATALOGS: Record<string, TaxDocumentSimulationMessages> = { "en-US": ENGLISH, "de-DE": GERMAN, "ar-EG": ARABIC, "ja-JP": JAPANESE };

export function taxDocumentSimulationMessagesFor(locale: string): TaxDocumentSimulationMessages {
  return CATALOGS[locale] ?? CATALOGS[DEFAULT_LOCALE] ?? ENGLISH;
}
