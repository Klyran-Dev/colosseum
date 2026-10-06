import { SupportedLocale } from "./formatters";

export interface CheckoutMessages {
  review: {
    eyebrow: string;
    title: string;
    status: string;
    description: string;
    walletStep: string;
    walletConnected: string;
    walletMissing: string;
    emptyTitle: string;
    emptyCopy: string;
    total: string;
    taxWithheld: string;
    amountSent: string;
    paymentAsset: string;
    taxRule: string;
    taxStatus: string;
    calculatedNotAttested: string;
    paymentReference: string;
    beforeSignEyebrow: string;
    beforeSignTitle: string;
    readyStatus: string;
    quoteExpires: string;
    expired: string;
    serverEnforced: string;
    walletBoundary: string;
    approve: string;
    prepare: string;
    preparing: string;
    signing: string;
    submitting: string;
    paymentStatus: (status: string) => string;
    networkReference: (signature: string) => string;
    noNetworkReference: string;
    submittingNotice: string;
    unknownNotice: string;
    refreshStatus: string;
    reconcile: string;
  };
  route: {
    paymentRoute: string;
    loading: string;
    unavailable: string;
    select: string;
    none: string;
    checkedTitle: string;
    checkedCopy: string;
    technicalLabel: string;
    technicalDetails: string;
    payerSource: string;
    version: (version: number) => string;
  };
  split: {
    sharing: string;
    oneDestination: string;
    recipientPlan: string;
    approvedPlan: string;
    loading: string;
    unavailable: string;
    select: string;
    none: string;
    matches: string;
    mismatch: string;
    version: (version: number) => string;
    recipients: (count: number) => string;
    details: string;
    chooseRoute: string;
    sharesBound: string;
    sharesCopy: string;
    recipientDetails: string;
    planReference: string;
    retry: string;
    basisPoints: string;
    distributionReview: string;
  };
  safeguards: {
    title: string;
    copy: string;
  };
  technical: {
    details: string;
    programId: string;
    policyAuthority: string;
    computePolicy: string;
    priorityFee: string;
    accounts: string;
    paymentRequest: string;
    ruleReference: string;
    computeUnits: (value: string) => string;
    microLamports: (value: string) => string;
    exactAccountEntries: (value: number) => string;
  };
  status: {
    walletBoundary: string;
    legalBoundary: string;
  };
  errors: CheckoutErrors;
}

export interface CheckoutErrors {
  serviceUnavailable: string;
  authorizationRequired: string;
  quoteRequired: string;
  quoteExpired: string;
  walletRequired: string;
  routeRequired: string;
  splitPlanRequired: string;
  splitPlanMismatch: string;
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
  routeRequestFailed: (status: number) => string;
  splitPlanRequestFailed: (status: number) => string;
}

function displayHttpStatus(status: number): string {
  return Number.isInteger(status) && status >= 100 && status <= 599 ? String(status) : "unknown status";
}

function displayIntentStatus(status: string): string {
  return /^[a-z][a-z0-9_]{0,31}$/i.test(status) ? status : "unknown";
}

const ENGLISH_ERRORS: CheckoutErrors = {
  serviceUnavailable: "The service is unavailable; no client-side fallback is used.",
  authorizationRequired: "An access token with the required settlement permissions is needed.",
  quoteRequired: "Create an authenticated published-rule quote before preparing settlement.",
  quoteExpired: "This quote has expired. Create a new quote before preparing settlement.",
  walletRequired: "Connect the payer wallet before compiling the signed transaction.",
  routeRequired: "Select an active settlement route.",
  splitPlanRequired: "Select an active governed Split Plan before compiling.",
  splitPlanMismatch: "The selected Split Plan is not compatible with the selected settlement route.",
  invalidFields: "Settlement fields are invalid.",
  compileRequestFailed: (status) => `Settlement compilation request failed (${displayHttpStatus(status)}).`,
  compileFailed: "Settlement compilation could not be completed.",
  intentRequired: "Compile a durable intent and connect the payer wallet first.",
  signatureExpired: "This quote has expired. Create a new quote; no wallet signature was requested.",
  submissionRequestFailed: (status) => `Settlement submission request failed (${displayHttpStatus(status)}).`,
  intentRecovery: (status) => `The submission request did not complete. The durable intent is ${displayIntentStatus(status)}; do not authorize a second transaction until the server permits retry.`,
  submissionUnknown: "The submission outcome is unknown. Do not sign or submit again; recover this durable intent through the authenticated status path.",
  signingFailed: "Wallet signing or settlement submission failed.",
  statusRequestFailed: (status) => `Status request failed (${displayHttpStatus(status)}).`,
  statusRefreshFailed: "Payment status could not be refreshed.",
  reconciliationRequestFailed: (status) => `Reconciliation request failed (${displayHttpStatus(status)}).`,
  reconciliationFailed: "Payment reconciliation could not be completed.",
  routeRequestFailed: (status) => `Payment route request failed (${displayHttpStatus(status)}).`,
  splitPlanRequestFailed: (status) => `Recipient plan request failed (${displayHttpStatus(status)}).`,
};

