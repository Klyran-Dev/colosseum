import { SupportedLocale } from "./formatters";

export interface ReplayOpsMessages {
  control: {
    ariaLabel: string; eyebrow: string; title: string; badge: string; description: string;
    stream: string; streamPlaceholder: string; status: string; limit: string;
    loading: string; load: string; unavailable: string; authorityTitle: string;
    authorityCopy: string;
  };
  route: {
    summary: string; title: string; description: string; load: string; loading: string;
    unavailable: string; error: string; empty: string; emptyCopy: string;
    statusLabels: Record<string, string>; routesExamined: string; activated: string;
    alreadyActive: string; started: string; completed: string; failure: string;
    failed: (count: number) => string; evidenceReference: string; run: string;
    approval: string; evidenceCopy: string;
  };
  targeted: {
    eyebrow: string; title: string; description: string; address: string;
    addressPlaceholder: string; signature: string; signaturePlaceholder: string;
    pages: string; note: string; notePlaceholder: string; recovering: string;
    recover: string;
  };
  queue: {
    eyebrow: string; title: string; returned: (count: number) => string;
    notLoaded: string; notLoadedCopy: string; noMatches: string; noMatchesCopy: string;
  };
  gap: {
    slotRange: string; observedSlot: string; reason: string; lastError: string;
    none: string; created: string; deadLettered: string; reviewedBy: string;
    attempts: (attempts: number) => string; note: string; notePlaceholder: string;
    acknowledgedPlaceholder: string; acknowledgeBusy: string; acknowledge: string;
    requeueBusy: string; requeue: string; noMutation: string;
  };
  actions: { acknowledge: string; requeue: string };
  status: { labels: Record<string, string> };
  common: {
    unavailable: string; error: string; recoveryError: string; routeError: string;
    malformedRouteEvidence: string; pagesOutOfRange: (max: number) => string;
    httpError: (message: string, status: number) => string;
    actionFailed: (action: string) => string;
    projectionReloadFailed: (action: string) => string;
  };
}

function displayHttpStatus(status: number): string {
  return Number.isInteger(status) && status >= 100 && status <= 599 ? String(status) : "unknown status";
}

