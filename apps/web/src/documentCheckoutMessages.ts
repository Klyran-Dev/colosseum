import { SupportedLocale } from "./formatters";

export interface DocumentCheckoutMessages {
  evidence: {
    eyebrow: string;
    title: string;
    status: string;
    description: string;
    walletStep: string;
    walletConnected: string;
    walletMissing: string;
    connectWallet: string;
    invoiceReference: string;
    paymentRoute: string;
    loadingRoutes: string;
    routesUnavailable: string;
    selectRoute: string;
    noRoutes: string;
    verifiedRoute: string;
    paymentAsset: string;
    recipientProtection: string;
    checkedTitle: string;
    pendingTitle: string;
    checkedCopy: string;
    pendingCopy: string;
    recordDetails: string;
    checkedAt: string;
    expires: string;
    evidenceFingerprint: string;
    issuerKey: string;
    check: string;
    checking: string;
    checked: string;
    verifiedNotSent: string;
    evidenceRecord: string;
    signingAuthority: string;
    expired: string;
    checkedByService: string;
  };
  review: {
    eyebrow: string;
    title: string;
    status: string;
    paymentScope: string;
    fullDocument: string;
    lineSubset: string;
    invoiceLines: string;
    lineHelp: string;
    safeguardsTitle: string;
    safeguardsCopy: string;
    prepare: string;
    preparing: string;
    emptyTitle: string;
    emptyCopy: string;
    amountBeforeTax: string;
    taxWithheld: string;
    amountSent: string;
    paymentAsset: string;
    selectedByRoute: string;
    paymentDetails: string;
    verifiedInvoiceEvidence: string;
    paymentReference: string;
    readyTitle: string;
    readyCopy: string;
    approve: string;
    waitingWallet: string;
    sendingPayment: string;
    paymentStatus: string;
    noChainTransaction: string;
    refreshStatus: string;
    checkRecord: string;
  };
  status: {
    walletBoundary: string;
    legalBoundary: string;
    transactionReference: (signature: string) => string;
    expiresChecked: (expired: boolean) => string;
  };
  errors: DocumentCheckoutErrors;
}

export interface DocumentCheckoutErrors {
  serviceUnavailable: string;
  authorizationRequired: string;
  inputRequired: string;
  routeRequestFailed: (status: number) => string;
  evidenceCheckFailed: (status: number) => string;
  evidenceCheckUnavailable: string;
  invoiceEvidenceRequired: string;
  walletAndRouteRequired: string;
  prepareRequired: string;
  attestationExpired: string;
  idempotencyRequired: string;
  invalidFields: string;
  compileRequestFailed: (status: number) => string;
  compileFailed: string;
  intentRequired: string;
  signatureExpired: string;
  submissionRequestFailed: (status: number) => string;
  intentRecovery: (status: string) => string;
  submissionUnknown: string;
  signingFailed: string;
  statusRequestFailed: (status: number) => string;
  statusRefreshFailed: string;
  reconciliationRequestFailed: (status: number) => string;
  reconciliationFailed: string;
}

function displayHttpStatus(status: number): string {
  return Number.isInteger(status) && status >= 100 && status <= 599 ? String(status) : "unknown status";
}

function displayIntentStatus(status: string): string {
  return /^[a-z][a-z0-9_]{0,31}$/i.test(status) ? status : "unknown";
}

