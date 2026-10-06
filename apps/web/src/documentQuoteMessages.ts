import { SupportedLocale } from "./formatters";

export interface DocumentQuoteMessages {
  form: {
    eyebrow: string;
    title: string;
    status: string;
    description: string;
    sessionStatus: string;
    signedInSession: string;
    signedInHelp: string;
    signInRequired: string;
    signInHelp: string;
    fields: {
      documentId: string;
      currency: string;
      jurisdiction: string;
      category: string;
      approvedRule: string;
      ruleVersion: string;
      categoryPlaceholder: string;
      rulePlaceholder: string;
    };
    retryTitle: string;
    retryDescription: string;
    location: {
      summary: string;
      description: string;
      label: string;
      placeholder: string;
      help: string;
    };
    lines: {
      heading: string;
      description: string;
      add: string;
      line: (number: number) => string;
      id: string;
      idPlaceholder: string;
      beforeTax: string;
      beforeTaxPlaceholder: string;
      afterTax: string;
      afterTaxPlaceholder: string;
      exemptions: string;
      optional: string;
      remove: string;
    };
    calculating: string;
    submit: string;
    offline: string;
  };
  result: {
    eyebrow: string;
    title: string;
    verified: string;
    needsVerification: string;
    emptyTitle: string;
    emptyCopy: string;
    totalTax: string;
    calculationId: string;
    document: string;
    gross: string;
    netAfterTax: string;
    jurisdiction: string;
    taxRule: string;
    currencyConversion: string;
    expires: string;
    recordDetails: string;
    calculationReference: string;
    factsFingerprint: string;
    invoiceLines: string;
    beforeTax: string;
    afterTax: string;
    tax: string;
    openCheckout: string;
    notAttested: string;
    attestedNotice: string;
    calculatedNotice: string;
  };
  errors: DocumentQuoteErrors;
}

export interface DocumentQuoteErrors {
  serviceUnavailable: string;
  signInRequired: string;
  invalidFields: string;
  evidenceTooLarge: string;
  evidenceInvalidJson: string;
  evidenceArrayBound: string;
  ruleVersionInvalid: string;
  approvedTaxRuleRequired: string;
  taxCategoryRequired: string;
  idempotencyRequired: string;
  quoteRequestFailed: (status: number) => string;
  quoteFailed: string;
}

function displayHttpStatus(status: number): string {
  return Number.isInteger(status) && status >= 100 && status <= 599 ? String(status) : "unknown status";
}

const ENGLISH_ERRORS: DocumentQuoteErrors = {
  serviceUnavailable: "The service is unavailable; no browser-side tax result is substituted.",
  signInRequired: "Sign in with an account allowed to create document quotes.",
  invalidFields: "Document quote fields are invalid.",
  evidenceTooLarge: "Jurisdiction evidence exceeds the 256 KiB browser safety bound.",
  evidenceInvalidJson: "Provider jurisdiction evidence must be valid JSON.",
  evidenceArrayBound: "Provider jurisdiction evidence must be a non-empty bounded JSON array.",
  ruleVersionInvalid: "Rule version must be a positive bounded integer.",
  approvedTaxRuleRequired: "Approved tax rule is required.",
  taxCategoryRequired: "Tax category is required.",
  idempotencyRequired: "An idempotency key is required for durable quote replay.",
  quoteRequestFailed: (status) => `Document quote request failed (${displayHttpStatus(status)}).`,
  quoteFailed: "Document quote could not be completed.",
};

const GERMAN_ERRORS: DocumentQuoteErrors = {
  serviceUnavailable: "Der Dienst ist nicht verfügbar; es wird kein Steuerergebnis im Browser ersetzt.",
  signInRequired: "Melden Sie sich mit einem Konto an, das Rechnungsangebote erstellen darf.",
  invalidFields: "Die Felder für das Rechnungsangebot sind ungültig.",
  evidenceTooLarge: "Der Standortnachweis überschreitet die Browsergrenze von 256 KiB.",
  evidenceInvalidJson: "Der Standortnachweis des Anbieters muss gültiges JSON sein.",
  evidenceArrayBound: "Der Standortnachweis muss ein nicht-leeres, begrenztes JSON-Array sein.",
  ruleVersionInvalid: "Die Regelversion muss eine positive, begrenzte Ganzzahl sein.",
  approvedTaxRuleRequired: "Eine genehmigte Steuerregel ist erforderlich.",
  taxCategoryRequired: "Eine Steuerkategorie ist erforderlich.",
  idempotencyRequired: "Für eine dauerhafte Wiederholung ist ein Idempotenzschlüssel erforderlich.",
  quoteRequestFailed: (status) => `Das Rechnungsangebot konnte nicht angefordert werden (${displayHttpStatus(status)}).`,
  quoteFailed: "Das Rechnungsangebot konnte nicht abgeschlossen werden.",
};

