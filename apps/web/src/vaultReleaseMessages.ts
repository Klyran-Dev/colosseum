import { SupportedLocale } from "./formatters";

export interface VaultReleaseMessages {
  proposal: {
    ariaLabel: string;
    eyebrow: string;
    title: string;
    control: string;
    description: string;
    signInStatus: string;
    accountReady: string;
    accountHelp: string;
    requestReference: string;
    requestHelp: string;
    releaseNumber: string;
    releaseNumberHelp: string;
    verifiedRoute: string;
    loadingRoutes: string;
    selectRoute: string;
    routeHelp: string;
    recipient: string;
    recipientPlaceholder: string;
    amount: string;
    amountHelp: string;
    earliest: string;
    earliestHelp: string;
    expiry: string;
    expiryHelp: string;
    evidenceReference: string;
    evidenceHelp: string;
    saving: string;
    create: string;
    routeErrorSuffix: string;
    routeDetails: string;
    routeEvidence: string;
    taxHoldingAccount: string;
    offline: string;
  };
  lifecycle: {
    ariaLabel: string;
    eyebrow: string;
    title: string;
    description: string;
    requestReference: string;
    requestPlaceholder: string;
    load: string;
    status: string;
    amount: string;
    releaseNumber: string;
    releaseWindow: string;
    requestEvidence: string;
    supportingEvidence: string;
    createdBy: string;
    approvedBy: string;
    notApproved: string;
    nextStep: string;
    approved: string;
    ledgerCreated: string;
    completed: string;
    cancelled: string;
    needsInvestigation: string;
    onChainAccount: string;
    required: string;
    ifAvailable: string;
    ledgerTransaction: string;
    confirmedPosition: string;
    whyStopped: string;
    decisionEvidence: string;
    decisionEvidenceHelp: string;
    record: string;
  };
  authority: {
    title: string;
    copy: string;
  };
  errors: VaultReleaseErrors;
}

export interface VaultReleaseErrors {
  authorizationRequired: string;
  routeRequestFailed: (status: number) => string;
  routeLoadFailed: string;
  noVerifiedRoutes: string;
  invalidClaim: string;
  invalidTransition: string;
  referenceDigestInvalid: string;
  claimProposalRequestFailed: (status: number) => string;
  claimProposalFailed: string;
  claimCreated: string;
  claimLookupRequestFailed: (status: number) => string;
  claimLookupFailed: string;
  claimLoaded: string;
  evidenceObjectRequired: string;
  claimTransitionRequestFailed: (status: number) => string;
  claimTransitionFailed: string;
  claimMismatch: string;
  transitionSaved: string;
  transitionSavedButRefreshFailed: string;
}

function displayHttpStatus(status: number): string {
  return Number.isInteger(status) && status >= 100 && status <= 599 ? String(status) : "unknown status";
}

const ENGLISH_ERRORS: VaultReleaseErrors = {
  authorizationRequired: "Sign in with an account allowed to manage treasury releases. The credential is held in memory only.",
  routeRequestFailed: (status) => `Verified payment routes could not be loaded (${displayHttpStatus(status)}).`,
  routeLoadFailed: "Verified payment routes could not be loaded.",
  noVerifiedRoutes: "No active, verified payment routes are available for this account.",
  invalidClaim: "The service returned an invalid release request. No transition was recorded; reconcile the request before continuing.",
  invalidTransition: "The service returned an invalid release transition. No ledger action was authorized.",
  referenceDigestInvalid: "Reference evidence must be a JSON array containing exactly 32 byte values.",
  claimProposalRequestFailed: (status) => `Release request proposal failed (${displayHttpStatus(status)}).`,
  claimProposalFailed: "Release request proposal failed.",
  claimCreated: "The release request was saved for approval. No funds moved and no spending authority was delegated.",
  claimLookupRequestFailed: (status) => `Release request lookup failed (${displayHttpStatus(status)}).`,
  claimLookupFailed: "Release request lookup failed.",
  claimLoaded: "The saved release request was loaded. Confirmed ledger evidence remains the final source of truth.",
  evidenceObjectRequired: "Decision evidence must be a JSON object.",
  claimTransitionRequestFailed: (status) => `Release request transition failed (${displayHttpStatus(status)}).`,
  claimTransitionFailed: "Release request transition failed.",
  claimMismatch: "The service returned a transition for a different release request. No local state was accepted.",
  transitionSaved: "The release request was transitioned and the page was refreshed from the service.",
  transitionSavedButRefreshFailed: "The release request transition was accepted, but the refreshed request could not be loaded. Reconcile before taking another action.",
};