const GERMAN_ERRORS: CheckoutErrors = {
  serviceUnavailable: "Der Dienst ist nicht verfügbar; es wird kein Fallback im Browser verwendet.",
  authorizationRequired: "Ein Zugriffsmittel mit den erforderlichen Zahlungsberechtigungen ist nötig.",
  quoteRequired: "Erstellen Sie vor der Zahlungsvorbereitung ein authentifiziertes Angebot mit veröffentlichter Regel.",
  quoteExpired: "Dieses Angebot ist abgelaufen. Erstellen Sie vor der Zahlungsvorbereitung ein neues Angebot.",
  walletRequired: "Verbinden Sie vor dem Kompilieren der signierten Transaktion das Zahler-Wallet.",
  routeRequired: "Wählen Sie eine aktive Zahlungsroute.",
  splitPlanRequired: "Wählen Sie vor dem Kompilieren einen aktiven, genehmigten Split Plan.",
  splitPlanMismatch: "Der ausgewählte Split Plan ist mit dieser Zahlungsroute nicht kompatibel.",
  invalidFields: "Die Zahlungsfelder sind ungültig.",
  compileRequestFailed: (status) => `Die Zahlungszusammenstellung konnte nicht angefordert werden (${displayHttpStatus(status)}).`,
  compileFailed: "Die Zahlungszusammenstellung konnte nicht abgeschlossen werden.",
  intentRequired: "Erstellen Sie zuerst eine dauerhafte Absicht und verbinden Sie das Zahler-Wallet.",
  signatureExpired: "Dieses Angebot ist abgelaufen. Erstellen Sie ein neues Angebot; es wurde keine Wallet-Signatur angefordert.",
  submissionRequestFailed: (status) => `Die Zahlungsübermittlung konnte nicht angefordert werden (${displayHttpStatus(status)}).`,
  intentRecovery: (status) => `Die Übermittlungsanfrage wurde nicht abgeschlossen. Die dauerhafte Absicht ist ${displayIntentStatus(status)}; autorisieren Sie keine zweite Transaktion, bis der Dienst eine Wiederholung erlaubt.`,
  submissionUnknown: "Das Ergebnis der Übermittlung ist unklar. Signieren oder senden Sie nicht erneut; gleichen Sie diese dauerhafte Absicht über den authentifizierten Statusweg ab.",
  signingFailed: "Wallet-Signatur oder Zahlungsübermittlung fehlgeschlagen.",
  statusRequestFailed: (status) => `Die Statusanfrage ist fehlgeschlagen (${displayHttpStatus(status)}).`,
  statusRefreshFailed: "Der Zahlungsstatus konnte nicht aktualisiert werden.",
  reconciliationRequestFailed: (status) => `Die Abgleichanfrage ist fehlgeschlagen (${displayHttpStatus(status)}).`,
  reconciliationFailed: "Der Zahlungsabgleich konnte nicht abgeschlossen werden.",
  routeRequestFailed: (status) => `Die Zahlungsroute konnte nicht angefordert werden (${displayHttpStatus(status)}).`,
  splitPlanRequestFailed: (status) => `Der Empfängerplan konnte nicht angefordert werden (${displayHttpStatus(status)}).`,
};