const ARABIC_ERRORS: DocumentQuoteErrors = {
  serviceUnavailable: "الخدمة غير متاحة؛ لن يتم استبدال نتيجة الضريبة بحساب من المتصفح.",
  signInRequired: "سجّل الدخول بحساب مسموح له بإنشاء عروض المستندات.",
  invalidFields: "حقول عرض المستند غير صالحة.",
  evidenceTooLarge: "يتجاوز دليل الاختصاص حد المتصفح البالغ 256 كيلوبايت.",
  evidenceInvalidJson: "يجب أن يكون دليل الاختصاص من المزوّد بصيغة JSON صحيحة.",
  evidenceArrayBound: "يجب أن يكون دليل الاختصاص مصفوفة JSON غير فارغة ومحدودة.",
  ruleVersionInvalid: "يجب أن يكون إصدار القاعدة عددًا صحيحًا موجبًا ومحدودًا.",
  approvedTaxRuleRequired: "القاعدة الضريبية المعتمدة مطلوبة.",
  taxCategoryRequired: "الفئة الضريبية مطلوبة.",
  idempotencyRequired: "مفتاح منع التكرار مطلوب لإعادة المحاولة الدائمة للعرض.",
  quoteRequestFailed: (status) => `تعذر طلب عرض المستند (${displayHttpStatus(status)}).`,
  quoteFailed: "تعذر إكمال عرض المستند.",
};

const JAPANESE_ERRORS: DocumentQuoteErrors = {
  serviceUnavailable: "サービスを利用できません。ブラウザ側の税額で置き換えることはありません。",
  signInRequired: "ドキュメント見積もりを作成できるアカウントでサインインしてください。",
  invalidFields: "ドキュメント見積もりの入力が正しくありません。",
  evidenceTooLarge: "管轄証拠がブラウザの256 KiB制限を超えています。",
  evidenceInvalidJson: "プロバイダーの管轄証拠は有効なJSONである必要があります。",
  evidenceArrayBound: "プロバイダーの管轄証拠は空でない制限付きJSON配列である必要があります。",
  ruleVersionInvalid: "ルールバージョンは正の範囲内の整数である必要があります。",
  approvedTaxRuleRequired: "承認済み税ルールが必要です。",
  taxCategoryRequired: "税区分が必要です。",
  idempotencyRequired: "永続的な見積もり再試行には冪等性キーが必要です。",
  quoteRequestFailed: (status) => `ドキュメント見積もりを要求できませんでした（${displayHttpStatus(status)}）。`,
  quoteFailed: "ドキュメント見積もりを完了できませんでした。",
};