const GERMAN_ERRORS: VaultReleaseErrors = {
  authorizationRequired: "Melden Sie sich mit einem Konto an, das Treasury-Freigaben verwalten darf. Das Zugriffstoken bleibt nur im Speicher.",
  routeRequestFailed: (status) => `Verifizierte Zahlungsrouten konnten nicht geladen werden (${displayHttpStatus(status)}).`,
  routeLoadFailed: "Verifizierte Zahlungsrouten konnten nicht geladen werden.",
  noVerifiedRoutes: "Für dieses Konto sind keine aktiven, verifizierten Zahlungsrouten verfügbar.",
  invalidClaim: "Der Dienst hat einen ungültigen Freigabeantrag zurückgegeben. Es wurde kein Übergang erfasst; gleichen Sie den Antrag vor dem Fortfahren ab.",
  invalidTransition: "Der Dienst hat einen ungültigen Freigabeübergang zurückgegeben. Es wurde keine Ledger-Aktion autorisiert.",
  referenceDigestInvalid: "Der Nachweis muss ein JSON-Array mit genau 32 Bytewerten sein.",
  claimProposalRequestFailed: (status) => `Der Freigabeantrag konnte nicht vorgeschlagen werden (${displayHttpStatus(status)}).`,
  claimProposalFailed: "Der Freigabeantrag konnte nicht vorgeschlagen werden.",
  claimCreated: "Der Freigabeantrag wurde zur Genehmigung gespeichert. Es wurden keine Mittel bewegt und keine Ausgabeberechtigung delegiert.",
  claimLookupRequestFailed: (status) => `Der Freigabeantrag konnte nicht geladen werden (${displayHttpStatus(status)}).`,
  claimLookupFailed: "Der Freigabeantrag konnte nicht geladen werden.",
  claimLoaded: "Der gespeicherte Freigabeantrag wurde geladen. Bestätigte Ledger-Nachweise bleiben die maßgebliche Quelle.",
  evidenceObjectRequired: "Entscheidungsnachweis muss ein JSON-Objekt sein.",
  claimTransitionRequestFailed: (status) => `Der Status des Freigabeantrags konnte nicht geändert werden (${displayHttpStatus(status)}).`,
  claimTransitionFailed: "Der Status des Freigabeantrags konnte nicht geändert werden.",
  claimMismatch: "Der Dienst hat einen Übergang für einen anderen Freigabeantrag zurückgegeben. Der lokale Status wurde nicht übernommen.",
  transitionSaved: "Der Status des Freigabeantrags wurde geändert und die Seite aus dem Dienst aktualisiert.",
  transitionSavedButRefreshFailed: "Der Übergang wurde angenommen, aber der aktualisierte Antrag konnte nicht geladen werden. Stimmen Sie ihn ab, bevor Sie erneut handeln.",
};

const ARABIC_ERRORS: VaultReleaseErrors = {
  authorizationRequired: "سجّل الدخول بحساب مسموح له بإدارة إصدارات الخزينة. يبقى الاعتماد في الذاكرة فقط.",
  routeRequestFailed: (status) => `تعذر تحميل مسارات الدفع الموثقة (${displayHttpStatus(status)}).`,
  routeLoadFailed: "تعذر تحميل مسارات الدفع الموثقة.",
  noVerifiedRoutes: "لا تتوفر لهذا الحساب مسارات دفع نشطة وموثقة.",
  invalidClaim: "أعادت الخدمة طلب إصدار غير صالح. لم يُسجّل أي انتقال؛ طابق الطلب قبل المتابعة.",
  invalidTransition: "أعادت الخدمة انتقال إصدار غير صالح. لم يُسمح بأي إجراء على دفتر الأستاذ.",
  referenceDigestInvalid: "يجب أن يكون دليل المرجع مصفوفة JSON تحتوي بالضبط على 32 قيمة بايت.",
  claimProposalRequestFailed: (status) => `تعذر اقتراح طلب الإصدار (${displayHttpStatus(status)}).`,
  claimProposalFailed: "تعذر اقتراح طلب الإصدار.",
  claimCreated: "تم حفظ طلب الإصدار للموافقة. لم تتحرك أي أموال ولم تُفوّض صلاحية إنفاق.",
  claimLookupRequestFailed: (status) => `تعذر تحميل طلب الإصدار (${displayHttpStatus(status)}).`,
  claimLookupFailed: "تعذر تحميل طلب الإصدار.",
  claimLoaded: "تم تحميل طلب الإصدار المحفوظ. يظل دليل دفتر الأستاذ المؤكد المصدر النهائي للحقيقة.",
  evidenceObjectRequired: "يجب أن يكون دليل القرار كائن JSON.",
  claimTransitionRequestFailed: (status) => `تعذر تغيير حالة طلب الإصدار (${displayHttpStatus(status)}).`,
  claimTransitionFailed: "تعذر تغيير حالة طلب الإصدار.",
  claimMismatch: "أعادت الخدمة انتقالاً لطلب إصدار مختلف. لم تُقبل الحالة المحلية.",
  transitionSaved: "تم تغيير حالة طلب الإصدار وتحديث الصفحة من الخدمة.",
  transitionSavedButRefreshFailed: "تم قبول الانتقال، لكن تعذر تحميل الطلب المحدّث. طابق الحالة قبل اتخاذ إجراء آخر.",
};