const ARABIC_ERRORS: CheckoutErrors = {
  serviceUnavailable: "الخدمة غير متاحة؛ لا يتم استخدام بديل من جهة العميل.",
  authorizationRequired: "يلزم رمز وصول يملك صلاحيات التسوية المطلوبة.",
  quoteRequired: "أنشئ عرضًا موثّقًا بقاعدة منشورة قبل إعداد التسوية.",
  quoteExpired: "انتهى هذا العرض. أنشئ عرضًا جديدًا قبل إعداد التسوية.",
  walletRequired: "صل محفظة الدافع قبل تجميع المعاملة الموقعة.",
  routeRequired: "اختر مسار تسوية نشطًا.",
  splitPlanRequired: "اختر خطة توزيع نشطة ومعتمدة قبل التجميع.",
  splitPlanMismatch: "خطة التوزيع المحددة غير متوافقة مع مسار التسوية المحدد.",
  invalidFields: "حقول التسوية غير صالحة.",
  compileRequestFailed: (status) => `تعذر طلب تجميع التسوية (${displayHttpStatus(status)}).`,
  compileFailed: "تعذر إكمال تجميع التسوية.",
  intentRequired: "أنشئ نية دائمة ووصل محفظة الدافع أولًا.",
  signatureExpired: "انتهى هذا العرض. أنشئ عرضًا جديدًا؛ لم يُطلب توقيع المحفظة.",
  submissionRequestFailed: (status) => `تعذر طلب إرسال التسوية (${displayHttpStatus(status)}).`,
  intentRecovery: (status) => `لم يكتمل طلب الإرسال. حالة النية الدائمة هي ${displayIntentStatus(status)}؛ لا توافق على معاملة ثانية حتى يسمح الخادم بإعادة المحاولة.`,
  submissionUnknown: "نتيجة الإرسال غير معروفة. لا توقّع أو ترسل مرة أخرى؛ استعد هذه النية عبر مسار الحالة الموثّق.",
  signingFailed: "فشل توقيع المحفظة أو إرسال التسوية.",
  statusRequestFailed: (status) => `فشل طلب الحالة (${displayHttpStatus(status)}).`,
  statusRefreshFailed: "تعذر تحديث حالة الدفع.",
  reconciliationRequestFailed: (status) => `فشل طلب المطابقة (${displayHttpStatus(status)}).`,
  reconciliationFailed: "تعذر إكمال مطابقة الدفع.",
  routeRequestFailed: (status) => `تعذر طلب مسار الدفع (${displayHttpStatus(status)}).`,
  splitPlanRequestFailed: (status) => `تعذر طلب خطة المستلمين (${displayHttpStatus(status)}).`,
};

const JAPANESE_ERRORS: CheckoutErrors = {
  serviceUnavailable: "サービスを利用できません。クライアント側の代替処理は使用しません。",
  authorizationRequired: "必要な決済権限を持つアクセストークンが必要です。",
  quoteRequired: "決済準備の前に、認証済みで公開ルールに基づく見積もりを作成してください。",
  quoteExpired: "この見積もりは期限切れです。決済準備の前に新しい見積もりを作成してください。",
  walletRequired: "署名済みトランザクションをコンパイルする前に支払者ウォレットを接続してください。",
  routeRequired: "有効な決済ルートを選択してください。",
  splitPlanRequired: "コンパイルする前に有効な承認済み分配プランを選択してください。",
  splitPlanMismatch: "選択した分配プランは決済ルートと互換性がありません。",
  invalidFields: "決済項目が正しくありません。",
  compileRequestFailed: (status) => `決済コンパイルを要求できませんでした（${displayHttpStatus(status)}）。`,
  compileFailed: "決済コンパイルを完了できませんでした。",
  intentRequired: "最初に永続的な意図をコンパイルし、支払者ウォレットを接続してください。",
  signatureExpired: "この見積もりは期限切れです。新しい見積もりを作成してください。ウォレット署名は要求されていません。",
  submissionRequestFailed: (status) => `決済送信を要求できませんでした（${displayHttpStatus(status)}）。`,
  intentRecovery: (status) => `送信要求は完了しませんでした。永続的な意図の状態は${displayIntentStatus(status)}です。サーバーが再試行を許可するまで、2回目のトランザクションを承認しないでください。`,
  submissionUnknown: "送信結果が不明です。再度署名または送信せず、認証済みステータス経路でこの意図を復旧してください。",
  signingFailed: "ウォレット署名または決済送信に失敗しました。",
  statusRequestFailed: (status) => `ステータス要求に失敗しました（${displayHttpStatus(status)}）。`,
  statusRefreshFailed: "支払いステータスを更新できませんでした。",
  reconciliationRequestFailed: (status) => `照合要求に失敗しました（${displayHttpStatus(status)}）。`,
  reconciliationFailed: "支払いの照合を完了できませんでした。",
  routeRequestFailed: (status) => `決済ルートを要求できませんでした（${displayHttpStatus(status)}）。`,
  splitPlanRequestFailed: (status) => `受取人プランを要求できませんでした（${displayHttpStatus(status)}）。`,
};