const ENGLISH_ERRORS: DocumentCheckoutErrors = {
  serviceUnavailable: "The payment service is unavailable; no browser-side verification is used.",
  authorizationRequired: "Sign in with an account allowed to prepare this payment.",
  inputRequired: "Enter the document calculation ID and select an active route.",
  routeRequestFailed: (status) => `Payment routes could not be loaded (${displayHttpStatus(status)}).`,
  evidenceCheckFailed: (status) => `Invoice evidence could not be checked (${displayHttpStatus(status)}).`,
  evidenceCheckUnavailable: "Invoice evidence could not be checked.",
  invoiceEvidenceRequired: "Verify the invoice evidence before preparing payment.",
  walletAndRouteRequired: "Connect the payer wallet and select an active route.",
  prepareRequired: "Prepare the durable payment review and connect the payer wallet first.",
  attestationExpired: "The document attestation expired; verify a new signed attestation.",
  idempotencyRequired: "A payment reference is required for durable document preparation.",
  invalidFields: "Document payment fields are invalid.",
  compileRequestFailed: (status) => `Document payment preparation failed (${displayHttpStatus(status)}).`,
  compileFailed: "Document payment preparation failed.",
  intentRequired: "The service did not return a durable payment intent; signing is disabled.",
  signatureExpired: "The document attestation expired; no wallet signature was requested.",
  submissionRequestFailed: (status) => `Document payment submission failed (${displayHttpStatus(status)}).`,
  intentRecovery: (status) => `The submission is recorded as ${displayIntentStatus(status)}; do not sign again until the service permits retry.`,
  submissionUnknown: "The submission outcome is unknown. Do not sign again; recover this payment through the authenticated status path.",
  signingFailed: "Wallet signing or document payment submission failed.",
  statusRequestFailed: (status) => `Payment status could not be refreshed (${displayHttpStatus(status)}).`,
  statusRefreshFailed: "Payment status could not be refreshed.",
  reconciliationRequestFailed: (status) => `Payment reconciliation failed (${displayHttpStatus(status)}).`,
  reconciliationFailed: "Payment reconciliation failed.",
};

const GERMAN_ERRORS: DocumentCheckoutErrors = {
  serviceUnavailable: "Der Zahlungsdienst ist nicht verfügbar; es wird keine Prüfung im Browser verwendet.",
  authorizationRequired: "Melden Sie sich mit einem Konto an, das diese Zahlung vorbereiten darf.",
  inputRequired: "Geben Sie die Dokumentberechnungs-ID ein und wählen Sie eine aktive Route.",
  routeRequestFailed: (status) => `Zahlungsrouten konnten nicht geladen werden (${displayHttpStatus(status)}).`,
  evidenceCheckFailed: (status) => `Der Rechnungsnachweis konnte nicht geprüft werden (${displayHttpStatus(status)}).`,
  evidenceCheckUnavailable: "Der Rechnungsnachweis konnte nicht geprüft werden.",
  invoiceEvidenceRequired: "Prüfen Sie den Rechnungsnachweis, bevor Sie die Zahlung vorbereiten.",
  walletAndRouteRequired: "Verbinden Sie das Zahler-Wallet und wählen Sie eine aktive Route.",
  prepareRequired: "Bereiten Sie die dauerhafte Zahlungsprüfung vor und verbinden Sie zuerst das Zahler-Wallet.",
  attestationExpired: "Die Dokumentattestierung ist abgelaufen; prüfen Sie eine neue signierte Attestierung.",
  idempotencyRequired: "Für die dauerhafte Zahlungsvorbereitung ist eine Zahlungsreferenz erforderlich.",
  invalidFields: "Die Felder der Dokumentzahlung sind ungültig.",
  compileRequestFailed: (status) => `Die Dokumentzahlung konnte nicht vorbereitet werden (${displayHttpStatus(status)}).`,
  compileFailed: "Die Dokumentzahlung konnte nicht vorbereitet werden.",
  intentRequired: "Der Dienst lieferte keine dauerhafte Zahlungsabsicht; Signieren ist deaktiviert.",
  signatureExpired: "Die Dokumentattestierung ist abgelaufen; es wurde keine Wallet-Signatur angefordert.",
  submissionRequestFailed: (status) => `Die Dokumentzahlung konnte nicht gesendet werden (${displayHttpStatus(status)}).`,
  intentRecovery: (status) => `Die Übermittlung ist als ${displayIntentStatus(status)} gespeichert; signieren Sie erst wieder, wenn eine Wiederholung erlaubt ist.`,
  submissionUnknown: "Das Ergebnis der Übermittlung ist unbekannt. Signieren Sie nicht erneut; stellen Sie die Zahlung über den authentifizierten Statuspfad wieder her.",
  signingFailed: "Wallet-Signatur oder Übermittlung der Dokumentzahlung fehlgeschlagen.",
  statusRequestFailed: (status) => `Der Zahlungsstatus konnte nicht aktualisiert werden (${displayHttpStatus(status)}).`,
  statusRefreshFailed: "Der Zahlungsstatus konnte nicht aktualisiert werden.",
  reconciliationRequestFailed: (status) => `Die Zahlungsabstimmung ist fehlgeschlagen (${displayHttpStatus(status)}).`,
  reconciliationFailed: "Die Zahlungsabstimmung ist fehlgeschlagen.",
};