const JAPANESE_ERRORS: VaultReleaseErrors = {
  authorizationRequired: "財務リリースを管理できるアカウントでサインインしてください。認証情報はメモリ内だけに保持されます。",
  routeRequestFailed: (status) => `検証済み支払いルートを読み込めませんでした（${displayHttpStatus(status)}）。`,
  routeLoadFailed: "検証済み支払いルートを読み込めませんでした。",
  noVerifiedRoutes: "このアカウントで利用できるアクティブな検証済み支払いルートはありません。",
  invalidClaim: "サービスが無効なリリース申請を返しました。状態変更は記録されていません。続行前に照合してください。",
  invalidTransition: "サービスが無効なリリース状態変更を返しました。台帳操作は承認されていません。",
  referenceDigestInvalid: "証拠参照は32個のバイト値を含むJSON配列である必要があります。",
  claimProposalRequestFailed: (status) => `リリース申請を提案できませんでした（${displayHttpStatus(status)}）。`,
  claimProposalFailed: "リリース申請を提案できませんでした。",
  claimCreated: "リリース申請を承認用に保存しました。資金は移動せず、支出権限も委譲されていません。",
  claimLookupRequestFailed: (status) => `リリース申請を読み込めませんでした（${displayHttpStatus(status)}）。`,
  claimLookupFailed: "リリース申請を読み込めませんでした。",
  claimLoaded: "保存済みリリース申請を読み込みました。確認済み台帳証拠が最終的な真実の情報源です。",
  evidenceObjectRequired: "判断証拠はJSONオブジェクトである必要があります。",
  claimTransitionRequestFailed: (status) => `リリース申請の状態を変更できませんでした（${displayHttpStatus(status)}）。`,
  claimTransitionFailed: "リリース申請の状態を変更できませんでした。",
  claimMismatch: "サービスが別のリリース申請の状態変更を返しました。ローカル状態は受け入れていません。",
  transitionSaved: "リリース申請の状態を変更し、サービスからページを更新しました。",
  transitionSavedButRefreshFailed: "状態変更は受理されましたが、更新後の申請を読み込めませんでした。別の操作の前に照合してください。",
};