const ENGLISH: CheckoutMessages = {
  review: {
    eyebrow: "Payment review",
    title: "Review payment before signing",
    status: "Wallet approval required",
    description: "Review the amount, tax, destination, and policy status before your wallet approves the payment. The service checks the payment again before it is sent to the network.",
    walletStep: "Wallet to use",
    walletConnected: "Wallet connected",
    walletMissing: "No wallet connected",
    emptyTitle: "Verified quote and payment preparation required",
    emptyCopy: "Generate a verified quote, then prepare the payment before the wallet approval control becomes available.",
    total: "Amount your wallet will authorize",
    taxWithheld: "Tax withheld",
    amountSent: "Amount sent",
    paymentAsset: "Payment asset",
    taxRule: "Tax rule used",
    taxStatus: "Tax status",
    calculatedNotAttested: "Calculated; not attested",
    paymentReference: "Payment reference",
    beforeSignEyebrow: "Before you sign",
    beforeSignTitle: "Payment review",
    readyStatus: "Ready for wallet approval",
    quoteExpires: "Quote expires",
    expired: "expired; create a new quote",
    serverEnforced: "server-enforced",
    walletBoundary: "Your wallet approves only the payment shown above. Wallet approval is not confirmation that the payment reached the network; use the payment status below.",
    approve: "Sign and send payment",
    prepare: "Prepare payment for review",
    preparing: "Preparing payment…",
    signing: "Waiting for wallet…",
    submitting: "Sending the reviewed payment…",
    paymentStatus: (status) => `Payment status: ${status}`,
    networkReference: (signature) => `Network reference ${signature}`,
    noNetworkReference: "No network reference is recorded yet.",
    submittingNotice: "One request is sending this payment. Do not approve it again; refresh or reconcile the recorded payment.",
    unknownNotice: "The network result is not yet clear. Do not approve another payment; reconcile this recorded payment.",
    refreshStatus: "Refresh payment status",
    reconcile: "Check network record",
  },
  route: {
    paymentRoute: "Payment route",
    loading: "Loading checked routes…",
    unavailable: "Routes unavailable",
    select: "Select a payment route",
    none: "No active routes",
    checkedTitle: "Payment route checked",
    checkedCopy: "This route was selected from your account’s active payment settings and checked before signing.",
    technicalLabel: "Technical route details",
    technicalDetails: "Technical route details",
    payerSource: "The payer source account is derived as the canonical associated token account. No account address is accepted from this browser form.",
    version: (version) => `version ${version}`,
  },
  split: {
    sharing: "How payment is shared",
    oneDestination: "One approved destination",
    recipientPlan: "Approved recipient plan",
    approvedPlan: "Approved recipient plan",
    loading: "Loading approved plans…",
    unavailable: "Plans unavailable",
    select: "Select a recipient plan",
    none: "No active recipient plans",
    matches: "Recipient plan matches the payment route",
    mismatch: "Recipient plan does not match this route",
    version: (version) => `Version ${version}`,
    recipients: (count) => `${count} approved recipients`,
    details: "Recipient account details",
    chooseRoute: "Choose a payment route for the plan’s merchant and asset.",
    sharesBound: "Recipient shares are bound to this payment",
    sharesCopy: "approved recipients · the service owns recipient and amount authority.",
    recipientDetails: "Recipient account details",
    planReference: "Plan reference",
    retry: "Retry",
    basisPoints: "basis points",
    distributionReview: "Recipient distribution review",
  },
  safeguards: {
    title: "Payment safeguards selected automatically",
    copy: "The verified quote supplies the policy. A secure payment reference is assigned for safe retry; no protocol values need to be entered.",
  },
  technical: {
    details: "Technical payment details",
    programId: "Program ID",
    policyAuthority: "Policy authority",
    computePolicy: "Compute policy",
    priorityFee: "Priority-fee bid",
    accounts: "Accounts",
    paymentRequest: "Payment request",
    ruleReference: "Rule reference",
    computeUnits: (value) => `${value} compute units`,
    microLamports: (value) => `${value} micro-lamports per compute unit`,
    exactAccountEntries: (value) => `${value} exact account entries`,
  },
  status: {
    walletBoundary: "Wallet approval is not proof that the payment settled. Only finalized network evidence confirms movement.",
    legalBoundary: "This is withholding authorization—not tax filing, reporting, legal compliance, or tax advice.",
  },
  errors: ENGLISH_ERRORS,
};