const ARABIC_ERRORS: DocumentCheckoutErrors = {
  serviceUnavailable: "خدمة الدفع غير متاحة؛ لا يتم استخدام التحقق داخل المتصفح.",
  authorizationRequired: "سجّل الدخول بحساب مسموح له بإعداد هذه الدفعة.",
  inputRequired: "أدخل معرّف حساب المستند واختر مساراً نشطاً.",
  routeRequestFailed: (status) => `تعذر تحميل مسارات الدفع (${displayHttpStatus(status)}).`,
  evidenceCheckFailed: (status) => `تعذر فحص دليل الفاتورة (${displayHttpStatus(status)}).`,
  evidenceCheckUnavailable: "تعذر فحص دليل الفاتورة.",
  invoiceEvidenceRequired: "تحقق من دليل الفاتورة قبل إعداد الدفعة.",
  walletAndRouteRequired: "صِل محفظة الدافع واختر مساراً نشطاً.",
  prepareRequired: "أعد مراجعة الدفعة الدائمة وصِل محفظة الدافع أولاً.",
  attestationExpired: "انتهت صلاحية شهادة المستند؛ تحقق من شهادة موقعة جديدة.",
  idempotencyRequired: "مرجع الدفع مطلوب لإعداد المستند بشكل دائم.",
  invalidFields: "حقول دفعة المستند غير صالحة.",
  compileRequestFailed: (status) => `تعذر إعداد دفعة المستند (${displayHttpStatus(status)}).`,
  compileFailed: "تعذر إعداد دفعة المستند.",
  intentRequired: "لم تُرجع الخدمة نية دفع دائمة؛ تم تعطيل التوقيع.",
  signatureExpired: "انتهت صلاحية شهادة المستند؛ لم يُطلب توقيع المحفظة.",
  submissionRequestFailed: (status) => `تعذر إرسال دفعة المستند (${displayHttpStatus(status)}).`,
  intentRecovery: (status) => `تم تسجيل الإرسال كـ ${displayIntentStatus(status)}؛ لا توقّع مجدداً حتى تسمح الخدمة بإعادة المحاولة.`,
  submissionUnknown: "نتيجة الإرسال غير معروفة. لا توقّع مجدداً؛ استعد الدفعة عبر مسار الحالة الموثّق.",
  signingFailed: "فشل توقيع المحفظة أو إرسال دفعة المستند.",
  statusRequestFailed: (status) => `تعذر تحديث حالة الدفع (${displayHttpStatus(status)}).`,
  statusRefreshFailed: "تعذر تحديث حالة الدفع.",
  reconciliationRequestFailed: (status) => `فشلت مطابقة الدفع (${displayHttpStatus(status)}).`,
  reconciliationFailed: "فشلت مطابقة الدفع.",
};