const ENGLISH: VaultReleaseMessages = {
  proposal: {
    ariaLabel: "Governed vault release workspace",
    eyebrow: "Treasury / two-person approval",
    title: "Create a release request",
    control: "Two-person control",
    description: "Create a release request for review. Approval, ledger recording, completion, cancellation, and uncertainty are tracked separately so every decision can be explained. The browser never signs or moves funds.",
    signInStatus: "Sign-in status",
    accountReady: "Account ready",
    accountHelp: "Your account must be allowed to create treasury release requests. Sign-in data is never saved by this page.",
    requestReference: "Request reference",
    requestHelp: "A unique reference prevents duplicate requests when you retry.",
    releaseNumber: "Release reference number",
    releaseNumberHelp: "This binds the request to one ledger record.",
    verifiedRoute: "Verified payment route",
    loadingRoutes: "Loading verified routes…",
    selectRoute: "Select a verified route",
    routeHelp: "Mint, token program, and tax holding account are resolved from this server-approved route.",
    recipient: "Recipient account",
    recipientPlaceholder: "Paste the recipient account",
    amount: "Amount",
    amountHelp: "Enter the exact amount in the asset’s smallest unit. The service checks precision before saving.",
    earliest: "Earliest release time",
    earliestHelp: "Use the UTC timestamp required by the release policy.",
    expiry: "Release expiry",
    expiryHelp: "The request cannot be executed after this time.",
    evidenceReference: "Evidence reference",
    evidenceHelp: "Paste the exact evidence reference supplied by the approved workflow. A request cannot be created without it.",
    saving: "Saving…",
    create: "Create release request",
    routeErrorSuffix: "No release request can be created until a verified route is available.",
    routeDetails: "Verified route details",
    routeEvidence: "Route evidence",
    taxHoldingAccount: "Tax holding account:",
    offline: "The API is unavailable. No client-side fallback or optimistic claim is created.",
  },
  lifecycle: {
    ariaLabel: "Release lifecycle and evidence",
    eyebrow: "Lifecycle and evidence",
    title: "Review or transition a claim",
    description: "Each next step requires the right approval. Ledger-created and completed states need proof from the network; an uncertain state must include a clear reason.",
    requestReference: "Release request reference",
    requestPlaceholder: "Paste a saved release request reference",
    load: "Load release request",
    status: "Status",
    amount: "Amount",
    releaseNumber: "Release reference number",
    releaseWindow: "Release window",
    requestEvidence: "Request evidence",
    supportingEvidence: "Supporting evidence",
    createdBy: "Created by",
    approvedBy: "Approved by",
    notApproved: "Not approved",
    nextStep: "Next step",
    approved: "Approved",
    ledgerCreated: "Ledger record created",
    completed: "Completed",
    cancelled: "Cancelled",
    needsInvestigation: "Needs investigation",
    onChainAccount: "On-chain release account",
    required: "required",
    ifAvailable: "if available",
    ledgerTransaction: "Ledger transaction reference (required)",
    confirmedPosition: "Confirmed ledger position (required)",
    whyStopped: "Why it stopped (required)",
    decisionEvidence: "Decision evidence",
    decisionEvidenceHelp: "Advanced evidence for the audit record. Use the format supplied by your operations workflow.",
    record: "Record next step",
  },
  authority: {
    title: "What this page can do:",
    copy: "record requests and evidence. A separately approved operator must prepare, sign, submit, and reconcile the ledger transaction. Confirmed ledger state—not this page—decides whether funds moved.",
  },
  errors: ENGLISH_ERRORS,
};