const ENGLISH: ReplayOpsMessages = {
  control: {
    ariaLabel: "Indexer replay operations workspace", eyebrow: "Chain data / recovery control", title: "Review replay gaps", badge: "Operator role required",
    description: "Review records that need recovery and document the incident before asking the service to try again. Recovery never confirms that a payment moved.",
    stream: "Replay stream", streamPlaceholder: "mainnet/settlement", status: "Status filter", limit: "Maximum rows", loading: "Loading recovery items…", load: "Load recovery items", unavailable: "The service is unavailable. No cached or fabricated replay state is shown.", authorityTitle: "What this action does", authorityCopy: "Access is checked by the service. Acknowledging records the review; retrying schedules a bounded recovery attempt.",
  },
  route: {
    summary: "Route-verification evidence", title: "Route-verification evidence", description: "Review the verifier's durable operational record without opening database access. Reading this evidence does not activate a route or confirm finalized settlement.", load: "Load route-verification evidence", loading: "Loading route-verification evidence…", unavailable: "The service is unavailable. No cached or fabricated route evidence is shown.", error: "Route-verification evidence lookup failed", empty: "No verification runs returned", emptyCopy: "The service returned an empty evidence set; this is not a claim that every route is healthy.", statusLabels: { running: "Verification in progress", completed: "Verification completed", completed_with_failures: "Verification completed with failures", failed: "Verification failed" }, routesExamined: "Routes examined", activated: "Activated", alreadyActive: "Already active", started: "Started", completed: "Completed", failure: "Failure", failed: (count) => `${count} failed`, evidenceReference: "Evidence reference", run: "Run", approval: "approval", evidenceCopy: "This record describes verifier processing only; finalized account evidence and the settlement program remain authoritative.",
  },
  targeted: {
    eyebrow: "Verified chain record", title: "Recover one transaction", description: "The service finds and verifies the finalized chain record, then rebuilds the saved evidence. It never submits funds or changes the payment itself.", address: "Target address", addressPlaceholder: "Base58 address", signature: "Transaction signature", signaturePlaceholder: "Base58 signature", pages: "Page budget", note: "Incident / recovery note (required)", notePlaceholder: "Reference the incident and why this evidence repair is required", recovering: "Recovering finalized evidence…", recover: "Recover finalized evidence",
  },
  queue: {
    eyebrow: "Saved recovery records", title: "Recovery queue", returned: (count) => `${count} returned`, notLoaded: "No recovery items loaded", notLoadedCopy: "Sign in with a recovery role and load the current service records before taking any action.", noMatches: "No items match this filter", noMatchesCopy: "This is a service response, not a claim that every chain record is healthy.",
  },
  gap: {
    slotRange: "Slot range", observedSlot: "Observed finalized slot", reason: "Reason", lastError: "Last error", none: "None recorded", created: "Created", deadLettered: "Dead-lettered", reviewedBy: "Reviewed by", attempts: (attempts) => `${attempts}/8 attempts`, note: "Incident / operator note (required, max 512 UTF-8 bytes)", notePlaceholder: "Reference the incident and review decision", acknowledgedPlaceholder: "Explain why bounded repair may be requeued", acknowledgeBusy: "Recording review…", acknowledge: "Acknowledge review", requeueBusy: "Requeueing bounded repair…", requeue: "Requeue repair", noMutation: "No operator mutation is available for this state. The API remains the authority for replay lifecycle transitions.",
  },
  actions: { acknowledge: "Acknowledge review", requeue: "Requeue repair" },
  status: { labels: { pending: "Pending", processing: "Processing", completed: "Completed", dead_lettered: "Dead-lettered", failed: "Failed" } },
  common: {
    unavailable: "The service is unavailable.", error: "Replay recovery failed.", recoveryError: "Targeted recovery failed; inspect the server audit state before retrying.", routeError: "Route-verification evidence lookup failed",
    malformedRouteEvidence: "Route-verification evidence is malformed. No evidence was rendered.",
    pagesOutOfRange: (max) => `Page budget must be between 1 and ${max}.`,
    httpError: (message, status) => `${message} (${displayHttpStatus(status)}).`,
    actionFailed: (action) => `${action} failed.`,
    projectionReloadFailed: (action) => `The server accepted ${action.toLowerCase()}, but the follow-up projection reload failed. Reconcile before taking another action.`,
  },
};