const JAPANESE_ERRORS: DocumentCheckoutErrors = {
  serviceUnavailable: "支払いサービスを利用できません。ブラウザー内の検証は使用しません。",
  authorizationRequired: "この支払いを準備できるアカウントでサインインしてください。",
  inputRequired: "ドキュメント計算IDを入力し、有効なルートを選択してください。",
  routeRequestFailed: (status) => `支払いルートを読み込めませんでした（${displayHttpStatus(status)}）。`,
  evidenceCheckFailed: (status) => `請求書証拠を確認できませんでした（${displayHttpStatus(status)}）。`,
  evidenceCheckUnavailable: "請求書証拠を確認できませんでした。",
  invoiceEvidenceRequired: "支払いを準備する前に請求書証拠を確認してください。",
  walletAndRouteRequired: "支払人ウォレットを接続し、有効なルートを選択してください。",
  prepareRequired: "永続的な支払い確認を準備し、先に支払人ウォレットを接続してください。",
  attestationExpired: "ドキュメント証明の有効期限が切れました。新しい署名済み証明を確認してください。",
  idempotencyRequired: "ドキュメント支払いを永続的に準備するには支払い参照が必要です。",
  invalidFields: "ドキュメント支払いの項目が無効です。",
  compileRequestFailed: (status) => `ドキュメント支払いを準備できませんでした（${displayHttpStatus(status)}）。`,
  compileFailed: "ドキュメント支払いを準備できませんでした。",
  intentRequired: "サービスが永続的な支払いインテントを返さなかったため、署名を無効にしました。",
  signatureExpired: "ドキュメント証明の有効期限が切れたため、ウォレット署名は要求されませんでした。",
  submissionRequestFailed: (status) => `ドキュメント支払いを送信できませんでした（${displayHttpStatus(status)}）。`,
  intentRecovery: (status) => `送信は${displayIntentStatus(status)}として記録されています。サービスが再試行を許可するまで再署名しないでください。`,
  submissionUnknown: "送信結果が不明です。再署名せず、認証済みステータスから支払いを復旧してください。",
  signingFailed: "ウォレット署名またはドキュメント支払いの送信に失敗しました。",
  statusRequestFailed: (status) => `支払い状態を更新できませんでした（${displayHttpStatus(status)}）。`,
  statusRefreshFailed: "支払い状態を更新できませんでした。",
  reconciliationRequestFailed: (status) => `支払いの照合に失敗しました（${displayHttpStatus(status)}）。`,
  reconciliationFailed: "支払いの照合に失敗しました。",
};

const ENGLISH: DocumentCheckoutMessages = {
  evidence: {
    eyebrow: "Invoice payment",
    title: "Review and authorize this payment",
    status: "Evidence required",
    description: "Review the verified invoice and authorize the wallet payment. The service owns the totals, tax, recipient, and payment route; your wallet remains the only signer.",
    walletStep: "Your wallet",
    walletConnected: "Wallet connected",
    walletMissing: "No wallet connected",
    connectWallet: "Connect wallet",
    invoiceReference: "Invoice reference",
    paymentRoute: "Payment route",
    loadingRoutes: "Loading checked routes…",
    routesUnavailable: "Routes unavailable",
    selectRoute: "Select a payment route",
    noRoutes: "No active routes",
    verifiedRoute: "Verified payment route selected",
    paymentAsset: "Payment asset",
    recipientProtection: "Recipient and merchant accounts are protected by the service.",
    checkedTitle: "Invoice evidence checked",
    pendingTitle: "Invoice evidence will be checked by the service",
    checkedCopy: "The stored invoice record matches this payment route and is ready for amount review.",
    pendingCopy: "Enter the invoice reference. The service loads the verified record; nothing needs to be pasted here.",
    recordDetails: "Record details",
    checkedAt: "Checked",
    expires: "expires",
    evidenceFingerprint: "Evidence fingerprint",
    issuerKey: "issuer key",
    check: "Check invoice evidence",
    checking: "Checking invoice evidence…",
    checked: "Invoice evidence checked",
    verifiedNotSent: "Invoice verified; payment not sent",
    evidenceRecord: "Evidence record",
    signingAuthority: "signing authority",
    expired: "expired; verify a new record",
    checkedByService: "checked by the service",
  },
  review: {
    eyebrow: "Payment review",
    title: "Confirm amount and recipient",
    status: "Safeguards checked",
    paymentScope: "Payment scope",
    fullDocument: "Full document",
    lineSubset: "Complete line subset",
    invoiceLines: "Invoice lines to pay",
    lineHelp: "Only complete invoice lines can be selected. Request a new invoice quote for a partial line.",
    safeguardsTitle: "Payment safeguards checked automatically",
    safeguardsCopy: "The verified invoice supplies the tax and payment rules. A secure retry reference is assigned; no protocol values need to be entered.",
    prepare: "Prepare payment for review",
    preparing: "Preparing payment review…",
    emptyTitle: "Verify the invoice before preparing payment",
    emptyCopy: "Once the service confirms the invoice evidence, you can review the amount and recipient before your wallet approves the payment.",
    amountBeforeTax: "Amount before tax",
    taxWithheld: "Tax withheld",
    amountSent: "Amount sent to recipient",
    paymentAsset: "Payment asset",
    selectedByRoute: "Selected by payment route",
    paymentDetails: "Payment details",
    verifiedInvoiceEvidence: "Verified invoice evidence",
    paymentReference: "Payment reference",
    readyTitle: "Payment ready for your approval",
    readyCopy: "Payment safeguards confirmed from the verified invoice.",
    approve: "Approve and send payment",
    waitingWallet: "Waiting for wallet…",
    sendingPayment: "Sending payment…",
    paymentStatus: "Payment status",
    noChainTransaction: "No chain transaction is recorded yet.",
    refreshStatus: "Refresh payment status",
    checkRecord: "Check payment record",
  },
  status: {
    walletBoundary: "Your wallet signature is not proof the payment settled. Only finalized network evidence confirms movement.",
    legalBoundary: "This is withholding authorization—not tax filing, reporting, legal compliance, or tax advice.",
    transactionReference: (signature) => `Transaction reference ${signature}`,
    expiresChecked: (expired) => expired ? "expired; verify a new record" : "checked by the service",
  },
  errors: ENGLISH_ERRORS,
};