const GERMAN: VaultReleaseMessages = {
  proposal: {
    ariaLabel: "Verwalteter Tresor-Freigabebereich",
    eyebrow: "Treasury / Vier-Augen-Freigabe",
    title: "Freigabeantrag erstellen",
    control: "Vier-Augen-Kontrolle",
    description: "Erstellen Sie einen Freigabeantrag zur Prüfung. Genehmigung, Ledger-Erfassung, Abschluss, Stornierung und Unsicherheit werden getrennt erfasst, damit jede Entscheidung erklärt werden kann. Der Browser signiert oder bewegt niemals Geld.",
    signInStatus: "Anmeldestatus",
    accountReady: "Konto bereit",
    accountHelp: "Ihr Konto muss zum Erstellen von Treasury-Freigaben berechtigt sein. Anmeldedaten werden von dieser Seite niemals gespeichert.",
    requestReference: "Antragsreferenz",
    requestHelp: "Eine eindeutige Referenz verhindert doppelte Anträge bei Wiederholungen.",
    releaseNumber: "Freigabereferenznummer",
    releaseNumberHelp: "Damit wird der Antrag an genau einen Ledger-Eintrag gebunden.",
    verifiedRoute: "Verifizierte Zahlungsroute",
    loadingRoutes: "Verifizierte Routen werden geladen…",
    selectRoute: "Verifizierte Route auswählen",
    routeHelp: "Mint, Tokenprogramm und Steuerverwahrungskonto werden aus dieser serverseitig genehmigten Route aufgelöst.",
    recipient: "Empfängerkonto",
    recipientPlaceholder: "Empfängerkonto einfügen",
    amount: "Betrag",
    amountHelp: "Geben Sie den exakten Betrag in der kleinsten Einheit des Assets ein. Der Dienst prüft die Genauigkeit vor dem Speichern.",
    earliest: "Früheste Freigabezeit",
    earliestHelp: "Verwenden Sie den von der Freigaberichtlinie geforderten UTC-Zeitstempel.",
    expiry: "Ablauf der Freigabe",
    expiryHelp: "Nach diesem Zeitpunkt kann der Antrag nicht mehr ausgeführt werden.",
    evidenceReference: "Nachweisreferenz",
    evidenceHelp: "Fügen Sie die exakte Nachweisreferenz aus dem genehmigten Ablauf ein. Ohne sie kann kein Antrag erstellt werden.",
    saving: "Wird gespeichert…",
    create: "Freigabeantrag erstellen",
    routeErrorSuffix: "Ein Freigabeantrag kann erst erstellt werden, wenn eine verifizierte Route verfügbar ist.",
    routeDetails: "Details der verifizierten Route",
    routeEvidence: "Routennachweis",
    taxHoldingAccount: "Steuerverwahrungskonto:",
    offline: "Die API ist nicht verfügbar. Es wird kein clientseitiger Ersatz oder optimistischer Antrag erstellt.",
  },
  lifecycle: {
    ariaLabel: "Lebenszyklus und Nachweise der Freigabe",
    eyebrow: "Lebenszyklus und Nachweise",
    title: "Antrag prüfen oder weiterführen",
    description: "Jeder nächste Schritt erfordert die passende Genehmigung. Für im Ledger erstellte und abgeschlossene Zustände ist ein Netzwerknachweis erforderlich; ein unklarer Zustand braucht eine eindeutige Begründung.",
    requestReference: "Referenz des Freigabeantrags",
    requestPlaceholder: "Gespeicherte Referenz des Freigabeantrags einfügen",
    load: "Freigabeantrag laden",
    status: "Status",
    amount: "Betrag",
    releaseNumber: "Freigabereferenznummer",
    releaseWindow: "Freigabezeitraum",
    requestEvidence: "Antragsnachweis",
    supportingEvidence: "Zusätzlicher Nachweis",
    createdBy: "Erstellt von",
    approvedBy: "Genehmigt von",
    notApproved: "Nicht genehmigt",
    nextStep: "Nächster Schritt",
    approved: "Genehmigt",
    ledgerCreated: "Ledger-Eintrag erstellt",
    completed: "Abgeschlossen",
    cancelled: "Storniert",
    needsInvestigation: "Prüfung erforderlich",
    onChainAccount: "On-Chain-Freigabekonto",
    required: "erforderlich",
    ifAvailable: "falls vorhanden",
    ledgerTransaction: "Ledger-Transaktionsreferenz (erforderlich)",
    confirmedPosition: "Bestätigte Ledger-Position (erforderlich)",
    whyStopped: "Grund des Stopps (erforderlich)",
    decisionEvidence: "Entscheidungsnachweis",
    decisionEvidenceHelp: "Erweiterter Nachweis für den Prüfdatensatz. Verwenden Sie das Format Ihres Betriebsablaufs.",
    record: "Nächsten Schritt erfassen",
  },
  authority: {
    title: "Diese Seite kann Folgendes tun:",
    copy: "Anträge und Nachweise erfassen. Ein separat genehmigter Operator muss die Ledger-Transaktion vorbereiten, signieren, übermitteln und abgleichen. Der bestätigte Ledger-Status – nicht diese Seite – entscheidet, ob Geld bewegt wurde.",
  },
  errors: GERMAN_ERRORS,
};