const GERMAN: ReplayOpsMessages = {
  control: { ariaLabel: "Arbeitsbereich für Indexer-Wiederherstellung", eyebrow: "Kettendaten / Wiederherstellung", title: "Wiederherstellungslücken prüfen", badge: "Bedienerrolle erforderlich", description: "Prüfen Sie Datensätze, die eine Wiederherstellung benötigen, und dokumentieren Sie den Vorfall, bevor Sie einen neuen Versuch anfordern. Die Wiederherstellung bestätigt niemals, dass eine Zahlung bewegt wurde.", stream: "Wiederherstellungsstream", streamPlaceholder: "mainnet/settlement", status: "Statusfilter", limit: "Maximale Zeilen", loading: "Wiederherstellungselemente werden geladen…", load: "Wiederherstellungselemente laden", unavailable: "Der Dienst ist nicht verfügbar. Es wird kein zwischengespeicherter oder erfundener Wiederherstellungsstatus angezeigt.", authorityTitle: "Was diese Aktion tut", authorityCopy: "Der Dienst prüft den Zugriff. Die Bestätigung erfasst die Prüfung; ein erneuter Versuch plant eine begrenzte Wiederherstellung.", },
  route: { summary: "Routenprüfungsnachweise", title: "Routenprüfungsnachweise", description: "Prüfen Sie den dauerhaften Betriebsdatensatz des Verifizierers ohne Datenbankzugriff. Das Lesen dieses Nachweises aktiviert keine Route und bestätigt keine endgültige Abwicklung.", load: "Routenprüfungsnachweise laden", loading: "Routenprüfungsnachweise werden geladen…", unavailable: "Der Dienst ist nicht verfügbar. Es wird kein zwischengespeicherter oder erfundener Routennachweis angezeigt.", error: "Die Routenprüfungsnachweise konnten nicht geladen werden", empty: "Keine Prüfläufe zurückgegeben", emptyCopy: "Der Dienst hat einen leeren Nachweissatz zurückgegeben; dies ist keine Aussage, dass jede Route gesund ist.", statusLabels: { running: "Prüfung läuft", completed: "Prüfung abgeschlossen", completed_with_failures: "Prüfung mit Fehlern abgeschlossen", failed: "Prüfung fehlgeschlagen" }, routesExamined: "Geprüfte Routen", activated: "Aktiviert", alreadyActive: "Bereits aktiv", started: "Gestartet", completed: "Abgeschlossen", failure: "Fehler", failed: (count) => `${count} fehlgeschlagen`, evidenceReference: "Nachweisreferenz", run: "Lauf", approval: "Genehmigung", evidenceCopy: "Dieser Datensatz beschreibt nur die Verarbeitung des Verifizierers; endgültige Kontonachweise und das Settlement-Programm bleiben maßgeblich.", },
  targeted: { eyebrow: "Verifizierter Chain-Datensatz", title: "Eine Transaktion wiederherstellen", description: "Der Dienst findet und verifiziert den endgültigen Chain-Datensatz und baut anschließend den gespeicherten Nachweis neu auf. Er übermittelt niemals Geld und ändert die Zahlung nicht.", address: "Zieladresse", addressPlaceholder: "Base58-Adresse", signature: "Transaktionssignatur", signaturePlaceholder: "Base58-Signatur", pages: "Seitenbudget", note: "Vorfalls- / Wiederherstellungsnotiz (erforderlich)", notePlaceholder: "Vorfall und Grund für die Nachweisreparatur angeben", recovering: "Endgültiger Nachweis wird wiederhergestellt…", recover: "Endgültigen Nachweis wiederherstellen", },
  queue: { eyebrow: "Gespeicherte Wiederherstellungsdatensätze", title: "Wiederherstellungswarteschlange", returned: (count) => `${count} zurückgegeben`, notLoaded: "Keine Wiederherstellungselemente geladen", notLoadedCopy: "Melden Sie sich mit einer Wiederherstellungsrolle an und laden Sie die aktuellen Dienstaufzeichnungen, bevor Sie handeln.", noMatches: "Keine Elemente entsprechen diesem Filter", noMatchesCopy: "Dies ist eine Dienstantwort und keine Aussage, dass jeder Chain-Datensatz gesund ist.", },
  gap: { slotRange: "Slot-Bereich", observedSlot: "Beobachteter endgültiger Slot", reason: "Grund", lastError: "Letzter Fehler", none: "Nichts erfasst", created: "Erstellt", deadLettered: "In Fehlerwarteschlange", reviewedBy: "Geprüft von", attempts: (attempts) => `${attempts}/8 Versuche`, note: "Vorfalls- / Bedienernotiz (erforderlich, max. 512 UTF-8-Bytes)", notePlaceholder: "Vorfall und Prüfentscheidung angeben", acknowledgedPlaceholder: "Begründen Sie, warum die begrenzte Reparatur erneut eingereiht werden darf", acknowledgeBusy: "Prüfung wird erfasst…", acknowledge: "Prüfung bestätigen", requeueBusy: "Begrenzte Reparatur wird erneut eingereiht…", requeue: "Reparatur erneut einreihen", noMutation: "Für diesen Status ist keine Bedieneränderung verfügbar. Die API bleibt maßgeblich für Übergänge im Wiederherstellungslebenszyklus.", },
  actions: { acknowledge: "Prüfung bestätigen", requeue: "Reparatur erneut einreihen" },
  status: { labels: { pending: "Ausstehend", processing: "In Verarbeitung", completed: "Abgeschlossen", dead_lettered: "In Fehlerwarteschlange", failed: "Fehlgeschlagen" } },
  common: {
    unavailable: "Der Dienst ist nicht verfügbar.", error: "Die Wiederherstellung ist fehlgeschlagen.", recoveryError: "Die gezielte Wiederherstellung ist fehlgeschlagen; prüfen Sie den Server-Prüfstatus vor einem erneuten Versuch.", routeError: "Die Routenprüfungsnachweise konnten nicht geladen werden",
    malformedRouteEvidence: "Die Routenprüfungsnachweise sind fehlerhaft. Es wurden keine Nachweise angezeigt.",
    pagesOutOfRange: (max) => `Das Seitenbudget muss zwischen 1 und ${max} liegen.`,
    httpError: (message, status) => `${message} (${displayHttpStatus(status)}).`,
    actionFailed: (action) => `${action} ist fehlgeschlagen.`,
    projectionReloadFailed: (action) => `Der Dienst hat ${action.toLowerCase()} akzeptiert, aber die anschließende Projektnachladung ist fehlgeschlagen. Stimmen Sie den Status ab, bevor Sie erneut handeln.`,
  },
};