const GERMAN: DocumentCheckoutMessages = {
  evidence: {
    eyebrow: "Rechnungszahlung",
    title: "Diese Zahlung prüfen und autorisieren",
    status: "Nachweis erforderlich",
    description: "Prüfen Sie die verifizierte Rechnung und autorisieren Sie die Zahlung mit dem Wallet. Der Dienst besitzt Summen, Steuer, Empfänger und Zahlungsroute; nur Ihr Wallet signiert.",
    walletStep: "Ihr Wallet",
    walletConnected: "Wallet verbunden",
    walletMissing: "Kein Wallet verbunden",
    connectWallet: "Wallet verbinden",
    invoiceReference: "Rechnungsreferenz",
    paymentRoute: "Zahlungsroute",
    loadingRoutes: "Geprüfte Routen werden geladen…",
    routesUnavailable: "Routen nicht verfügbar",
    selectRoute: "Zahlungsroute auswählen",
    noRoutes: "Keine aktiven Routen",
    verifiedRoute: "Verifizierte Zahlungsroute ausgewählt",
    paymentAsset: "Zahlungsasset",
    recipientProtection: "Empfänger- und Händlerkonten werden vom Dienst geschützt.",
    checkedTitle: "Rechnungsnachweis geprüft",
    pendingTitle: "Der Dienst prüft den Rechnungsnachweis",
    checkedCopy: "Der gespeicherte Rechnungsdatensatz passt zu dieser Zahlungsroute und ist zur Betragsprüfung bereit.",
    pendingCopy: "Geben Sie die Rechnungsreferenz ein. Der Dienst lädt den geprüften Datensatz; nichts muss eingefügt werden.",
    recordDetails: "Datensatzdetails",
    checkedAt: "Geprüft",
    expires: "läuft ab",
    evidenceFingerprint: "Nachweis-Fingerabdruck",
    issuerKey: "Ausstellerschlüssel",
    check: "Rechnungsnachweis prüfen",
    checking: "Rechnungsnachweis wird geprüft…",
    checked: "Rechnungsnachweis geprüft",
    verifiedNotSent: "Rechnung verifiziert; Zahlung nicht gesendet",
    evidenceRecord: "Nachweisdatensatz",
    signingAuthority: "Signaturinstanz",
    expired: "abgelaufen; neuen Datensatz prüfen",
    checkedByService: "vom Dienst geprüft",
  },
  review: {
    eyebrow: "Zahlungsprüfung",
    title: "Betrag und Empfänger bestätigen",
    status: "Schutzmaßnahmen geprüft",
    paymentScope: "Zahlungsumfang",
    fullDocument: "Gesamtes Dokument",
    lineSubset: "Vollständige Positionsauswahl",
    invoiceLines: "Zu zahlende Rechnungspositionen",
    lineHelp: "Nur vollständige Rechnungspositionen können ausgewählt werden. Für eine Teilposition ein neues Rechnungsangebot anfordern.",
    safeguardsTitle: "Zahlungsschutz automatisch geprüft",
    safeguardsCopy: "Die geprüfte Rechnung liefert Steuer- und Zahlungsregeln. Eine sichere Wiederholungsreferenz wird vergeben; keine Protokollwerte müssen eingegeben werden.",
    prepare: "Zahlung zur Prüfung vorbereiten",
    preparing: "Zahlungsprüfung wird vorbereitet…",
    emptyTitle: "Rechnung vor der Zahlungsvorbereitung prüfen",
    emptyCopy: "Sobald der Dienst den Rechnungsnachweis bestätigt, können Sie Betrag und Empfänger prüfen, bevor Ihr Wallet die Zahlung genehmigt.",
    amountBeforeTax: "Betrag vor Steuer",
    taxWithheld: "Einbehaltene Steuer",
    amountSent: "An Empfänger gesendeter Betrag",
    paymentAsset: "Zahlungsasset",
    selectedByRoute: "Durch Zahlungsroute ausgewählt",
    paymentDetails: "Zahlungsdetails",
    verifiedInvoiceEvidence: "Geprüfter Rechnungsnachweis",
    paymentReference: "Zahlungsreferenz",
    readyTitle: "Zahlung zur Genehmigung bereit",
    readyCopy: "Zahlungsschutz aus der geprüften Rechnung bestätigt.",
    approve: "Zahlung genehmigen und senden",
    waitingWallet: "Warten auf Wallet…",
    sendingPayment: "Zahlung wird gesendet…",
    paymentStatus: "Zahlungsstatus",
    noChainTransaction: "Noch keine Chain-Transaktion aufgezeichnet.",
    refreshStatus: "Zahlungsstatus aktualisieren",
    checkRecord: "Zahlungsdatensatz prüfen",
  },
  status: {
    walletBoundary: "Ihre Wallet-Signatur beweist nicht, dass die Zahlung abgewickelt wurde. Nur ein finaler Netzwerknachweis bestätigt die Bewegung.",
    legalBoundary: "Dies ist eine Genehmigung zum Einbehalt – keine Steuererklärung, Meldung, rechtliche Compliance oder Steuerberatung.",
    transactionReference: (signature) => `Transaktionsreferenz ${signature}`,
    expiresChecked: (expired) => expired ? "abgelaufen; neuen Datensatz prüfen" : "vom Dienst geprüft",
  },
  errors: GERMAN_ERRORS,
};

