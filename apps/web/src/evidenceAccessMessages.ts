import { SupportedLocale } from "./formatters";

export interface EvidenceAccessMessages {
  control: {
    ariaLabel: string;
    eyebrow: string;
    title: string;
    badge: string;
    description: string;
    calculationFilter: string;
    calculationPlaceholder: string;
    calculationHelp: string;
    limit: string;
    load: string;
    loading: string;
    unavailable: string;
    signIn: string;
    invalidFilter: string;
  };
  ledger: {
    eyebrow: string;
    title: string;
    notLoaded: string;
    notLoadedCopy: string;
    empty: string;
    emptyCopy: string;
    returned: (count: number) => string;
    allowed: string;
    denied: string;
    calculation: string;
    actor: string;
    purpose: string;
    occurred: string;
    technical: string;
    auditId: string;
    requestId: string;
    factsDigest: string;
    digestUnavailable: string;
    authorityTitle: string;
    authorityCopy: string;
  };
  common: {
    error: string;
    malformed: string;
    httpError: (message: string, status: number) => string;
  };
}

function displayHttpStatus(status: number): string {
  return Number.isInteger(status) && status >= 100 && status <= 599 ? String(status) : "unknown status";
}

const ENGLISH: EvidenceAccessMessages = {
  control: {
    ariaLabel: "Evidence and audit workspace",
    eyebrow: "Records / access history",
    title: "Review evidence access",
    badge: "Read only",
    description: "See when protected tax evidence was accessed, by whom, and for what reviewed purpose.",
    calculationFilter: "Calculation reference (optional)",
    calculationPlaceholder: "Paste a UUIDv7 calculation reference",
    calculationHelp: "Leave this blank to review the latest access records for this account.",
    limit: "Records to show",
    load: "Load access history",
    loading: "Loading access history…",
    unavailable: "Evidence history is unavailable while the service is offline.",
    signIn: "Sign in to review protected evidence access.",
    invalidFilter: "Enter a valid UUIDv7 calculation reference or clear the filter.",
  },
  ledger: {
    eyebrow: "Protected evidence ledger",
    title: "Recent access activity",
    notLoaded: "No access history loaded",
    notLoadedCopy: "Choose a bounded view to review the account's sensitive evidence-access record.",
    empty: "No access records found",
    emptyCopy: "No protected evidence access matches this account or filter.",
    returned: (count) => `${count} record${count === 1 ? "" : "s"}`,
    allowed: "Access allowed",
    denied: "Access denied",
    calculation: "Calculation",
    actor: "Actor",
    purpose: "Purpose",
    occurred: "Occurred",
    technical: "Technical record",
    auditId: "Audit ID",
    requestId: "Request ID",
    factsDigest: "Facts commitment",
    digestUnavailable: "No commitment recorded",
    authorityTitle: "What this page records:",
    authorityCopy: "Access activity only. It does not prove tax filing, legal compliance, payment authorization, or finalized Solana settlement.",
  },
  common: {
    error: "Access history could not be loaded.",
    malformed: "The service returned an unreadable access-history record.",
    httpError: (message, status) => `${message} (HTTP ${displayHttpStatus(status)})`,
  },
};

const GERMAN: EvidenceAccessMessages = {
  control: {
    ariaLabel: "Arbeitsbereich für Nachweise und Audit",
    eyebrow: "Aufzeichnungen / Zugriffshistorie",
    title: "Zugriff auf Nachweise prüfen",
    badge: "Nur Lesen",
    description: "Sehen Sie, wann geschützte Steuernachweise von wem und zu welchem geprüften Zweck aufgerufen wurden.",
    calculationFilter: "Berechnungsreferenz (optional)",
    calculationPlaceholder: "UUIDv7-Berechnungsreferenz einfügen",
    calculationHelp: "Leer lassen, um die neuesten Zugriffsdatensätze dieses Kontos zu prüfen.",
    limit: "Anzuzeigende Datensätze",
    load: "Zugriffshistorie laden",
    loading: "Zugriffshistorie wird geladen…",
    unavailable: "Die Nachweishistorie ist nicht verfügbar, solange der Dienst offline ist.",
    signIn: "Melden Sie sich an, um geschützte Nachweiszugriffe zu prüfen.",
    invalidFilter: "Geben Sie eine gültige UUIDv7-Berechnungsreferenz ein oder löschen Sie den Filter.",
  },
  ledger: {
    eyebrow: "Geschütztes Nachweisprotokoll",
    title: "Letzte Zugriffsaktivität",
    notLoaded: "Keine Zugriffshistorie geladen",
    notLoadedCopy: "Wählen Sie eine begrenzte Ansicht, um die sensiblen Nachweiszugriffe dieses Kontos zu prüfen.",
    empty: "Keine Zugriffsdatensätze gefunden",
    emptyCopy: "Für dieses Konto oder diesen Filter wurden keine geschützten Nachweise aufgerufen.",
    returned: (count) => `${count} Datensatz${count === 1 ? "" : "e"}`,
    allowed: "Zugriff erlaubt",
    denied: "Zugriff abgelehnt",
    calculation: "Berechnung",
    actor: "Akteur",
    purpose: "Zweck",
    occurred: "Zeitpunkt",
    technical: "Technischer Datensatz",
    auditId: "Audit-ID",
    requestId: "Anfrage-ID",
    factsDigest: "Faktenbindung",
    digestUnavailable: "Keine Bindung aufgezeichnet",
    authorityTitle: "Was diese Seite aufzeichnet:",
    authorityCopy: "Nur Zugriffsaktivität. Dies ist kein Nachweis für Steuererklärung, Rechtskonformität, Zahlungsfreigabe oder finalisierte Solana-Abwicklung.",
  },
  common: { error: "Die Zugriffshistorie konnte nicht geladen werden.", malformed: "Der Dienst hat einen unlesbaren Zugriffshistorieneintrag zurückgegeben.", httpError: (message, status) => `${message} (HTTP-Status ${displayHttpStatus(status)})` },
};