const ENGLISH: DocumentQuoteMessages = {
  form: {
    eyebrow: "Invoice tax",
    title: "Calculate tax for an invoice",
    status: "Ready to review",
    description: "Enter the invoice details and verified location evidence. The service applies the selected published rule, saves the calculation, and keeps the result available for review and payment.",
    sessionStatus: "Sign-in status",
    signedInSession: "Signed-in session ready",
    signedInHelp: "Your session is held in memory and is used only for this request. The credential is never displayed in this workspace.",
    signInRequired: "Sign-in required",
    signInHelp: "Sign in before preparing an invoice calculation.",
    fields: {
      documentId: "Document ID",
      currency: "Currency",
      jurisdiction: "Jurisdiction",
      category: "Tax category",
      approvedRule: "Approved tax rule",
      ruleVersion: "Rule version",
      categoryPlaceholder: "For example, sales tax",
      rulePlaceholder: "Enter the approved rule reference",
    },
    retryTitle: "Retry protection selected automatically",
    retryDescription: "A secure retry reference is assigned automatically and stays the same while these invoice details remain unchanged.",
    location: {
      summary: "Location verification",
      description: "A production quote requires a record from an approved location provider. This advanced exchange keeps the signed record intact for server verification.",
      label: "Provider-signed location record (advanced)",
      placeholder: "Paste the signed record from your approved provider",
      help: "The service checks the source, account, dates, and location before using this record. It does not guess a customer location.",
    },
    lines: {
      heading: "Invoice lines",
      description: "Enter each line explicitly. The service validates line identity, amounts, exemptions, and conservation before saving the quote.",
      add: "Add line",
      line: (number) => `Line ${number}`,
      id: "Line ID",
      idPlaceholder: "Enter a unique line ID",
      beforeTax: "Amount before tax",
      beforeTaxPlaceholder: "Enter the line amount",
      afterTax: "Amount after tax",
      afterTaxPlaceholder: "Enter the taxable amount",
      exemptions: "Exemption codes",
      optional: "Optional",
      remove: "Remove line",
    },
    calculating: "Calculating invoice tax…",
    submit: "Calculate invoice tax",
    offline: "The service is unavailable. No locally invented total is displayed.",
  },
  result: {
    eyebrow: "Invoice result",
    title: "Review the tax calculation",
    verified: "Verified",
    needsVerification: "Needs verification",
    emptyTitle: "No invoice calculation yet",
    emptyCopy: "Enter the invoice details and location evidence to review the tax result.",
    totalTax: "Total tax withheld",
    calculationId: "Calculation ID",
    document: "Document",
    gross: "Gross",
    netAfterTax: "Net after tax",
    jurisdiction: "Jurisdiction",
    taxRule: "Tax rule",
    currencyConversion: "Currency conversion",
    expires: "Expires",
    recordDetails: "Record details",
    calculationReference: "Calculation reference",
    factsFingerprint: "Facts fingerprint",
    invoiceLines: "Invoice lines",
    beforeTax: "Before tax",
    afterTax: "after tax",
    tax: "tax",
    openCheckout: "Open document checkout",
    notAttested: "The quote is calculated but not attested. Checkout will require a valid issuer attestation before compilation or signing.",
    attestedNotice: "Persisted document evidence returned by the authenticated API with issuer attestation. It is not settled yet.",
    calculatedNotice: "Persisted document evidence returned by the authenticated API. It is not attested or settled yet.",
  },
  errors: ENGLISH_ERRORS,
};