const ARABIC: DocumentCheckoutMessages = {
  evidence: {
    eyebrow: "دفع الفاتورة",
    title: "مراجعة هذه الدفعة وتفويضها",
    status: "الدليل مطلوب",
    description: "راجع الفاتورة المتحقق منها وفوّض دفعة المحفظة. تملك الخدمة الإجماليات والضريبة والمستلم ومسار الدفع؛ وتبقى محفظتك الموقّع الوحيد.",
    walletStep: "محفظتك",
    walletConnected: "المحفظة متصلة",
    walletMissing: "لا توجد محفظة متصلة",
    connectWallet: "توصيل المحفظة",
    invoiceReference: "مرجع الفاتورة",
    paymentRoute: "مسار الدفع",
    loadingRoutes: "جارٍ تحميل المسارات المتحققة…",
    routesUnavailable: "المسارات غير متاحة",
    selectRoute: "اختر مسار دفع",
    noRoutes: "لا توجد مسارات نشطة",
    verifiedRoute: "تم اختيار مسار دفع متحقق",
    paymentAsset: "أصل الدفع",
    recipientProtection: "تحمي الخدمة حسابات المستلم والتاجر.",
    checkedTitle: "تم فحص دليل الفاتورة",
    pendingTitle: "سيتم فحص دليل الفاتورة بواسطة الخدمة",
    checkedCopy: "يطابق سجل الفاتورة المحفوظ مسار الدفع هذا وهو جاهز لمراجعة المبلغ.",
    pendingCopy: "أدخل مرجع الفاتورة. تحمّل الخدمة السجل المتحقق؛ لا حاجة للصق أي شيء.",
    recordDetails: "تفاصيل السجل",
    checkedAt: "تم الفحص",
    expires: "ينتهي",
    evidenceFingerprint: "بصمة الدليل",
    issuerKey: "مفتاح الجهة المصدرة",
    check: "فحص دليل الفاتورة",
    checking: "جارٍ فحص دليل الفاتورة…",
    checked: "تم فحص دليل الفاتورة",
    verifiedNotSent: "تم التحقق من الفاتورة؛ لم تُرسل الدفعة",
    evidenceRecord: "سجل الدليل",
    signingAuthority: "جهة التوقيع",
    expired: "منتهي؛ تحقق من سجل جديد",
    checkedByService: "تم فحصه بواسطة الخدمة",
  },
  review: {
    eyebrow: "مراجعة الدفع",
    title: "تأكيد المبلغ والمستلم",
    status: "تم فحص الضمانات",
    paymentScope: "نطاق الدفع",
    fullDocument: "المستند بالكامل",
    lineSubset: "مجموعة بنود كاملة",
    invoiceLines: "بنود الفاتورة للدفع",
    lineHelp: "يمكن اختيار بنود الفاتورة الكاملة فقط. اطلب عرض فاتورة جديداً لبند جزئي.",
    safeguardsTitle: "تم فحص ضمانات الدفع تلقائياً",
    safeguardsCopy: "توفر الفاتورة المتحققة قواعد الضريبة والدفع. يتم تعيين مرجع آمن لإعادة المحاولة؛ لا حاجة لإدخال قيم البروتوكول.",
    prepare: "إعداد الدفعة للمراجعة",
    preparing: "جارٍ إعداد مراجعة الدفع…",
    emptyTitle: "تحقق من الفاتورة قبل إعداد الدفعة",
    emptyCopy: "بعد تأكيد الخدمة لدليل الفاتورة، يمكنك مراجعة المبلغ والمستلم قبل موافقة محفظتك.",
    amountBeforeTax: "المبلغ قبل الضريبة",
    taxWithheld: "الضريبة المحجوزة",
    amountSent: "المبلغ المرسل إلى المستلم",
    paymentAsset: "أصل الدفع",
    selectedByRoute: "يحدده مسار الدفع",
    paymentDetails: "تفاصيل الدفع",
    verifiedInvoiceEvidence: "دليل الفاتورة المتحقق",
    paymentReference: "مرجع الدفع",
    readyTitle: "الدفعة جاهزة لموافقتك",
    readyCopy: "تم تأكيد ضمانات الدفع من الفاتورة المتحققة.",
    approve: "الموافقة على الدفعة وإرسالها",
    waitingWallet: "في انتظار المحفظة…",
    sendingPayment: "جارٍ إرسال الدفعة…",
    paymentStatus: "حالة الدفع",
    noChainTransaction: "لم يتم تسجيل معاملة على الشبكة بعد.",
    refreshStatus: "تحديث حالة الدفع",
    checkRecord: "فحص سجل الدفع",
  },
  status: {
    walletBoundary: "توقيع محفظتك لا يثبت تسوية الدفعة. يؤكد الحركةَ دليل شبكة نهائي فقط.",
    legalBoundary: "هذا تفويض لحجز الضريبة، وليس إقراراً ضريبياً أو تقريراً أو امتثالاً قانونياً أو استشارة ضريبية.",
    transactionReference: (signature) => `مرجع المعاملة ${signature}`,
    expiresChecked: (expired) => expired ? "منتهي؛ تحقق من سجل جديد" : "تم فحصه بواسطة الخدمة",
  },
  errors: ARABIC_ERRORS,
};