const ARABIC: EvidenceAccessMessages = {
  control: {
    ariaLabel: "مساحة عمل الأدلة والتدقيق",
    eyebrow: "السجلات / سجل الوصول",
    title: "مراجعة الوصول إلى الأدلة",
    badge: "للقراءة فقط",
    description: "اعرض متى تم الوصول إلى أدلة الضرائب المحمية ومن قام بذلك والغرض المراجع.",
    calculationFilter: "مرجع الحساب (اختياري)",
    calculationPlaceholder: "ألصق مرجع حساب UUIDv7",
    calculationHelp: "اتركه فارغًا لمراجعة أحدث سجلات الوصول لهذا الحساب.",
    limit: "السجلات المعروضة",
    load: "تحميل سجل الوصول",
    loading: "جارٍ تحميل سجل الوصول…",
    unavailable: "سجل الأدلة غير متاح أثناء توقف الخدمة.",
    signIn: "سجّل الدخول لمراجعة الوصول إلى الأدلة المحمية.",
    invalidFilter: "أدخل مرجع حساب UUIDv7 صالحًا أو امسح عامل التصفية.",
  },
  ledger: {
    eyebrow: "سجل الأدلة المحمية",
    title: "نشاط الوصول الأخير",
    notLoaded: "لم يتم تحميل سجل الوصول",
    notLoadedCopy: "اختر عرضًا محدودًا لمراجعة سجل الوصول إلى الأدلة الحساسة للحساب.",
    empty: "لم يتم العثور على سجلات وصول",
    emptyCopy: "لا يطابق هذا الحساب أو عامل التصفية أي وصول إلى أدلة محمية.",
    returned: (count) => `${count} سجل`,
    allowed: "تم السماح بالوصول",
    denied: "تم رفض الوصول",
    calculation: "الحساب",
    actor: "الفاعل",
    purpose: "الغرض",
    occurred: "حدث في",
    technical: "السجل التقني",
    auditId: "معرّف التدقيق",
    requestId: "معرّف الطلب",
    factsDigest: "التزام الحقائق",
    digestUnavailable: "لم يتم تسجيل التزام",
    authorityTitle: "ما تسجله هذه الصفحة:",
    authorityCopy: "نشاط الوصول فقط. لا يثبت ذلك تقديم الضرائب أو الامتثال القانوني أو تفويض الدفع أو التسوية النهائية على Solana.",
  },
  common: { error: "تعذر تحميل سجل الوصول.", malformed: "أعادت الخدمة سجل وصول غير قابل للقراءة.", httpError: (message, status) => `${message} (حالة HTTP: ${displayHttpStatus(status)})` },
};

const JAPANESE: EvidenceAccessMessages = {
  control: {
    ariaLabel: "証拠と監査ワークスペース",
    eyebrow: "記録 / アクセス履歴",
    title: "証拠へのアクセスを確認",
    badge: "読み取り専用",
    description: "保護された税務証拠に、いつ、誰が、どの確認済みの目的でアクセスしたかを確認します。",
    calculationFilter: "計算参照（任意）",
    calculationPlaceholder: "UUIDv7計算参照を貼り付け",
    calculationHelp: "空欄にすると、このアカウントの最新アクセス記録を確認します。",
    limit: "表示する記録",
    load: "アクセス履歴を読み込む",
    loading: "アクセス履歴を読み込み中…",
    unavailable: "サービスがオフラインのため、証拠履歴を利用できません。",
    signIn: "保護された証拠へのアクセスを確認するにはサインインしてください。",
    invalidFilter: "有効なUUIDv7計算参照を入力するか、フィルターを消去してください。",
  },
  ledger: {
    eyebrow: "保護された証拠台帳",
    title: "最近のアクセス活動",
    notLoaded: "アクセス履歴は未読み込みです",
    notLoadedCopy: "範囲を限定した表示を選択して、このアカウントの機密証拠アクセス記録を確認します。",
    empty: "アクセス記録が見つかりません",
    emptyCopy: "このアカウントまたはフィルターに一致する保護証拠へのアクセスはありません。",
    returned: (count) => `${count}件の記録`,
    allowed: "アクセスを許可",
    denied: "アクセスを拒否",
    calculation: "計算",
    actor: "アクター",
    purpose: "目的",
    occurred: "発生日時",
    technical: "技術記録",
    auditId: "監査ID",
    requestId: "リクエストID",
    factsDigest: "事実コミットメント",
    digestUnavailable: "コミットメント未記録",
    authorityTitle: "このページが記録する内容:",
    authorityCopy: "アクセス活動のみです。税務申告、法令遵守、支払い承認、Solanaの最終決済を証明するものではありません。",
  },
  common: { error: "アクセス履歴を読み込めませんでした。", malformed: "サービスが読み取れないアクセス履歴を返しました。", httpError: (message, status) => `${message}（HTTPステータス: ${displayHttpStatus(status)}）` },
};

const CATALOGS: Readonly<Partial<Record<SupportedLocale, EvidenceAccessMessages>>> = Object.freeze({
  "en-US": ENGLISH,
  "de-DE": GERMAN,
  "ar-EG": ARABIC,
  "ja-JP": JAPANESE,
});

export function evidenceAccessMessagesFor(locale: SupportedLocale | string): EvidenceAccessMessages {
  return CATALOGS[locale as SupportedLocale] ?? ENGLISH;
}
