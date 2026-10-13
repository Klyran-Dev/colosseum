import { SupportedLocale } from "./formatters";

export interface QuoteModeMessages {
  eyebrow: string;
  title: string;
  status: string;
  description: string;
  submit: string;
  result: {
    whyTax: string;
    explanation: string;
    ruleLabel: string;
    emptyCopy: string;
  };
}

export interface QuoteWorkspaceMessages {
  quoteMode: QuoteModeMessages;
  previewMode: QuoteModeMessages;
  readOnlyWarning: string;
  location: {
    summary: string;
    description: string;
    label: string;
    placeholder: string;
    help: string;
  };
  fields: {
    grossLabel: string;
    grossPlaceholder: string;
    netLabel: string;
    netPlaceholder: string;
    currencyLabel: string;
    currencyPlaceholder: string;
    rateLabel: string;
    ratePlaceholder: string;
    approvedRuleLabel: string;
    approvedRulePlaceholder: string;
    jurisdictionLabel: string;
    jurisdictionPlaceholder: string;
    ruleVersionLabel: string;
    ruleVersionPlaceholder: string;
    categoryLabel: string;
    categoryPlaceholder: string;
  };
  retry: {
    title: string;
    description: string;
  };
  amountHelp: string;
  offlineWarning: string;
  preparing: string;
  result: {
    eyebrow: string;
    title: string;
    verified: string;
    needsVerification: string;
    scenarioOnly: string;
    whyTax: string;
    explanation: string;
    approvedRule: string;
    currencyConversion: string;
    recordFingerprints: string;
    ruleFingerprint: string;
    inputFingerprint: string;
    emptyTitle: string;
    emptyCopy: string;
    totalTax: string;
    reference: string;
    jurisdiction: string;
    amountAfterTax: string;
    resultStatus: string;
    verifiedStatus: string;
    calculatedStatus: string;
    scenarioStatus: string;
    technicalDetails: string;
    calculationReference: string;
    engineVersion: string;
    legacyEvidence: string;
    policyState: string;
    publishedAttestation: string;
    publishedNotAttested: string;
    callerPreview: string;
    checkRecord: string;
    checkingRecord: string;
    recordChecked: string;
    evidenceNote: string;
    checkStatus: string;
    factsDigest: string;
    ruleDigest: string;
  };
}