const GERMAN: DocumentQuoteMessages = {
  form: {
    eyebrow: "Rechnungssteuer",
    title: "Steuer für eine Rechnung berechnen",
    status: "Bereit zur Prüfung",
    description: "Geben Sie Rechnungsdaten und geprüfte Standortnachweise ein. Der Dienst wendet die veröffentlichte Regel an, speichert die Berechnung und hält das Ergebnis für Prüfung und Zahlung bereit.",
    sessionStatus: "Anmeldestatus",
    signedInSession: "Angemeldete Sitzung bereit",
    signedInHelp: "Ihre Sitzung wird nur für diese Anfrage im Speicher gehalten. Das Zugangsmittel wird in diesem Arbeitsbereich nie angezeigt.",
    signInRequired: "Anmeldung erforderlich",
    signInHelp: "Melden Sie sich an, bevor Sie eine Rechnungsberechnung vorbereiten.",
    fields: {
      documentId: "Dokument-ID",
      currency: "Währung",
      jurisdiction: "Steuergebiet",
      category: "Steuerkategorie",
      approvedRule: "Genehmigte Steuerregel",
      ruleVersion: "Regelversion",
      categoryPlaceholder: "Zum Beispiel Umsatzsteuer",
      rulePlaceholder: "Referenz der genehmigten Regel eingeben",
    },
    retryTitle: "Wiederholungsschutz automatisch ausgewählt",
    retryDescription: "Eine sichere Wiederholungsreferenz wird automatisch vergeben und bleibt gleich, solange diese Rechnungsdaten unverändert bleiben.",
    location: {
      summary: "Standortprüfung",
      description: "Für ein Produktionsangebot ist ein Datensatz eines genehmigten Standortanbieters erforderlich. Dieser erweiterte Austausch bewahrt den signierten Datensatz für die Serverprüfung.",
      label: "Vom Anbieter signierter Standortdatensatz (erweitert)",
      placeholder: "Signierten Datensatz des genehmigten Anbieters einfügen",
      help: "Der Dienst prüft Quelle, Konto, Daten und Standort, bevor er diesen Datensatz verwendet. Der Kundenstandort wird nicht erraten.",
    },
    lines: {
      heading: "Rechnungspositionen",
      description: "Geben Sie jede Position ausdrücklich ein. Der Dienst prüft Identität, Beträge, Befreiungen und Erhaltung, bevor das Angebot gespeichert wird.",
      add: "Position hinzufügen",
      line: (number) => `Position ${number}`,
      id: "Positions-ID",
      idPlaceholder: "Eindeutige Positions-ID eingeben",
      beforeTax: "Betrag vor Steuer",
      beforeTaxPlaceholder: "Positionsbetrag eingeben",
      afterTax: "Betrag nach Steuer",
      afterTaxPlaceholder: "Steuerpflichtigen Betrag eingeben",
      exemptions: "Befreiungscodes",
      optional: "Optional",
      remove: "Position entfernen",
    },
    calculating: "Rechnungssteuer wird berechnet…",
    submit: "Rechnungssteuer berechnen",
    offline: "Der Dienst ist nicht verfügbar. Es wird kein lokal erfundener Gesamtbetrag angezeigt.",
  },
  result: {
    eyebrow: "Rechnungsergebnis",
    title: "Steuerberechnung prüfen",
    verified: "Verifiziert",
    needsVerification: "Verifizierung erforderlich",
    emptyTitle: "Noch keine Rechnungsberechnung",
    emptyCopy: "Geben Sie Rechnungsdaten und Standortnachweis ein, um das Steuerergebnis zu prüfen.",
    totalTax: "Einbehaltene Gesamtsteuer",
    calculationId: "Berechnungs-ID",
    document: "Dokument",
    gross: "Brutto",
    netAfterTax: "Netto nach Steuer",
    jurisdiction: "Steuergebiet",
    taxRule: "Steuerregel",
    currencyConversion: "Währungsumrechnung",
    expires: "Läuft ab",
    recordDetails: "Datensatzdetails",
    calculationReference: "Berechnungsreferenz",
    factsFingerprint: "Fakten-Fingerabdruck",
    invoiceLines: "Rechnungspositionen",
    beforeTax: "Vor Steuer",
    afterTax: "nach Steuer",
    tax: "Steuer",
    openCheckout: "Dokumentzahlung öffnen",
    notAttested: "Das Angebot ist berechnet, aber nicht attestiert. Vor Kompilierung oder Signatur ist eine gültige Ausstellerattestierung erforderlich.",
    attestedNotice: "Der authentifizierte Dienst hat persistierte Rechnungsnachweise mit Ausstellerattestierung zurückgegeben. Sie sind noch nicht abgewickelt.",
    calculatedNotice: "Der authentifizierte Dienst hat persistierte Rechnungsnachweise zurückgegeben. Sie sind noch nicht attestiert oder abgewickelt.",
  },
  errors: GERMAN_ERRORS,
};