const ARABIC: ReplayOpsMessages = {
  ...ENGLISH,
  control: { ...ENGLISH.control, ariaLabel: "مساحة عمليات استعادة الفهرس", eyebrow: "بيانات السلسلة / التحكم بالاستعادة", title: "مراجعة فجوات الاستعادة", badge: "دور المشغل مطلوب", description: "راجع السجلات التي تحتاج إلى استعادة ووثق الحادث قبل طلب المحاولة مرة أخرى. لا تثبت الاستعادة أبداً تحرك دفعة.", stream: "مسار الاستعادة", status: "مرشح الحالة", limit: "الحد الأقصى للصفوف", loading: "جارٍ تحميل عناصر الاستعادة…", load: "تحميل عناصر الاستعادة", unavailable: "الخدمة غير متاحة. لا يتم عرض حالة استعادة مخزنة أو مختلقة.", authorityTitle: "ما تفعله هذه العملية", authorityCopy: "تتحقق الخدمة من الوصول. يسجل التأكيد المراجعة؛ وتجدولة إعادة المحاولة استعادة محدودة." },
  route: { ...ENGLISH.route, summary: "أدلة التحقق من المسار", title: "أدلة التحقق من المسار", description: "راجع السجل التشغيلي الدائم للتحقق دون فتح وصول قاعدة البيانات. لا يؤدي قراءة هذا الدليل إلى تفعيل مسار أو تأكيد التسوية النهائية.", load: "تحميل أدلة التحقق من المسار", loading: "جارٍ تحميل أدلة التحقق من المسار…", unavailable: "الخدمة غير متاحة. لا يتم عرض دليل مسار مخزن أو مختلق.", empty: "لم يتم إرجاع عمليات تحقق", emptyCopy: "أعادت الخدمة مجموعة أدلة فارغة؛ لا يعني ذلك أن كل مسار سليم.", statusLabels: { running: "التحقق جارٍ", completed: "اكتمل التحقق", completed_with_failures: "اكتمل التحقق مع أخطاء", failed: "فشل التحقق" }, routesExamined: "المسارات المفحوصة", activated: "مفعّل", alreadyActive: "مفعّل مسبقاً", started: "بدأ", completed: "اكتمل", failure: "الفشل", failed: (count) => `${count} فشل`, evidenceReference: "مرجع الدليل", run: "عملية", approval: "موافقة", evidenceCopy: "يسجل هذا السجل معالجة التحقق فقط؛ تظل أدلة الحساب النهائية وبرنامج التسوية هي المرجع.", },
  targeted: { ...ENGLISH.targeted, eyebrow: "سجل سلسلة تم التحقق منه", title: "استعادة معاملة واحدة", description: "تجد الخدمة سجل السلسلة النهائي وتتحقق منه ثم تعيد بناء الدليل المحفوظ. لا ترسل أموالاً ولا تغير الدفعة نفسها.", address: "العنوان المستهدف", addressPlaceholder: "عنوان Base58", signature: "توقيع المعاملة", signaturePlaceholder: "توقيع Base58", pages: "ميزانية الصفحات", note: "ملاحظة الحادث / الاستعادة (مطلوبة)", notePlaceholder: "اذكر الحادث وسبب الحاجة إلى إصلاح الدليل", recovering: "جارٍ استعادة الدليل النهائي…", recover: "استعادة الدليل النهائي", },
  queue: { ...ENGLISH.queue, eyebrow: "سجلات الاستعادة المحفوظة", title: "قائمة انتظار الاستعادة", returned: (count) => `${count} مُعاد`, notLoaded: "لم يتم تحميل عناصر الاستعادة", notLoadedCopy: "سجّل الدخول بدور استعادة وحمّل سجلات الخدمة الحالية قبل اتخاذ أي إجراء.", noMatches: "لا توجد عناصر تطابق هذا المرشح", noMatchesCopy: "هذه استجابة من الخدمة وليست ادعاءً بأن كل سجل سلسلة سليم.", },
  gap: { ...ENGLISH.gap, slotRange: "نطاق الموضع", observedSlot: "الموضع النهائي المرصود", reason: "السبب", lastError: "آخر خطأ", none: "لم يسجل شيء", created: "أُنشئ", deadLettered: "في قائمة الأخطاء", reviewedBy: "راجعه", attempts: (attempts) => `${attempts}/8 محاولات`, note: "ملاحظة الحادث / المشغل (مطلوبة، بحد أقصى 512 بايت UTF-8)", notePlaceholder: "اذكر الحادث وقرار المراجعة", acknowledgedPlaceholder: "اشرح سبب السماح بإعادة إدراج الإصلاح المحدود", acknowledgeBusy: "جارٍ تسجيل المراجعة…", acknowledge: "تأكيد المراجعة", requeueBusy: "جارٍ إعادة إدراج الإصلاح المحدود…", requeue: "إعادة إدراج الإصلاح", noMutation: "لا يتوفر تعديل للمشغل في هذه الحالة. تظل واجهة API هي المرجع لانتقالات دورة حياة الاستعادة.", },
  actions: { acknowledge: "تأكيد المراجعة", requeue: "إعادة إدراج الإصلاح" },
  status: { labels: { pending: "معلّق", processing: "قيد المعالجة", completed: "مكتمل", dead_lettered: "في قائمة الأخطاء", failed: "فشل" } },
  common: {
    ...ENGLISH.common,
    unavailable: "الخدمة غير متاحة.", recoveryError: "فشلت الاستعادة المستهدفة؛ افحص حالة تدقيق الخادم قبل إعادة المحاولة.", routeError: "تعذر تحميل أدلة التحقق من المسار",
    malformedRouteEvidence: "دليل التحقق من المسار غير صالح. لم يتم عرض أي دليل.",
    pagesOutOfRange: (max) => `يجب أن تكون ميزانية الصفحات بين 1 و${max}.`,
    httpError: (message, status) => `${message} (${displayHttpStatus(status)}).`,
    actionFailed: (action) => `فشل ${action}.`,
    projectionReloadFailed: (action) => `قبلت الخدمة ${action}، لكن تعذر تحميل الإسقاط اللاحق. طابق الحالة قبل اتخاذ إجراء آخر.`,
  },
};