const ENGLISH: QuoteWorkspaceMessages = {
  quoteMode: {
    eyebrow: "Customer quote",
    title: "Create a tax quote",
    status: "Ready to review",
    description: "Create a quote from an approved tax rule. Your account and the quote details are checked by the service, and the result can be reviewed later.",
    submit: "Prepare quote",
    result: {
      whyTax: "Why this tax?",
      explanation: "The service applied the approved tax rule to the location and amounts you entered. This explanation helps you review the result; it is not tax advice, a filing, or a compliance guarantee.",
      ruleLabel: "Approved tax rule",
      emptyCopy: "Prepare a quote to review the tax amount and its explanation.",
    },
  },
  previewMode: {
    eyebrow: "Try a scenario",
    title: "Explore a tax scenario",
    status: "No live changes",
    description: "Explore a what-if scenario. It does not change a live rule or authorize a payment.",
    submit: "Calculate scenario",
    result: {
      whyTax: "How this scenario was calculated",
      explanation: "The service calculated this read-only scenario from the amounts and rate you entered. It does not use or change a live tax rule. This explanation helps you review the result; it is not tax advice, a filing, or a compliance guarantee.",
      ruleLabel: "Scenario rule",
      emptyCopy: "Explore a scenario to review the tax amount and its explanation.",
    },
  },
  readOnlyWarning: "This workspace is read-only until secure sign-in is configured. No quote will be submitted.",
  location: {
    summary: "Location verification",
    description: "A production quote requires a record from an approved location provider. This advanced exchange keeps the signed record intact for server verification.",
    label: "Provider-signed location record (advanced)",
    placeholder: "Paste the signed record from your approved provider",
    help: "Required for production quotes. The service checks the source, account, dates, and location before it uses this record.",
  },
  fields: {
    grossLabel: "Total before tax",
    grossPlaceholder: "Enter the total amount",
    netLabel: "Amount subject to tax",
    netPlaceholder: "Enter the taxable amount",
    currencyLabel: "Currency",
    currencyPlaceholder: "Enter the currency code",
    rateLabel: "Tax rate (%)",
    ratePlaceholder: "Enter a rate",
    approvedRuleLabel: "Approved tax rule",
    approvedRulePlaceholder: "Enter the approved rule reference",
    jurisdictionLabel: "Jurisdiction",
    jurisdictionPlaceholder: "Enter a jurisdiction",
    ruleVersionLabel: "Rule version",
    ruleVersionPlaceholder: "Enter the rule version",
    categoryLabel: "Tax category",
    categoryPlaceholder: "Enter the tax category",
  },
  retry: {
    title: "Retry protection selected automatically",
    description: "A secure retry reference is assigned automatically and stays the same while these quote details remain unchanged.",
  },
  amountHelp: "Enter amounts in the smallest supported account unit. The service keeps the calculation exact and does not infer missing values.",
  offlineWarning: "The service is unavailable. No client-side fallback is used.",
  preparing: "Preparing…",
  result: {
    eyebrow: "Your tax breakdown",
    title: "Review the result",
    verified: "Verified",
    needsVerification: "Needs verification",
    scenarioOnly: "Scenario only",
    whyTax: "Why this tax?",
    explanation: "The service applied the approved tax rule to the location and amounts you entered. This explanation helps you review the result; it is not tax advice, a filing, or a compliance guarantee.",
    approvedRule: "Approved tax rule",
    currencyConversion: "Currency conversion",
    recordFingerprints: "Record fingerprints",
    ruleFingerprint: "Rule fingerprint",
    inputFingerprint: "Input fingerprint",
    emptyTitle: "Your result will appear here",
    emptyCopy: "Prepare a quote or explore a scenario to review the tax amount and its explanation.",
    totalTax: "Total tax",
    reference: "Reference",
    jurisdiction: "Jurisdiction",
    amountAfterTax: "Amount after tax",
    resultStatus: "Result status",
    verifiedStatus: "Verified against approved evidence",
    calculatedStatus: "Calculated; verification still needed",
    scenarioStatus: "Scenario supplied for exploration",
    technicalDetails: "Technical and audit details",
    calculationReference: "Calculation reference",
    engineVersion: "Engine version",
    legacyEvidence: "Legacy evidence",
    policyState: "Policy state",
    publishedAttestation: "Published rule and issuer attestation",
    publishedNotAttested: "Published rule, not attested",
    callerPreview: "Caller-supplied preview",
    checkRecord: "Check saved record",
    checkingRecord: "Checking record…",
    recordChecked: "Record checked",
    evidenceNote: "This checks the saved facts, approved rule, engine version, and full component breakdown. It does not approve or move funds.",
    checkStatus: "Check status",
    factsDigest: "Facts digest",
    ruleDigest: "Rule digest",
  },
};