const ARABIC: DocumentQuoteMessages = {
  form: {
    eyebrow: "ضريبة الفاتورة",
    title: "حساب الضريبة لفاتورة",
    status: "جاهز للمراجعة",
    description: "أدخل بيانات الفاتورة ودليل الموقع المتحقق منه. تطبق الخدمة القاعدة المنشورة المحددة وتحفظ الحساب وتبقي النتيجة متاحة للمراجعة والدفع.",
    sessionStatus: "حالة تسجيل الدخول",
    signedInSession: "جلسة تسجيل الدخول جاهزة",
    signedInHelp: "تُحفظ جلستك في الذاكرة وتُستخدم لهذا الطلب فقط. لا تظهر بيانات الاعتماد في مساحة العمل هذه.",
    signInRequired: "تسجيل الدخول مطلوب",
    signInHelp: "سجّل الدخول قبل إعداد حساب الفاتورة.",
    fields: {
      documentId: "معرّف المستند",
      currency: "العملة",
      jurisdiction: "الاختصاص الضريبي",
      category: "الفئة الضريبية",
      approvedRule: "القاعدة الضريبية المعتمدة",
      ruleVersion: "إصدار القاعدة",
      categoryPlaceholder: "مثال: ضريبة المبيعات",
      rulePlaceholder: "أدخل مرجع القاعدة المعتمدة",
    },
    retryTitle: "تم اختيار حماية التكرار تلقائياً",
    retryDescription: "يُعيّن مرجع آمن لإعادة المحاولة تلقائياً ويبقى نفسه ما دامت بيانات الفاتورة دون تغيير.",
    location: {
      summary: "التحقق من الموقع",
      description: "يتطلب العرض الإنتاجي سجلاً من مزود موقع معتمد. يحافظ هذا التبادل المتقدم على السجل الموقّع للتحقق من الخدمة.",
      label: "سجل الموقع الموقّع من المزود (متقدم)",
      placeholder: "ألصق السجل الموقّع من مزودك المعتمد",
      help: "تتحقق الخدمة من المصدر والحساب والتواريخ والموقع قبل استخدام السجل. لا تخمّن موقع العميل.",
    },
    lines: {
      heading: "بنود الفاتورة",
      description: "أدخل كل بند صراحةً. تتحقق الخدمة من هوية البند والمبالغ والإعفاءات وحفظ القيمة قبل حفظ العرض.",
      add: "إضافة بند",
      line: (number) => `البند ${number}`,
      id: "معرّف البند",
      idPlaceholder: "أدخل معرّفاً فريداً للبند",
      beforeTax: "المبلغ قبل الضريبة",
      beforeTaxPlaceholder: "أدخل مبلغ البند",
      afterTax: "المبلغ بعد الضريبة",
      afterTaxPlaceholder: "أدخل المبلغ الخاضع للضريبة",
      exemptions: "رموز الإعفاء",
      optional: "اختياري",
      remove: "إزالة البند",
    },
    calculating: "جارٍ حساب ضريبة الفاتورة…",
    submit: "حساب ضريبة الفاتورة",
    offline: "الخدمة غير متاحة. لا يتم عرض إجمالي مُخترع محلياً.",
  },
  result: {
    eyebrow: "نتيجة الفاتورة",
    title: "مراجعة حساب الضريبة",
    verified: "تم التحقق",
    needsVerification: "يتطلب التحقق",
    emptyTitle: "لا يوجد حساب فاتورة بعد",
    emptyCopy: "أدخل بيانات الفاتورة ودليل الموقع لمراجعة النتيجة الضريبية.",
    totalTax: "إجمالي الضريبة المحجوزة",
    calculationId: "معرّف الحساب",
    document: "المستند",
    gross: "الإجمالي",
    netAfterTax: "الصافي بعد الضريبة",
    jurisdiction: "الاختصاص الضريبي",
    taxRule: "القاعدة الضريبية",
    currencyConversion: "تحويل العملة",
    expires: "ينتهي",
    recordDetails: "تفاصيل السجل",
    calculationReference: "مرجع الحساب",
    factsFingerprint: "بصمة الحقائق",
    invoiceLines: "بنود الفاتورة",
    beforeTax: "قبل الضريبة",
    afterTax: "بعد الضريبة",
    tax: "ضريبة",
    openCheckout: "فتح دفع المستند",
    notAttested: "تم حساب العرض لكنه غير موثّق. سيتطلب الدفع توثيقاً صالحاً من الجهة المصدرة قبل التجميع أو التوقيع.",
    attestedNotice: "أعادت الخدمة الموثقة أدلة فاتورة محفوظة مع توثيق الجهة المصدرة. لم تتم التسوية بعد.",
    calculatedNotice: "أعادت الخدمة الموثقة أدلة فاتورة محفوظة. لم يتم توثيقها أو تسويتها بعد.",
  },
  errors: ARABIC_ERRORS,
};