const GERMAN: CheckoutMessages = {
  review: {
    eyebrow: "Zahlungsprüfung",
    title: "Zahlung vor der Signatur prüfen",
    status: "Wallet-Bestätigung erforderlich",
    description: "Prüfen Sie Betrag, Steuer, Ziel und Richtlinienstatus, bevor Ihr Wallet die Zahlung bestätigt. Der Dienst prüft die Zahlung erneut, bevor sie an das Netzwerk gesendet wird.",
    walletStep: "Zu verwendendes Wallet",
    walletConnected: "Wallet verbunden",
    walletMissing: "Kein Wallet verbunden",
    emptyTitle: "Verifizierte Offerte und Zahlungsvorbereitung erforderlich",
    emptyCopy: "Erstellen Sie eine verifizierte Offerte und bereiten Sie die Zahlung vor, bevor die Wallet-Bestätigung verfügbar wird.",
    total: "Betrag, den Ihr Wallet bestätigt",
    taxWithheld: "Einbehaltene Steuer",
    amountSent: "Gesendeter Betrag",
    paymentAsset: "Zahlungsasset",
    taxRule: "Verwendete Steuerregel",
    taxStatus: "Steuerstatus",
    calculatedNotAttested: "Berechnet; nicht attestiert",
    paymentReference: "Zahlungsreferenz",
    beforeSignEyebrow: "Vor der Signatur",
    beforeSignTitle: "Zahlungsprüfung",
    readyStatus: "Bereit für Wallet-Bestätigung",
    quoteExpires: "Offerte läuft ab",
    expired: "abgelaufen; neue Offerte erstellen",
    serverEnforced: "serverseitig erzwungen",
    walletBoundary: "Ihr Wallet bestätigt nur die oben angezeigte Zahlung. Die Wallet-Bestätigung bestätigt nicht, dass die Zahlung das Netzwerk erreicht hat; verwenden Sie den Zahlungsstatus unten.",
    approve: "Zahlung signieren und senden",
    prepare: "Zahlung zur Prüfung vorbereiten",
    preparing: "Zahlung wird vorbereitet…",
    signing: "Warten auf Wallet…",
    submitting: "Geprüfte Zahlung wird gesendet…",
    paymentStatus: (status) => `Zahlungsstatus: ${status}`,
    networkReference: (signature) => `Netzwerkreferenz ${signature}`,
    noNetworkReference: "Noch keine Netzwerkreferenz erfasst.",
    submittingNotice: "Eine Anfrage sendet diese Zahlung. Bestätigen Sie sie nicht erneut; aktualisieren oder gleichen Sie die erfasste Zahlung ab.",
    unknownNotice: "Das Netzwerkergebnis ist noch nicht eindeutig. Bestätigen Sie keine weitere Zahlung; gleichen Sie diese Zahlung ab.",
    refreshStatus: "Zahlungsstatus aktualisieren",
    reconcile: "Netzwerkdatensatz prüfen",
  },
  route: {
    paymentRoute: "Zahlungsroute",
    loading: "Geprüfte Routen werden geladen…",
    unavailable: "Routen nicht verfügbar",
    select: "Zahlungsroute auswählen",
    none: "Keine aktiven Routen",
    checkedTitle: "Zahlungsroute geprüft",
    checkedCopy: "Diese Route wurde aus den aktiven Zahlungseinstellungen Ihres Kontos ausgewählt und vor der Signatur geprüft.",
    technicalLabel: "Technische Routendetails",
    technicalDetails: "Technische Routendetails",
    payerSource: "Das Quellkonto des Zahlers wird als kanonisches Konto für verknüpfte Tokens abgeleitet. Dieses Formular akzeptiert keine Kontoadresse aus dem Browser.",
    version: (version) => `Version ${version}`,
  },
  split: {
    sharing: "Aufteilung der Zahlung",
    oneDestination: "Ein genehmigtes Ziel",
    recipientPlan: "Genehmigter Empfängerplan",
    approvedPlan: "Genehmigter Empfängerplan",
    loading: "Genehmigte Pläne werden geladen…",
    unavailable: "Pläne nicht verfügbar",
    select: "Empfängerplan auswählen",
    none: "Keine aktiven Empfängerpläne",
    matches: "Empfängerplan passt zur Zahlungsroute",
    mismatch: "Empfängerplan passt nicht zu dieser Route",
    version: (version) => `Version ${version}`,
    recipients: (count) => `${count} genehmigte Empfänger`,
    details: "Details der Empfängerkonten",
    chooseRoute: "Wählen Sie eine Zahlungsroute für Händler und Asset des Plans.",
    sharesBound: "Empfängeranteile sind an diese Zahlung gebunden",
    sharesCopy: "genehmigte Empfänger · der Dienst besitzt Autorität über Empfänger und Beträge.",
    recipientDetails: "Details der Empfängerkonten",
    planReference: "Planreferenz",
    retry: "Erneut versuchen",
    basisPoints: "Basispunkte",
    distributionReview: "Prüfung der Empfängerverteilung",
  },
  safeguards: {
    title: "Zahlungsschutz automatisch ausgewählt",
    copy: "Die verifizierte Offerte liefert die Richtlinie. Für sichere Wiederholungen wird eine Zahlungsreferenz vergeben; Protokollwerte müssen nicht eingegeben werden.",
  },
  technical: {
    details: "Technische Zahlungsdetails",
    programId: "Programm-ID",
    policyAuthority: "Richtlinienautorität",
    computePolicy: "Berechnungsrichtlinie",
    priorityFee: "Prioritätsgebot",
    accounts: "Konten",
    paymentRequest: "Zahlungsanfrage",
    ruleReference: "Regelreferenz",
    computeUnits: (value) => `${value} Recheneinheiten`,
    microLamports: (value) => `${value} Mikro-Lamports pro Recheneinheit`,
    exactAccountEntries: (value) => `${value} exakte Kontoeinträge`,
  },
  status: {
    walletBoundary: "Die Wallet-Bestätigung beweist keine abgeschlossene Zahlung. Nur ein finalisierter Netzwerknachweis bestätigt die Bewegung.",
    legalBoundary: "Dies ist eine Einbehaltungsautorisierung—keine Steuererklärung, Meldung, rechtliche Konformität oder Steuerberatung.",
  },
  errors: GERMAN_ERRORS,
};