const GERMAN: QuoteWorkspaceMessages = {
  quoteMode: {
    eyebrow: "Kundenangebot",
    title: "Steuerangebot erstellen",
    status: "Bereit zur Prüfung",
    description: "Erstellen Sie ein Angebot aus einer genehmigten Steuerregel. Konto und Angebotsdaten werden vom Dienst geprüft und können später erneut eingesehen werden.",
    submit: "Angebot vorbereiten",
    result: {
      whyTax: "Warum diese Steuer?",
      explanation: "Der Dienst hat die genehmigte Steuerregel auf Standort und eingegebene Beträge angewendet. Diese Erklärung unterstützt die Prüfung; sie ist keine Steuerberatung, Steuererklärung oder Compliance-Garantie.",
      ruleLabel: "Genehmigte Steuerregel",
      emptyCopy: "Bereiten Sie ein Angebot vor, um Steuerbetrag und Erklärung zu prüfen.",
    },
  },
  previewMode: {
    eyebrow: "Szenario testen",
    title: "Steuerszenario erkunden",
    status: "Keine Live-Änderungen",
    description: "Prüfen Sie ein Was-wäre-wenn-Szenario. Eine aktive Regel wird nicht geändert und keine Zahlung autorisiert.",
    submit: "Szenario berechnen",
    result: {
      whyTax: "So wurde dieses Szenario berechnet",
      explanation: "Der Dienst hat dieses schreibgeschützte Szenario anhand der eingegebenen Beträge und des eingegebenen Satzes berechnet. Eine aktive Steuerregel wird weder verwendet noch geändert. Diese Erklärung unterstützt die Prüfung; sie ist keine Steuerberatung, Steuererklärung oder Compliance-Garantie.",
      ruleLabel: "Szenarioregel",
      emptyCopy: "Erkunden Sie ein Szenario, um Steuerbetrag und Erklärung zu prüfen.",
    },
  },
  readOnlyWarning: "Dieser Arbeitsbereich ist schreibgeschützt, bis die sichere Anmeldung eingerichtet ist. Es wird kein Angebot übermittelt.",
  location: {
    summary: "Standortprüfung",
    description: "Für ein Produktionsangebot ist ein Datensatz eines genehmigten Standortanbieters erforderlich. Dieser erweiterte Austausch bewahrt den signierten Datensatz für die Serverprüfung.",
    label: "Vom Anbieter signierter Standortdatensatz (erweitert)",
    placeholder: "Signierten Datensatz des genehmigten Anbieters einfügen",
    help: "Für Produktionsangebote erforderlich. Der Dienst prüft Quelle, Konto, Daten und Standort, bevor er diesen Datensatz verwendet.",
  },
  fields: {
    grossLabel: "Gesamt vor Steuer",
    grossPlaceholder: "Gesamtbetrag eingeben",
    netLabel: "Steuerpflichtiger Betrag",
    netPlaceholder: "Steuerpflichtigen Betrag eingeben",
    currencyLabel: "Währung",
    currencyPlaceholder: "Währungscode eingeben",
    rateLabel: "Steuersatz (%)",
    ratePlaceholder: "Satz eingeben",
    approvedRuleLabel: "Genehmigte Steuerregel",
    approvedRulePlaceholder: "Referenz der genehmigten Regel eingeben",
    jurisdictionLabel: "Steuergebiet",
    jurisdictionPlaceholder: "Steuergebiet eingeben",
    ruleVersionLabel: "Regelversion",
    ruleVersionPlaceholder: "Regelversion eingeben",
    categoryLabel: "Steuerkategorie",
    categoryPlaceholder: "Steuerkategorie eingeben",
  },
  retry: {
    title: "Wiederholungsschutz automatisch ausgewählt",
    description: "Eine sichere Wiederholungsreferenz wird automatisch vergeben und bleibt gleich, solange diese Angebotsdaten unverändert bleiben.",
  },
  amountHelp: "Beträge in der kleinsten unterstützten Kontoeinheit eingeben. Der Dienst berechnet exakt und ergänzt keine fehlenden Werte.",
  offlineWarning: "Der Dienst ist nicht verfügbar. Es wird kein clientseitiger Ersatz verwendet.",
  preparing: "Wird vorbereitet…",
  result: {
    eyebrow: "Ihre Steueraufschlüsselung",
    title: "Ergebnis prüfen",
    verified: "Verifiziert",
    needsVerification: "Verifizierung erforderlich",
    scenarioOnly: "Nur Szenario",
    whyTax: "Warum diese Steuer?",
    explanation: "Der Dienst hat die genehmigte Steuerregel auf Standort und eingegebene Beträge angewendet. Diese Erklärung unterstützt die Prüfung; sie ist keine Steuerberatung, Steuererklärung oder Compliance-Garantie.",
    approvedRule: "Genehmigte Steuerregel",
    currencyConversion: "Währungsumrechnung",
    recordFingerprints: "Datensatz-Fingerabdrücke",
    ruleFingerprint: "Regel-Fingerabdruck",
    inputFingerprint: "Eingabe-Fingerabdruck",
    emptyTitle: "Ihr Ergebnis erscheint hier",
    emptyCopy: "Bereiten Sie ein Angebot vor oder prüfen Sie ein Szenario, um Steuerbetrag und Erklärung zu sehen.",
    totalTax: "Gesamtsteuer",
    reference: "Referenz",
    jurisdiction: "Steuergebiet",
    amountAfterTax: "Betrag nach Steuer",
    resultStatus: "Ergebnisstatus",
    verifiedStatus: "Anhand genehmigter Nachweise verifiziert",
    calculatedStatus: "Berechnet; Verifizierung noch erforderlich",
    scenarioStatus: "Szenario zur Erkundung bereitgestellt",
    technicalDetails: "Technische und Prüfdetails",
    calculationReference: "Berechnungsreferenz",
    engineVersion: "Engine-Version",
    legacyEvidence: "Ältere Nachweise",
    policyState: "Richtlinienstatus",
    publishedAttestation: "Veröffentlichte Regel und Ausstellerattestierung",
    publishedNotAttested: "Veröffentlichte Regel, nicht attestiert",
    callerPreview: "Vom Aufrufer bereitgestellte Vorschau",
    checkRecord: "Gespeicherten Datensatz prüfen",
    checkingRecord: "Datensatz wird geprüft…",
    recordChecked: "Datensatz geprüft",
    evidenceNote: "Geprüft werden gespeicherte Fakten, genehmigte Regel, Engine-Version und die vollständige Komponentenaufschlüsselung. Es werden keine Mittel genehmigt oder bewegt.",
    checkStatus: "Prüfstatus",
    factsDigest: "Fakten-Digest",
    ruleDigest: "Regel-Digest",
  },
};