const JAPANESE: DocumentQuoteMessages = {
  form: {
    eyebrow: "請求書の税",
    title: "請求書の税額を計算",
    status: "確認の準備完了",
    description: "請求書の詳細と検証済みの所在地証拠を入力してください。サービスが公開済みルールを適用し、計算を保存して、確認と支払いに利用できる状態にします。",
    sessionStatus: "サインイン状態",
    signedInSession: "サインイン済みセッションの準備完了",
    signedInHelp: "セッションはメモリ内に保持され、このリクエストだけに使用されます。このワークスペースに認証情報は表示されません。",
    signInRequired: "サインインが必要です",
    signInHelp: "請求書の計算を準備する前にサインインしてください。",
    fields: {
      documentId: "ドキュメント ID",
      currency: "通貨",
      jurisdiction: "管轄区域",
      category: "税区分",
      approvedRule: "承認済み税ルール",
      ruleVersion: "ルールバージョン",
      categoryPlaceholder: "例：売上税",
      rulePlaceholder: "承認済みルールの参照を入力",
    },
    retryTitle: "再試行保護を自動選択しました",
    retryDescription: "安全な再試行参照が自動で割り当てられ、請求書の詳細が変わらない間は同じ値を使います。",
    location: {
      summary: "所在地の検証",
      description: "本番の見積もりには、承認済み所在地プロバイダーの記録が必要です。この高度な交換では、サーバー検証のため署名済み記録をそのまま保持します。",
      label: "プロバイダー署名済み所在地記録（高度）",
      placeholder: "承認済みプロバイダーの署名済み記録を貼り付け",
      help: "サービスはこの記録を使う前に、ソース、アカウント、日付、所在地を確認します。顧客所在地を推測しません。",
    },
    lines: {
      heading: "請求書明細",
      description: "各明細を明示的に入力してください。保存前にサービスが明細の識別子、金額、免税、保存則を検証します。",
      add: "明細を追加",
      line: (number) => `明細 ${number}`,
      id: "明細 ID",
      idPlaceholder: "一意の明細 ID を入力",
      beforeTax: "税引前金額",
      beforeTaxPlaceholder: "明細金額を入力",
      afterTax: "税引後金額",
      afterTaxPlaceholder: "課税対象額を入力",
      exemptions: "免税コード",
      optional: "任意",
      remove: "明細を削除",
    },
    calculating: "請求書の税額を計算中…",
    submit: "請求書の税額を計算",
    offline: "サービスを利用できません。ブラウザーで作成した合計は表示しません。",
  },
  result: {
    eyebrow: "請求書の結果",
    title: "税額計算を確認",
    verified: "検証済み",
    needsVerification: "検証が必要",
    emptyTitle: "請求書の計算はまだありません",
    emptyCopy: "請求書の詳細と所在地証拠を入力して税額結果を確認してください。",
    totalTax: "源泉徴収税額合計",
    calculationId: "計算 ID",
    document: "ドキュメント",
    gross: "総額",
    netAfterTax: "税引後純額",
    jurisdiction: "管轄区域",
    taxRule: "税ルール",
    currencyConversion: "通貨換算",
    expires: "有効期限",
    recordDetails: "記録の詳細",
    calculationReference: "計算参照",
    factsFingerprint: "事実フィンガープリント",
    invoiceLines: "請求書明細",
    beforeTax: "税引前",
    afterTax: "税引後",
    tax: "税",
    openCheckout: "ドキュメント支払いを開く",
    notAttested: "見積もりは計算済みですが、証明されていません。コンパイルまたは署名前に有効な発行者証明が必要です。",
    attestedNotice: "認証済み API が発行者証明付きの保存済み請求書証拠を返しました。まだ決済されていません。",
    calculatedNotice: "認証済み API が保存済み請求書証拠を返しました。まだ証明または決済されていません。",
  },
  errors: JAPANESE_ERRORS,
};

const CATALOGS: Readonly<Record<SupportedLocale, DocumentQuoteMessages>> = {
  "en-US": ENGLISH,
  "de-DE": GERMAN,
  "ar-EG": ARABIC,
  "ja-JP": JAPANESE,
};

export function documentQuoteMessagesFor(locale: string): DocumentQuoteMessages {
  return CATALOGS[locale as SupportedLocale] ?? ENGLISH;
}