const ARABIC: CheckoutMessages = {
  review: {
    eyebrow: "مراجعة الدفع",
    title: "راجع الدفع قبل التوقيع",
    status: "تأكيد المحفظة مطلوب",
    description: "راجع المبلغ والضريبة والوجهة وحالة السياسة قبل أن تؤكد محفظتك الدفع. يتحقق الخادم من الدفع مرة أخرى قبل إرساله إلى الشبكة.",
    walletStep: "المحفظة المستخدمة",
    walletConnected: "المحفظة متصلة",
    walletMissing: "لا توجد محفظة متصلة",
    emptyTitle: "يلزم عرض سعر موثّق وإعداد الدفع",
    emptyCopy: "أنشئ عرض سعر موثّقًا ثم أعدّ الدفع قبل إتاحة تأكيد المحفظة.",
    total: "المبلغ الذي ستؤكده المحفظة",
    taxWithheld: "الضريبة المحتجزة",
    amountSent: "المبلغ المرسل",
    paymentAsset: "أصل الدفع",
    taxRule: "قاعدة الضريبة المستخدمة",
    taxStatus: "حالة الضريبة",
    calculatedNotAttested: "محسوبة؛ غير موثّقة",
    paymentReference: "مرجع الدفع",
    beforeSignEyebrow: "قبل التوقيع",
    beforeSignTitle: "مراجعة الدفع",
    readyStatus: "جاهز لتأكيد المحفظة",
    quoteExpires: "ينتهي عرض السعر",
    expired: "منتهي؛ أنشئ عرضًا جديدًا",
    serverEnforced: "مفروض من الخادم",
    walletBoundary: "تؤكد محفظتك الدفع المعروض أعلاه فقط. لا يؤكد قبول المحفظة وصول الدفع إلى الشبكة؛ استخدم حالة الدفع أدناه.",
    approve: "وقّع وأرسل الدفع",
    prepare: "إعداد الدفع للمراجعة",
    preparing: "جارٍ إعداد الدفع…",
    signing: "في انتظار المحفظة…",
    submitting: "جارٍ إرسال الدفع المراجع…",
    paymentStatus: (status) => `حالة الدفع: ${status}`,
    networkReference: (signature) => `مرجع الشبكة ${signature}`,
    noNetworkReference: "لم يُسجّل مرجع شبكة بعد.",
    submittingNotice: "يوجد طلب واحد يرسل هذا الدفع. لا تؤكده مرة أخرى؛ حدّث أو طابق الدفع المسجل.",
    unknownNotice: "نتيجة الشبكة غير واضحة بعد. لا تؤكد دفعًا آخر؛ طابق هذا الدفع المسجل.",
    refreshStatus: "تحديث حالة الدفع",
    reconcile: "فحص سجل الشبكة",
  },
  route: {
    paymentRoute: "مسار الدفع",
    loading: "جارٍ تحميل المسارات التي تم فحصها…",
    unavailable: "المسارات غير متاحة",
    select: "اختر مسار دفع",
    none: "لا توجد مسارات نشطة",
    checkedTitle: "تم فحص مسار الدفع",
    checkedCopy: "تم اختيار هذا المسار من إعدادات الدفع النشطة لحسابك وفحصه قبل التوقيع.",
    technicalLabel: "تفاصيل المسار التقنية",
    technicalDetails: "تفاصيل المسار التقنية",
    payerSource: "يُشتق حساب مصدر الدافع باعتباره حساب الرمز المرتبط القياسي. لا يقبل هذا النموذج عنوان حساب من المتصفح.",
    version: (version) => `الإصدار ${version}`,
  },
  split: {
    sharing: "كيفية مشاركة الدفع",
    oneDestination: "وجهة واحدة معتمدة",
    recipientPlan: "خطة مستلمين معتمدة",
    approvedPlan: "خطة مستلمين معتمدة",
    loading: "جارٍ تحميل الخطط المعتمدة…",
    unavailable: "الخطط غير متاحة",
    select: "اختر خطة مستلمين",
    none: "لا توجد خطط مستلمين نشطة",
    matches: "خطة المستلمين تطابق مسار الدفع",
    mismatch: "خطة المستلمين لا تطابق هذا المسار",
    version: (version) => `الإصدار ${version}`,
    recipients: (count) => `${count} مستلمين معتمدين`,
    details: "تفاصيل حسابات المستلمين",
    chooseRoute: "اختر مسار دفع لتاجر الخطة وأصلها.",
    sharesBound: "حصص المستلمين مرتبطة بهذا الدفع",
    sharesCopy: "مستلمون معتمدون · يملك الخادم سلطة المستلمين والمبالغ.",
    recipientDetails: "تفاصيل حسابات المستلمين",
    planReference: "مرجع الخطة",
    retry: "إعادة المحاولة",
    basisPoints: "نقاط أساس",
    distributionReview: "مراجعة توزيع المستلمين",
  },
  safeguards: {
    title: "تم اختيار ضمانات الدفع تلقائيًا",
    copy: "يوفر عرض السعر الموثّق السياسة. يُعيّن مرجع دفع آمن لإعادة المحاولة؛ لا حاجة لإدخال قيم البروتوكول.",
  },
  technical: {
    details: "تفاصيل الدفع التقنية",
    programId: "معرّف البرنامج",
    policyAuthority: "سلطة السياسة",
    computePolicy: "سياسة الحساب",
    priorityFee: "عرض أولوية الرسوم",
    accounts: "الحسابات",
    paymentRequest: "طلب الدفع",
    ruleReference: "مرجع القاعدة",
    computeUnits: (value) => `${value} وحدة حساب`,
    microLamports: (value) => `${value} ميكرو-لامبورت لكل وحدة حساب`,
    exactAccountEntries: (value) => `${value} إدخالات حساب دقيقة`,
  },
  status: {
    walletBoundary: "لا يثبت تأكيد المحفظة تسوية الدفع. يؤكد الحركةَ دليل شبكة نهائي فقط.",
    legalBoundary: "هذا تصريح بالاقتطاع—وليس إقرارًا ضريبيًا أو تقريرًا أو امتثالًا قانونيًا أو استشارة ضريبية.",
  },
  errors: ARABIC_ERRORS,
};