const ARABIC: QuoteWorkspaceMessages = {
  quoteMode: {
    eyebrow: "عرض العميل",
    title: "إنشاء عرض ضريبي",
    status: "جاهز للمراجعة",
    description: "أنشئ عرضًا من قاعدة ضريبية معتمدة. يتحقق النظام من حسابك وتفاصيل العرض ويمكن مراجعة النتيجة لاحقًا.",
    submit: "إعداد العرض",
    result: {
      whyTax: "لماذا هذه الضريبة؟",
      explanation: "طبّق النظام القاعدة الضريبية المعتمدة على الموقع والمبالغ التي أدخلتها. تساعدك هذه الشروحات على المراجعة؛ وليست استشارة ضريبية أو إقرارًا أو ضمانًا للامتثال.",
      ruleLabel: "القاعدة الضريبية المعتمدة",
      emptyCopy: "أعد عرضًا لمراجعة مبلغ الضريبة وشرحه.",
    },
  },
  previewMode: {
    eyebrow: "تجربة سيناريو",
    title: "استكشاف سيناريو ضريبي",
    status: "لا تغييرات مباشرة",
    description: "استكشف سيناريو افتراضيًا. لا يغيّر قاعدة مباشرة ولا يصرّح بدفعة.",
    submit: "حساب السيناريو",
    result: {
      whyTax: "كيف حُسب هذا السيناريو؟",
      explanation: "حسب النظام هذا السيناريو للقراءة فقط باستخدام المبالغ والمعدل اللذين أدخلتهما. لا يستخدم قاعدة ضريبية مباشرة ولا يغيّرها. تساعدك هذه الشروحات على المراجعة؛ وليست استشارة ضريبية أو إقرارًا أو ضمانًا للامتثال.",
      ruleLabel: "قاعدة السيناريو",
      emptyCopy: "استكشف سيناريو لمراجعة مبلغ الضريبة وشرحه.",
    },
  },
  readOnlyWarning: "مساحة العمل للقراءة فقط حتى يتم إعداد تسجيل الدخول الآمن. لن يتم إرسال أي عرض.",
  location: {
    summary: "التحقق من الموقع",
    description: "يتطلب العرض الإنتاجي سجلًا من موفر مواقع معتمد. يحافظ هذا التبادل المتقدم على السجل الموقّع للتحقق من الخادم.",
    label: "سجل موقع موقّع من الموفر (متقدم)",
    placeholder: "ألصق السجل الموقّع من الموفر المعتمد",
    help: "مطلوب للعروض الإنتاجية. يتحقق النظام من المصدر والحساب والتواريخ والموقع قبل استخدام السجل.",
  },
  fields: {
    grossLabel: "الإجمالي قبل الضريبة",
    grossPlaceholder: "أدخل المبلغ الإجمالي",
    netLabel: "المبلغ الخاضع للضريبة",
    netPlaceholder: "أدخل المبلغ الخاضع للضريبة",
    currencyLabel: "العملة",
    currencyPlaceholder: "أدخل رمز العملة",
    rateLabel: "معدل الضريبة (%)",
    ratePlaceholder: "أدخل المعدل",
    approvedRuleLabel: "القاعدة الضريبية المعتمدة",
    approvedRulePlaceholder: "أدخل مرجع القاعدة المعتمدة",
    jurisdictionLabel: "الاختصاص الضريبي",
    jurisdictionPlaceholder: "أدخل الاختصاص الضريبي",
    ruleVersionLabel: "إصدار القاعدة",
    ruleVersionPlaceholder: "أدخل إصدار القاعدة",
    categoryLabel: "الفئة الضريبية",
    categoryPlaceholder: "أدخل الفئة الضريبية",
  },
  retry: {
    title: "تم اختيار حماية إعادة المحاولة تلقائيًا",
    description: "يُعيّن مرجع آمن لإعادة المحاولة تلقائيًا ويبقى نفسه ما دامت تفاصيل العرض دون تغيير.",
  },
  amountHelp: "أدخل المبالغ بأصغر وحدة حساب مدعومة. يحافظ النظام على دقة الحساب ولا يستنتج القيم المفقودة.",
  offlineWarning: "الخدمة غير متاحة. لا يتم استخدام بديل من جهة العميل.",
  preparing: "جارٍ الإعداد…",
  result: {
    eyebrow: "تفصيل الضريبة الخاص بك",
    title: "مراجعة النتيجة",
    verified: "تم التحقق",
    needsVerification: "يلزم التحقق",
    scenarioOnly: "سيناريو فقط",
    whyTax: "لماذا هذه الضريبة؟",
    explanation: "طبّق النظام القاعدة الضريبية المعتمدة على الموقع والمبالغ التي أدخلتها. تساعدك هذه الشروحات على المراجعة؛ وليست استشارة ضريبية أو إقرارًا أو ضمانًا للامتثال.",
    approvedRule: "القاعدة الضريبية المعتمدة",
    currencyConversion: "تحويل العملة",
    recordFingerprints: "بصمات السجل",
    ruleFingerprint: "بصمة القاعدة",
    inputFingerprint: "بصمة الإدخال",
    emptyTitle: "ستظهر نتيجتك هنا",
    emptyCopy: "أعد عرضًا أو استكشف سيناريو لمراجعة مبلغ الضريبة وشرحه.",
    totalTax: "إجمالي الضريبة",
    reference: "المرجع",
    jurisdiction: "الاختصاص الضريبي",
    amountAfterTax: "المبلغ بعد الضريبة",
    resultStatus: "حالة النتيجة",
    verifiedStatus: "تم التحقق منها مقابل الأدلة المعتمدة",
    calculatedStatus: "تم الحساب؛ لا يزال التحقق مطلوبًا",
    scenarioStatus: "سيناريو مقدم للاستكشاف",
    technicalDetails: "التفاصيل التقنية وتفاصيل التدقيق",
    calculationReference: "مرجع الحساب",
    engineVersion: "إصدار المحرك",
    legacyEvidence: "أدلة قديمة",
    policyState: "حالة السياسة",
    publishedAttestation: "القاعدة المنشورة وشهادة الجهة المصدرة",
    publishedNotAttested: "قاعدة منشورة دون شهادة",
    callerPreview: "معاينة مقدمة من المستدعي",
    checkRecord: "التحقق من السجل المحفوظ",
    checkingRecord: "جارٍ التحقق من السجل…",
    recordChecked: "تم التحقق من السجل",
    evidenceNote: "يتحقق هذا من الحقائق المحفوظة والقاعدة المعتمدة وإصدار المحرك والتفصيل الكامل للمكونات. لا يوافق على الأموال ولا ينقلها.",
    checkStatus: "حالة التحقق",
    factsDigest: "ملخص الحقائق",
    ruleDigest: "ملخص القاعدة",
  },
};