const JAPANESE: DocumentCheckoutMessages = {
  evidence: {
    eyebrow: "請求書の支払い",
    title: "この支払いを確認して承認",
    status: "証拠が必要です",
    description: "検証済みの請求書を確認し、ウォレット支払いを承認してください。合計、税、受取人、支払いルートはサービスが管理し、署名できるのはウォレットだけです。",
    walletStep: "ウォレット",
    walletConnected: "ウォレット接続済み",
    walletMissing: "ウォレット未接続",
    connectWallet: "ウォレットを接続",
    invoiceReference: "請求書参照",
    paymentRoute: "支払いルート",
    loadingRoutes: "検証済みルートを読み込み中…",
    routesUnavailable: "ルートを利用できません",
    selectRoute: "支払いルートを選択",
    noRoutes: "有効なルートがありません",
    verifiedRoute: "検証済み支払いルートを選択済み",
    paymentAsset: "支払いアセット",
    recipientProtection: "受取人と加盟店のアカウントはサービスで保護されます。",
    checkedTitle: "請求書証拠を確認済み",
    pendingTitle: "サービスが請求書証拠を確認します",
    checkedCopy: "保存済み請求書記録はこの支払いルートと一致し、金額確認の準備ができています。",
    pendingCopy: "請求書参照を入力してください。検証済み記録はサービスが読み込み、貼り付けは不要です。",
    recordDetails: "記録の詳細",
    checkedAt: "確認日時",
    expires: "有効期限",
    evidenceFingerprint: "証拠フィンガープリント",
    issuerKey: "発行者キー",
    check: "請求書証拠を確認",
    checking: "請求書証拠を確認中…",
    checked: "請求書証拠を確認済み",
    verifiedNotSent: "請求書を検証済み。支払いは未送信",
    evidenceRecord: "証拠記録",
    signingAuthority: "署名権限",
    expired: "期限切れ。新しい記録を確認してください",
    checkedByService: "サービスが確認済み",
  },
  review: {
    eyebrow: "支払い確認",
    title: "金額と受取人を確認",
    status: "保護策を確認済み",
    paymentScope: "支払い範囲",
    fullDocument: "ドキュメント全体",
    lineSubset: "完全な明細範囲",
    invoiceLines: "支払う請求書明細",
    lineHelp: "完全な請求書明細だけを選択できます。部分的な明細には新しい請求書見積もりを依頼してください。",
    safeguardsTitle: "支払い保護策を自動確認済み",
    safeguardsCopy: "検証済み請求書が税と支払いのルールを提供します。安全な再試行参照が割り当てられ、プロトコル値の入力は不要です。",
    prepare: "支払いを確認用に準備",
    preparing: "支払い確認を準備中…",
    emptyTitle: "支払い準備前に請求書を確認",
    emptyCopy: "サービスが請求書証拠を確認すると、ウォレットが支払いを承認する前に金額と受取人を確認できます。",
    amountBeforeTax: "税引前金額",
    taxWithheld: "源泉徴収税額",
    amountSent: "受取人に送る金額",
    paymentAsset: "支払いアセット",
    selectedByRoute: "支払いルートで選択",
    paymentDetails: "支払いの詳細",
    verifiedInvoiceEvidence: "検証済み請求書証拠",
    paymentReference: "支払い参照",
    readyTitle: "支払いは承認の準備完了",
    readyCopy: "検証済み請求書から支払い保護策を確認しました。",
    approve: "支払いを承認して送信",
    waitingWallet: "ウォレットを待機中…",
    sendingPayment: "支払いを送信中…",
    paymentStatus: "支払い状態",
    noChainTransaction: "チェーン取引はまだ記録されていません。",
    refreshStatus: "支払い状態を更新",
    checkRecord: "支払い記録を確認",
  },
  status: {
    walletBoundary: "ウォレット署名は支払いが決済された証明ではありません。移動を確認できるのは最終化済みネットワーク証拠だけです。",
    legalBoundary: "これは源泉徴収の承認であり、税務申告、報告、法的コンプライアンス、税務助言ではありません。",
    transactionReference: (signature) => `取引参照 ${signature}`,
    expiresChecked: (expired) => expired ? "期限切れ。新しい記録を確認してください" : "サービスが確認済み",
  },
  errors: JAPANESE_ERRORS,
};

const CATALOGS: Readonly<Record<SupportedLocale, DocumentCheckoutMessages>> = {
  "en-US": ENGLISH,
  "de-DE": GERMAN,
  "ar-EG": ARABIC,
  "ja-JP": JAPANESE,
};

export function documentCheckoutMessagesFor(locale: string): DocumentCheckoutMessages {
  return CATALOGS[locale as SupportedLocale] ?? ENGLISH;
}