const JAPANESE: CheckoutMessages = {
  review: {
    eyebrow: "支払いレビュー",
    title: "署名前に支払いを確認",
    status: "ウォレットの承認が必要",
    description: "ウォレットで承認する前に、金額、税、宛先、ポリシーの状態を確認してください。ネットワークへ送信する前に、サービスが支払いを再確認します。",
    walletStep: "使用するウォレット",
    walletConnected: "ウォレット接続済み",
    walletMissing: "ウォレット未接続",
    emptyTitle: "検証済み見積もりと支払い準備が必要です",
    emptyCopy: "検証済み見積もりを作成し、ウォレット承認を利用できるように支払いを準備してください。",
    total: "ウォレットが承認する金額",
    taxWithheld: "源泉税",
    amountSent: "送金額",
    paymentAsset: "支払いアセット",
    taxRule: "使用した税ルール",
    taxStatus: "税の状態",
    calculatedNotAttested: "計算済み・証明未取得",
    paymentReference: "支払い参照",
    beforeSignEyebrow: "署名前",
    beforeSignTitle: "支払いレビュー",
    readyStatus: "ウォレット承認の準備完了",
    quoteExpires: "見積もりの有効期限",
    expired: "期限切れ・新しい見積もりを作成",
    serverEnforced: "サーバーで適用",
    walletBoundary: "ウォレットが承認するのは上記の支払いだけです。ウォレット承認はネットワーク到達の確認ではありません。下の支払い状態を確認してください。",
    approve: "署名して支払いを送信",
    prepare: "支払いを確認用に準備",
    preparing: "支払いを準備中…",
    signing: "ウォレットを待機中…",
    submitting: "確認済みの支払いを送信中…",
    paymentStatus: (status) => `支払い状態: ${status}`,
    networkReference: (signature) => `ネットワーク参照 ${signature}`,
    noNetworkReference: "ネットワーク参照はまだ記録されていません。",
    submittingNotice: "この支払いを送信中のリクエストが1件あります。再承認せず、記録された支払いを更新または照合してください。",
    unknownNotice: "ネットワーク結果はまだ不明確です。別の支払いを承認せず、この記録を照合してください。",
    refreshStatus: "支払い状態を更新",
    reconcile: "ネットワーク記録を確認",
  },
  route: {
    paymentRoute: "支払いルート",
    loading: "確認済みルートを読み込み中…",
    unavailable: "ルートを利用できません",
    select: "支払いルートを選択",
    none: "アクティブなルートはありません",
    checkedTitle: "支払いルートを確認済み",
    checkedCopy: "このルートはアカウントのアクティブな支払い設定から選択され、署名前に確認されました。",
    technicalLabel: "技術的なルート詳細",
    technicalDetails: "技術的なルート詳細",
    payerSource: "支払人のソースアカウントは正規の関連トークンアカウントとして導出されます。このフォームはブラウザーからのアカウントアドレスを受け付けません。",
    version: (version) => `バージョン ${version}`,
  },
  split: {
    sharing: "支払いの分配方法",
    oneDestination: "承認済みの単一宛先",
    recipientPlan: "承認済み受取人プラン",
    approvedPlan: "承認済み受取人プラン",
    loading: "承認済みプランを読み込み中…",
    unavailable: "プランを利用できません",
    select: "受取人プランを選択",
    none: "アクティブな受取人プランはありません",
    matches: "受取人プランは支払いルートに一致します",
    mismatch: "受取人プランはこのルートに一致しません",
    version: (version) => `バージョン ${version}`,
    recipients: (count) => `${count} 件の承認済み受取人`,
    details: "受取人アカウントの詳細",
    chooseRoute: "プランの加盟店とアセットに対応する支払いルートを選択してください。",
    sharesBound: "受取人の分配はこの支払いに紐付いています",
    sharesCopy: "承認済み受取人 · 受取人と金額の権限はサービスが保持します。",
    recipientDetails: "受取人アカウントの詳細",
    planReference: "プラン参照",
    retry: "再試行",
    basisPoints: "ベーシスポイント",
    distributionReview: "受取人分配レビュー",
  },
  safeguards: {
    title: "支払い保護を自動選択済み",
    copy: "検証済み見積もりがポリシーを提供します。安全な再試行用の支払い参照が割り当てられるため、プロトコル値の入力は不要です。",
  },
  technical: {
    details: "技術的な支払い詳細",
    programId: "プログラム ID",
    policyAuthority: "ポリシー権限",
    computePolicy: "計算ポリシー",
    priorityFee: "優先手数料入札",
    accounts: "アカウント",
    paymentRequest: "支払いリクエスト",
    ruleReference: "ルール参照",
    computeUnits: (value) => `${value} 計算ユニット`,
    microLamports: (value) => `計算ユニットあたり ${value} マイクロランポート`,
    exactAccountEntries: (value) => `${value} 件の正確なアカウントエントリ`,
  },
  status: {
    walletBoundary: "ウォレット承認は支払いが確定した証明ではありません。移動を確認できるのは確定済みネットワーク証拠だけです。",
    legalBoundary: "これは源泉徴収の承認であり、税務申告、報告、法令遵守、税務助言ではありません。",
  },
  errors: JAPANESE_ERRORS,
};

const CATALOGS: Record<SupportedLocale, CheckoutMessages> = {
  "en-US": ENGLISH,
  "de-DE": GERMAN,
  "ar-EG": ARABIC,
  "ja-JP": JAPANESE,
};

export function checkoutMessagesFor(locale: string): CheckoutMessages {
  return CATALOGS[locale as SupportedLocale] ?? ENGLISH;
}