const ARABIC: VaultReleaseMessages = {
  proposal: {
    ariaLabel: "مساحة إصدار الخزينة الخاضعة للحوكمة",
    eyebrow: "الخزينة / موافقة شخصين",
    title: "إنشاء طلب إصدار",
    control: "تحكم بموافقة شخصين",
    description: "أنشئ طلب إصدار للمراجعة. تُتتبّع الموافقة وتسجيل دفتر الأستاذ والإكمال والإلغاء وعدم اليقين بشكل منفصل حتى يمكن شرح كل قرار. لا يوقّع المتصفح ولا ينقل الأموال.",
    signInStatus: "حالة تسجيل الدخول",
    accountReady: "الحساب جاهز",
    accountHelp: "يجب أن يُسمح لحسابك بإنشاء طلبات إصدار الخزينة. لا تحفظ هذه الصفحة بيانات تسجيل الدخول أبداً.",
    requestReference: "مرجع الطلب",
    requestHelp: "يمنع المرجع الفريد إنشاء طلبات مكررة عند إعادة المحاولة.",
    releaseNumber: "رقم مرجع الإصدار",
    releaseNumberHelp: "يربط هذا الطلب بسجل واحد في دفتر الأستاذ.",
    verifiedRoute: "مسار دفع موثّق",
    loadingRoutes: "جارٍ تحميل المسارات الموثّقة…",
    selectRoute: "اختر مساراً موثّقاً",
    routeHelp: "يُحدَّد الأصل وبرنامج الرموز وحساب حيازة الضريبة من هذا المسار المعتمد من الخدمة.",
    recipient: "حساب المستلم",
    recipientPlaceholder: "ألصق حساب المستلم",
    amount: "المبلغ",
    amountHelp: "أدخل المبلغ الدقيق بأصغر وحدة من الأصل. تتحقق الخدمة من الدقة قبل الحفظ.",
    earliest: "أبكر وقت للإصدار",
    earliestHelp: "استخدم الطابع الزمني UTC المطلوب في سياسة الإصدار.",
    expiry: "انتهاء الإصدار",
    expiryHelp: "لا يمكن تنفيذ الطلب بعد هذا الوقت.",
    evidenceReference: "مرجع الدليل",
    evidenceHelp: "ألصق مرجع الدليل الدقيق الذي قدمه المسار المعتمد. لا يمكن إنشاء الطلب بدونه.",
    saving: "جارٍ الحفظ…",
    create: "إنشاء طلب إصدار",
    routeErrorSuffix: "لا يمكن إنشاء طلب إصدار حتى يتوفر مسار موثّق.",
    routeDetails: "تفاصيل المسار الموثّق",
    routeEvidence: "دليل المسار",
    taxHoldingAccount: "حساب حيازة الضريبة:",
    offline: "واجهة API غير متاحة. لم يتم إنشاء بديل محلي أو طلب متفائل.",
  },
  lifecycle: {
    ariaLabel: "دورة حياة الإصدار وأدلته",
    eyebrow: "دورة الحياة والأدلة",
    title: "مراجعة طلب أو تغيير حالته",
    description: "يتطلب كل إجراء تالٍ الموافقة المناسبة. تحتاج حالات إنشاء السجل في دفتر الأستاذ والإكمال إلى دليل من الشبكة؛ ويجب أن تتضمن الحالة غير المؤكدة سبباً واضحاً.",
    requestReference: "مرجع طلب الإصدار",
    requestPlaceholder: "ألصق مرجع طلب إصدار محفوظاً",
    load: "تحميل طلب الإصدار",
    status: "الحالة",
    amount: "المبلغ",
    releaseNumber: "رقم مرجع الإصدار",
    releaseWindow: "نافذة الإصدار",
    requestEvidence: "دليل الطلب",
    supportingEvidence: "الدليل الداعم",
    createdBy: "أنشأه",
    approvedBy: "وافق عليه",
    notApproved: "غير معتمد",
    nextStep: "الخطوة التالية",
    approved: "معتمد",
    ledgerCreated: "تم إنشاء سجل دفتر الأستاذ",
    completed: "مكتمل",
    cancelled: "ملغى",
    needsInvestigation: "يحتاج إلى تحقيق",
    onChainAccount: "حساب الإصدار على الشبكة",
    required: "مطلوب",
    ifAvailable: "إن توفر",
    ledgerTransaction: "مرجع معاملة دفتر الأستاذ (مطلوب)",
    confirmedPosition: "موضع دفتر الأستاذ المؤكد (مطلوب)",
    whyStopped: "سبب التوقف (مطلوب)",
    decisionEvidence: "دليل القرار",
    decisionEvidenceHelp: "دليل متقدم لسجل التدقيق. استخدم التنسيق الذي يحدده سير عمل العمليات.",
    record: "تسجيل الخطوة التالية",
  },
  authority: {
    title: "ما يمكن لهذه الصفحة فعله:",
    copy: "تسجيل الطلبات والأدلة. يجب على مشغّل معتمد بشكل منفصل إعداد معاملة دفتر الأستاذ وتوقيعها وإرسالها ومطابقتها. حالة دفتر الأستاذ المؤكدة — وليس هذه الصفحة — هي التي تحدد ما إذا كانت الأموال قد تحركت.",
  },
  errors: ARABIC_ERRORS,
};