const JAPANESE: QuoteWorkspaceMessages = {
  quoteMode: {
    eyebrow: "顧客向け見積もり",
    title: "税額見積もりを作成",
    status: "確認の準備完了",
    description: "承認済みの税ルールから見積もりを作成します。アカウントと見積もりの詳細はサービスで確認され、後で再確認できます。",
    submit: "見積もりを準備",
    result: {
      whyTax: "なぜこの税額ですか？",
      explanation: "サービスは入力された所在地と金額に承認済み税ルールを適用しました。この説明は結果の確認用であり、税務助言、申告、コンプライアンス保証ではありません。",
      ruleLabel: "承認済み税ルール",
      emptyCopy: "見積もりを準備すると、税額と説明を確認できます。",
    },
  },
  previewMode: {
    eyebrow: "シナリオを試す",
    title: "税額シナリオを確認",
    status: "本番変更なし",
    description: "仮のシナリオを確認します。公開済みのルールを変更したり、支払いを承認したりすることはありません。",
    submit: "シナリオを計算",
    result: {
      whyTax: "このシナリオはどのように計算されましたか？",
      explanation: "サービスは入力された金額と税率から、この読み取り専用シナリオを計算しました。公開済みの税ルールは使用も変更もしません。この説明は結果の確認用であり、税務助言、申告、コンプライアンス保証ではありません。",
      ruleLabel: "シナリオルール",
      emptyCopy: "シナリオを確認すると、税額と説明を確認できます。",
    },
  },
  readOnlyWarning: "安全なサインインが設定されるまで、このワークスペースは読み取り専用です。見積もりは送信されません。",
  location: {
    summary: "所在地の確認",
    description: "本番の見積もりには、承認済みの所在地プロバイダーの記録が必要です。この高度な交換では、サーバー検証のため署名済み記録をそのまま保持します。",
    label: "プロバイダー署名済み所在地記録（高度）",
    placeholder: "承認済みプロバイダーの署名済み記録を貼り付け",
    help: "本番の見積もりに必要です。サービスは記録を使用する前に、ソース、アカウント、日付、所在地を確認します。",
  },
  fields: {
    grossLabel: "税引前合計",
    grossPlaceholder: "合計金額を入力",
    netLabel: "課税対象額",
    netPlaceholder: "課税対象額を入力",
    currencyLabel: "通貨",
    currencyPlaceholder: "通貨コードを入力",
    rateLabel: "税率（%）",
    ratePlaceholder: "税率を入力",
    approvedRuleLabel: "承認済み税ルール",
    approvedRulePlaceholder: "承認済みルールの参照を入力",
    jurisdictionLabel: "管轄",
    jurisdictionPlaceholder: "管轄を入力",
    ruleVersionLabel: "ルールバージョン",
    ruleVersionPlaceholder: "ルールバージョンを入力",
    categoryLabel: "税区分",
    categoryPlaceholder: "税区分を入力",
  },
  retry: {
    title: "再試行保護を自動選択",
    description: "安全な再試行参照が自動的に割り当てられ、見積もりの詳細が変わらない限り同じ値が保持されます。",
  },
  amountHelp: "サポートされる最小のアカウント単位で金額を入力してください。サービスは正確に計算し、不足値を推測しません。",
  offlineWarning: "サービスを利用できません。クライアント側の代替処理は使用しません。",
  preparing: "準備中…",
  result: {
    eyebrow: "税額の内訳",
    title: "結果を確認",
    verified: "確認済み",
    needsVerification: "確認が必要",
    scenarioOnly: "シナリオのみ",
    whyTax: "なぜこの税額ですか？",
    explanation: "サービスは入力された所在地と金額に承認済み税ルールを適用しました。この説明は結果の確認用であり、税務助言、申告、コンプライアンス保証ではありません。",
    approvedRule: "承認済み税ルール",
    currencyConversion: "通貨換算",
    recordFingerprints: "記録フィンガープリント",
    ruleFingerprint: "ルールフィンガープリント",
    inputFingerprint: "入力フィンガープリント",
    emptyTitle: "結果はここに表示されます",
    emptyCopy: "見積もりを準備するかシナリオを確認すると、税額と説明を確認できます。",
    totalTax: "合計税額",
    reference: "参照",
    jurisdiction: "管轄",
    amountAfterTax: "税引後金額",
    resultStatus: "結果ステータス",
    verifiedStatus: "承認済み証拠に対して確認済み",
    calculatedStatus: "計算済み。確認が必要です",
    scenarioStatus: "確認用のシナリオ",
    technicalDetails: "技術・監査の詳細",
    calculationReference: "計算参照",
    engineVersion: "エンジンバージョン",
    legacyEvidence: "レガシー証拠",
    policyState: "ポリシー状態",
    publishedAttestation: "公開済みルールと発行者の証明",
    publishedNotAttested: "公開済みルール、証明なし",
    callerPreview: "呼び出し元が提供したプレビュー",
    checkRecord: "保存済み記録を確認",
    checkingRecord: "記録を確認中…",
    recordChecked: "記録を確認済み",
    evidenceNote: "保存済みの事実、承認済みルール、エンジンバージョン、全コンポーネントの内訳を確認します。資金の承認や移動は行いません。",
    checkStatus: "確認ステータス",
    factsDigest: "事実ダイジェスト",
    ruleDigest: "ルールダイジェスト",
  },
};

const CATALOGS: Readonly<Partial<Record<SupportedLocale, QuoteWorkspaceMessages>>> = Object.freeze({
  "en-US": ENGLISH,
  "de-DE": GERMAN,
  "ar-EG": ARABIC,
  "ja-JP": JAPANESE,
});

export function quoteMessagesFor(locale: SupportedLocale | string): QuoteWorkspaceMessages {
  return CATALOGS[locale as SupportedLocale] ?? ENGLISH;
}