const JAPANESE: ReplayOpsMessages = { ...ENGLISH,
  control: { ...ENGLISH.control, ariaLabel: "インデクサー復旧ワークスペース", title: "復旧ギャップを確認", badge: "オペレーター権限が必要", description: "復旧が必要な記録を確認し、再試行を依頼する前にインシデントを記録します。復旧は支払いが移動したことを確認しません。", load: "復旧項目を読み込む" },
  route: { ...ENGLISH.route, summary: "ルート検証証拠", title: "ルート検証証拠", load: "ルート検証証拠を読み込む", loading: "ルート検証証拠を読み込み中…" },
  targeted: { ...ENGLISH.targeted, title: "1件の取引を復旧", recover: "確定証拠を復旧" },
  queue: { ...ENGLISH.queue, title: "復旧キュー", notLoaded: "復旧項目が読み込まれていません" },
  common: {
    ...ENGLISH.common,
    unavailable: "サービスを利用できません。",
    error: "復旧に失敗しました。",
    recoveryError: "対象復旧に失敗しました。再試行前にサーバー監査状態を確認してください。",
    routeError: "ルート検証証拠を読み込めませんでした",
    malformedRouteEvidence: "ルート検証証拠が不正です。証拠は表示されていません。",
    pagesOutOfRange: (max) => `ページ予算は1から${max}の間で指定してください。`,
    httpError: (message, status) => `${message}（${displayHttpStatus(status)}）。`,
    actionFailed: (action) => `${action}に失敗しました。`,
    projectionReloadFailed: (action) => `${action}はサービスに受理されましたが、後続のプロジェクション再読み込みに失敗しました。別の操作の前に照合してください。`,
  },
};

const CATALOG: Record<SupportedLocale, ReplayOpsMessages> = { "en-US": ENGLISH, "de-DE": GERMAN, "ar-EG": ARABIC, "ja-JP": JAPANESE };

export function replayOpsMessagesFor(locale: string): ReplayOpsMessages {
  return CATALOG[locale as SupportedLocale] ?? ENGLISH;
}