const JAPANESE: VaultReleaseMessages = {
  proposal: {
    ariaLabel: "管理対象の保管庫リリースワークスペース",
    eyebrow: "財務 / 二者承認",
    title: "リリース申請を作成",
    control: "二者管理",
    description: "確認用のリリース申請を作成します。承認、台帳記録、完了、取消し、不確実性を分けて追跡し、すべての判断を説明できるようにします。ブラウザが署名したり資金を移動したりすることはありません。",
    signInStatus: "サインイン状態",
    accountReady: "アカウント準備完了",
    accountHelp: "アカウントには財務リリース申請の作成権限が必要です。このページはサインイン情報を保存しません。",
    requestReference: "申請参照",
    requestHelp: "一意の参照により、再試行時の重複申請を防ぎます。",
    releaseNumber: "リリース参照番号",
    releaseNumberHelp: "申請を1つの台帳レコードに結び付けます。",
    verifiedRoute: "検証済み支払いルート",
    loadingRoutes: "検証済みルートを読み込み中…",
    selectRoute: "検証済みルートを選択",
    routeHelp: "Mint、トークンプログラム、税金保管口座は、このサーバー承認済みルートから解決されます。",
    recipient: "受取口座",
    recipientPlaceholder: "受取口座を貼り付け",
    amount: "金額",
    amountHelp: "アセットの最小単位で正確な金額を入力します。保存前にサービスが精度を確認します。",
    earliest: "最短リリース時刻",
    earliestHelp: "リリースポリシーで指定されたUTCタイムスタンプを使用します。",
    expiry: "リリース期限",
    expiryHelp: "この時刻を過ぎると申請は実行できません。",
    evidenceReference: "証拠参照",
    evidenceHelp: "承認済みワークフローから提供された正確な証拠参照を貼り付けます。これがないと申請を作成できません。",
    saving: "保存中…",
    create: "リリース申請を作成",
    routeErrorSuffix: "検証済みルートが利用可能になるまでリリース申請は作成できません。",
    routeDetails: "検証済みルートの詳細",
    routeEvidence: "ルート証拠",
    taxHoldingAccount: "税金保管口座:",
    offline: "APIを利用できません。クライアント側の代替処理や楽観的な申請は作成されません。",
  },
  lifecycle: {
    ariaLabel: "リリースのライフサイクルと証拠",
    eyebrow: "ライフサイクルと証拠",
    title: "申請を確認または状態変更",
    description: "次の各ステップには適切な承認が必要です。台帳作成済みと完了の状態にはネットワーク証拠が必要で、不確実な状態には明確な理由が必要です。",
    requestReference: "リリース申請参照",
    requestPlaceholder: "保存したリリース申請参照を貼り付け",
    load: "リリース申請を読み込む",
    status: "状態",
    amount: "金額",
    releaseNumber: "リリース参照番号",
    releaseWindow: "リリース期間",
    requestEvidence: "申請証拠",
    supportingEvidence: "補足証拠",
    createdBy: "作成者",
    approvedBy: "承認者",
    notApproved: "未承認",
    nextStep: "次のステップ",
    approved: "承認済み",
    ledgerCreated: "台帳レコード作成済み",
    completed: "完了",
    cancelled: "取消済み",
    needsInvestigation: "調査が必要",
    onChainAccount: "オンチェーンのリリース口座",
    required: "必須",
    ifAvailable: "利用可能な場合",
    ledgerTransaction: "台帳トランザクション参照（必須）",
    confirmedPosition: "確認済み台帳位置（必須）",
    whyStopped: "停止理由（必須）",
    decisionEvidence: "判断証拠",
    decisionEvidenceHelp: "監査記録のための高度な証拠です。運用ワークフローで指定された形式を使用してください。",
    record: "次のステップを記録",
  },
  authority: {
    title: "このページでできること:",
    copy: "申請と証拠を記録します。別途承認された担当者が台帳トランザクションを準備、署名、送信、照合する必要があります。資金が移動したかどうかを決めるのは、このページではなく確認済み台帳状態です。",
  },
  errors: JAPANESE_ERRORS,
};

const CATALOG: Record<SupportedLocale, VaultReleaseMessages> = {
  "en-US": ENGLISH,
  "de-DE": GERMAN,
  "ar-EG": ARABIC,
  "ja-JP": JAPANESE,
};

export function vaultReleaseMessagesFor(locale: string): VaultReleaseMessages {
  return CATALOG[locale as SupportedLocale] ?? ENGLISH;
}
