import { SupportedLocale } from "./formatters";
import { ProductWorkspace } from "./navigation";
import { ActionRisk, UiAccessReasonCode, WorkspaceGroup } from "./authorization";

export interface ShellNavigationMessage {
  label: string;
  description: string;
}

export interface ShellChromeMessages {
  skipToWorkspace: string;
  productNavigation: string;
  primaryNavigation: string;
  account: string;
  openNavigation: string;
  closeNavigation: string;
  collapseNavigation: string;
  expandNavigation: string;
  operations: string;
  breadcrumb: string;
  displaySettings: string;
  openDisplaySettings: string;
  language: string;
  timeZone: string;
  theme: string;
  themeSystem: string;
  themeLight: string;
  themeDark: string;
  dateTimeHelp: (timeZone: string) => string;
  themeHelp: string;
  unknownTimeZone: (fallback: string) => string;
  workspaces: (count: number) => string;
  yourWorkArea: string;
  sidebarLegal: string;
}

export interface ShellSessionMessages {
  localSignIn: string;
  openLocalSignIn: string;
  signInCredential: string;
  localCredentialPlaceholder: string;
  localDevelopmentHelp: string;
  signedInSession: string;
  signedIn: string;
  signOut: string;
  signIn: string;
  openingSignIn: string;
  signInSetupNeeded: string;
  signInSetupNeededSummary: string;
  secureSignInHelp: string;
  emailOtpSignIn: string;
  openEmailOtpSignIn: string;
  emailLabel: string;
  emailPlaceholder: string;
  emailOtpHelp: string;
  sendCode: string;
  sendingCode: string;
  emailOtpSent: string;
  codeLabel: string;
  codePlaceholder: string;
  verifyCode: string;
  verifyingCode: string;
  changeEmail: string;
  protectedWorkspace: string;
  signInToContinue: string;
  signInToContinueCopy: string;
  signInInHeader: string;
}

export interface ShellMessages {
  groups: Readonly<Record<WorkspaceGroup, string>>;
  navigation: Readonly<Record<ProductWorkspace, ShellNavigationMessage>>;
  riskLabels: Readonly<Record<ActionRisk, string>>;
  accessStatus: Readonly<{
    available: string;
    unavailable: string;
    needsCheck: string;
    signInToCheck: string;
    unknownPolicy: string;
  }>;
  accessReason: (reasonCode: UiAccessReasonCode) => string;
  chrome: ShellChromeMessages;
  session: ShellSessionMessages;
  workspaceStatus(label: string): string;
}

const ENGLISH_GROUPS: Readonly<Record<WorkspaceGroup, string>> = {
  "Daily work": "Daily work",
  "Rules & access": "Rules & access",
  "Money movement": "Money movement",
  "Records & issues": "Records & issues",
};

const ENGLISH_NAVIGATION: Readonly<Record<ProductWorkspace, ShellNavigationMessage>> = {
  quote: { label: "Tax quotes", description: "Prepare a customer quote with a clear tax breakdown." },
  preview: { label: "Tax simulator", description: "Explore a scenario without changing live rules." },
  "document-quote": { label: "Invoice tax", description: "Calculate tax across an invoice and its lines." },
  checkout: { label: "Payment review", description: "Check payment details before the customer signs." },
  "document-checkout": { label: "Document payment", description: "Review and approve payment for an invoice." },
  governance: { label: "Tax rules", description: "Create, review, approve, and publish tax rules." },
  "access-control": { label: "Access & approvals", description: "See who can act, what requires approval, and why." },
  "vault-release": { label: "Treasury", description: "Prepare and review a controlled treasury release." },
  "vault-reconciliation": { label: "Balance checks", description: "Compare recorded balances with the latest verified observation." },
  "revenue-streaming": { label: "Partner payouts", description: "Review governed payout plans and prepare a verified release." },
  adjustments: { label: "Adjustments", description: "Prepare a correction for a completed transaction." },
  "refund-claims": { label: "Refund requests", description: "Prepare a refund request for review." },
  "refund-cancellations": { label: "Refund cancellations", description: "Stop a refund request with an approval record." },
  "refund-executions": { label: "Refund payments", description: "Release an approved refund after all checks pass." },
  "replay-ops": { label: "Exceptions & recovery", description: "Investigate and recover records that need attention." },
  "evidence-access": { label: "Evidence & audit", description: "Review protected evidence access activity for this account." },
};

const ENGLISH_RISK_LABELS: Readonly<Record<ActionRisk, string>> = {
  read: "Read-only workspace",
  write: "Change action",
  financial: "Sensitive financial action",
  governance: "Approval may be required",
};

const ENGLISH_ACCESS_STATUS = {
  available: "Available",
  unavailable: "Not available",
  needsCheck: "Needs access check",
  signInToCheck: "Sign in to check",
  unknownPolicy: "Access could not be described",
} as const;

const ENGLISH_ACCESS_REASONS: Readonly<Record<UiAccessReasonCode, string>> = {
  sign_in_required: "Sign in before using this workspace.",
  public_access: "This workspace is available without a signed-in account.",
  server_evaluation_required: "Your access is being checked before this workspace can be used.",
  server_denied: "Your account cannot use this workspace right now.",
  missing_expiry: "Your access check is missing an expiry; refresh before continuing.",
  clock_unavailable: "Access cannot be confirmed because the time check is unavailable.",
  access_expired: "Your access check is out of date; refresh before continuing.",
  missing_capability_decision: "Your access is not confirmed for this workspace yet.",
  unknown_policy: "Access could not be described for this workspace.",
  scope_and_role_satisfied: "Your account can use this workspace.",
  scope_missing: "Your account needs the required permission for this workspace.",
  role_missing: "This workspace needs an approved responsibility.",
  resource_context_required: "Choose the relevant record or jurisdiction to check access.",
};

const GERMAN_ACCESS_REASONS: Readonly<Record<UiAccessReasonCode, string>> = {
  sign_in_required: "Melden Sie sich an, bevor Sie diesen Arbeitsbereich verwenden.",
  public_access: "Dieser Arbeitsbereich ist ohne Anmeldung verfügbar.",
  server_evaluation_required: "Ihr Zugriff wird geprüft, bevor dieser Arbeitsbereich verwendet werden kann.",
  server_denied: "Ihr Konto kann diesen Arbeitsbereich derzeit nicht verwenden.",
  missing_expiry: "Für die Zugriffsprüfung fehlt ein Ablaufzeitpunkt; aktualisieren Sie die Prüfung.",
  clock_unavailable: "Der Zugriff kann nicht bestätigt werden, weil die Zeitprüfung nicht verfügbar ist.",
  access_expired: "Ihre Zugriffsprüfung ist nicht mehr aktuell; aktualisieren Sie sie.",
  missing_capability_decision: "Ihr Zugriff für diesen Arbeitsbereich ist noch nicht bestätigt.",
  unknown_policy: "Der Zugriff für diesen Arbeitsbereich konnte nicht beschrieben werden.",
  scope_and_role_satisfied: "Ihr Konto kann diesen Arbeitsbereich verwenden.",
  scope_missing: "Für Ihr Konto fehlt die erforderliche Berechtigung für diesen Arbeitsbereich.",
  role_missing: "Für diesen Arbeitsbereich ist eine genehmigte Verantwortung erforderlich.",
  resource_context_required: "Wählen Sie den relevanten Datensatz oder die Jurisdiktion, um den Zugriff zu prüfen.",
};

const ARABIC_ACCESS_REASONS: Readonly<Record<UiAccessReasonCode, string>> = {
  sign_in_required: "سجّل الدخول قبل استخدام مساحة العمل هذه.",
  public_access: "مساحة العمل هذه متاحة دون تسجيل الدخول.",
  server_evaluation_required: "يتم التحقق من وصولك قبل استخدام مساحة العمل هذه.",
  server_denied: "لا يمكن لحسابك استخدام مساحة العمل هذه حاليًا.",
  missing_expiry: "يفتقد فحص الوصول إلى وقت انتهاء؛ حدّث الفحص قبل المتابعة.",
  clock_unavailable: "لا يمكن تأكيد الوصول لأن فحص الوقت غير متاح.",
  access_expired: "لم يعد فحص الوصول محدثًا؛ حدّثه قبل المتابعة.",
  missing_capability_decision: "لم يتم تأكيد وصولك إلى مساحة العمل هذه بعد.",
  unknown_policy: "تعذر وصف الوصول إلى مساحة العمل هذه.",
  scope_and_role_satisfied: "يمكن لحسابك استخدام مساحة العمل هذه.",
  scope_missing: "يحتاج حسابك إلى الإذن المطلوب لمساحة العمل هذه.",
  role_missing: "تتطلب مساحة العمل هذه مسؤولية معتمدة.",
  resource_context_required: "اختر السجل أو الولاية القضائية ذات الصلة للتحقق من الوصول.",
};

const JAPANESE_ACCESS_REASONS: Readonly<Record<UiAccessReasonCode, string>> = {
  sign_in_required: "このワークスペースを使用するにはサインインしてください。",
  public_access: "このワークスペースはサインインなしで利用できます。",
  server_evaluation_required: "このワークスペースを使用する前にアクセスを確認しています。",
  server_denied: "現在、このアカウントではこのワークスペースを使用できません。",
  missing_expiry: "アクセス確認に有効期限がありません。続行する前に更新してください。",
  clock_unavailable: "時刻確認を利用できないため、アクセスを確認できません。",
  access_expired: "アクセス確認の有効期限が切れています。続行する前に更新してください。",
  missing_capability_decision: "このワークスペースへのアクセスはまだ確認されていません。",
  unknown_policy: "このワークスペースのアクセスを説明できません。",
  scope_and_role_satisfied: "このアカウントではこのワークスペースを使用できます。",
  scope_missing: "このワークスペースに必要な権限がアカウントにありません。",
  role_missing: "このワークスペースには承認された責任が必要です。",
  resource_context_required: "アクセスを確認するには対象の記録または管轄を選択してください。",
};

const ENGLISH_CHROME: ShellChromeMessages = {
  skipToWorkspace: "Skip to workspace content",
  productNavigation: "Product navigation",
  primaryNavigation: "Primary",
  account: "Account",
  openNavigation: "Open navigation",
  closeNavigation: "Close navigation",
  collapseNavigation: "Collapse navigation",
  expandNavigation: "Expand navigation",
  operations: "Operations",
  breadcrumb: "Breadcrumb",
  displaySettings: "Display settings",
  openDisplaySettings: "Open display settings",
  language: "Language",
  timeZone: "Time zone",
  theme: "Theme",
  themeSystem: "System default",
  themeLight: "Light",
  themeDark: "Dark",
  dateTimeHelp: (timeZone) => `Dates and times are shown in ${timeZone}. Currency values follow the selected language and account settings.`,
  themeHelp: "Theme changes appearance only.",
  unknownTimeZone: (fallback) => `Unknown time zone; ${fallback} is used for rendering.`,
  workspaces: (count) => `${count} workspaces`,
  yourWorkArea: "your work area",
  sidebarLegal: "Automated withholding and evidence—not tax filing or legal advice.",
};

const ENGLISH_SESSION: ShellSessionMessages = {
  localSignIn: "Local sign-in",
  openLocalSignIn: "Open local sign-in",
  signInCredential: "Sign-in credential (held in memory)",
  localCredentialPlaceholder: "Paste a local test credential",
  localDevelopmentHelp: "Local development only. This field is unavailable in staging and production builds.",
  signedInSession: "Signed-in session",
  signedIn: "Signed in",
  signOut: "Sign out",
  signIn: "Sign in",
  openingSignIn: "Opening sign-in…",
  signInSetupNeeded: "Sign-in setup needed",
  signInSetupNeededSummary: "Sign-in setup needed",
  secureSignInHelp: "Ask your administrator to configure secure sign-in for this deployment.",
  emailOtpSignIn: "Email sign-in",
  openEmailOtpSignIn: "Open email sign-in",
  emailLabel: "Email address",
  emailPlaceholder: "you@example.com",
  emailOtpHelp: "A one-time code is written to the local auth inbox in Docker.",
  sendCode: "Send code",
  sendingCode: "Sending code…",
  emailOtpSent: "Enter the six-digit code from the local auth inbox.",
  codeLabel: "One-time code",
  codePlaceholder: "123456",
  verifyCode: "Verify code",
  verifyingCode: "Verifying code…",
  changeEmail: "Use another email",
  protectedWorkspace: "Protected workspace",
  signInToContinue: "Sign in to continue",
  signInToContinueCopy: "Your account and tenant access are checked before protected actions become available.",
  signInInHeader: "Use the sign-in control in the header to continue.",
};

const ENGLISH: ShellMessages = {
  groups: ENGLISH_GROUPS,
  navigation: ENGLISH_NAVIGATION,
  riskLabels: ENGLISH_RISK_LABELS,
  accessStatus: ENGLISH_ACCESS_STATUS,
  accessReason: (reasonCode) => ENGLISH_ACCESS_REASONS[reasonCode],
  chrome: ENGLISH_CHROME,
  session: ENGLISH_SESSION,
  workspaceStatus: (label) => `${label} workspace`,
};

const GERMAN: ShellMessages = {
  groups: {
    "Daily work": "Tägliche Arbeit",
    "Rules & access": "Regeln & Zugriff",
    "Money movement": "Geldbewegung",
    "Records & issues": "Aufzeichnungen & Probleme",
  },
  navigation: {
    quote: { label: "Steuerangebote", description: "Ein Kundenangebot mit klarer Steueraufschlüsselung vorbereiten." },
    preview: { label: "Steuersimulator", description: "Ein Szenario prüfen, ohne veröffentlichte Regeln zu ändern." },
    "document-quote": { label: "Rechnungssteuer", description: "Steuern für eine Rechnung und ihre Positionen berechnen." },
    checkout: { label: "Zahlungsprüfung", description: "Zahlungsdetails prüfen, bevor der Kunde unterschreibt." },
    "document-checkout": { label: "Dokumentzahlung", description: "Die Zahlung für eine Rechnung prüfen und genehmigen." },
    governance: { label: "Steuerregeln", description: "Steuerregeln erstellen, prüfen, genehmigen und veröffentlichen." },
    "access-control": { label: "Zugriff & Genehmigungen", description: "Sehen, wer handeln darf, was genehmigt werden muss und warum." },
    "vault-release": { label: "Treasury", description: "Eine kontrollierte Treasury-Auszahlung vorbereiten und prüfen." },
    "vault-reconciliation": { label: "Saldenprüfung", description: "Erfasste Salden mit der letzten verifizierten Beobachtung vergleichen." },
    "revenue-streaming": { label: "Partnerauszahlungen", description: "Verwaltete Auszahlungspläne prüfen und eine verifizierte Auszahlung vorbereiten." },
    adjustments: { label: "Anpassungen", description: "Eine Korrektur für eine abgeschlossene Transaktion vorbereiten." },
    "refund-claims": { label: "Erstattungsanfragen", description: "Eine Erstattungsanfrage zur Prüfung vorbereiten." },
    "refund-cancellations": { label: "Erstattungsstornierungen", description: "Eine Erstattungsanfrage mit einem Genehmigungsnachweis stoppen." },
    "refund-executions": { label: "Erstattungszahlungen", description: "Eine genehmigte Erstattung nach Abschluss aller Prüfungen auszahlen." },
    "replay-ops": { label: "Ausnahmen & Wiederherstellung", description: "Datensätze mit Handlungsbedarf untersuchen und wiederherstellen." },
    "evidence-access": { label: "Nachweise & Audit", description: "Geschützte Nachweiszugriffe für dieses Konto prüfen." },
  },
  riskLabels: {
    read: "Schreibgeschützter Arbeitsbereich",
    write: "Änderungsaktion",
    financial: "Sensible Finanzaktion",
    governance: "Genehmigung kann erforderlich sein",
  },
  accessStatus: {
    available: "Verfügbar",
    unavailable: "Nicht verfügbar",
    needsCheck: "Zugriff muss geprüft werden",
    signInToCheck: "Zur Prüfung anmelden",
    unknownPolicy: "Zugriff konnte nicht beschrieben werden",
  },
  accessReason: (reasonCode) => GERMAN_ACCESS_REASONS[reasonCode],
  chrome: {
    skipToWorkspace: "Zum Arbeitsbereich springen",
    productNavigation: "Produktnavigation",
    primaryNavigation: "Hauptnavigation",
    account: "Konto",
    openNavigation: "Navigation öffnen",
    closeNavigation: "Navigation schließen",
    collapseNavigation: "Navigation einklappen",
    expandNavigation: "Navigation ausklappen",
    operations: "Vorgänge",
    breadcrumb: "Brotkrümelnavigation",
    displaySettings: "Anzeigeeinstellungen",
    openDisplaySettings: "Anzeigeeinstellungen öffnen",
    language: "Sprache",
    timeZone: "Zeitzone",
    theme: "Erscheinungsbild",
    themeSystem: "Systemstandard",
    themeLight: "Hell",
    themeDark: "Dunkel",
    dateTimeHelp: (timeZone) => `Datums- und Zeitangaben werden in ${timeZone} angezeigt. Währungswerte folgen der ausgewählten Sprache und den Kontoeinstellungen.`,
    themeHelp: "Das Erscheinungsbild ändert nur die Darstellung.",
    unknownTimeZone: (fallback) => `Unbekannte Zeitzone; ${fallback} wird für die Darstellung verwendet.`,
    workspaces: (count) => `${count} Arbeitsbereiche`,
    yourWorkArea: "Ihr Arbeitsbereich",
    sidebarLegal: "Automatisches Einbehalten und Nachweise—keine Steuererklärung und keine Rechtsberatung.",
  },
  session: {
    localSignIn: "Lokale Anmeldung",
    openLocalSignIn: "Lokale Anmeldung öffnen",
    signInCredential: "Anmeldeinformation (im Speicher gehalten)",
    localCredentialPlaceholder: "Lokale Test-Anmeldeinformation einfügen",
    localDevelopmentHelp: "Nur für die lokale Entwicklung. Dieses Feld ist in Staging- und Produktions-Builds nicht verfügbar.",
    signedInSession: "Angemeldete Sitzung",
    signedIn: "Angemeldet",
    signOut: "Abmelden",
    signIn: "Anmelden",
    openingSignIn: "Anmeldung wird geöffnet…",
    signInSetupNeeded: "Anmeldung muss eingerichtet werden",
    signInSetupNeededSummary: "Anmeldung muss eingerichtet werden",
    secureSignInHelp: "Bitten Sie die Administration, die sichere Anmeldung für diese Bereitstellung einzurichten.",
    emailOtpSignIn: "Anmeldung per E-Mail",
    openEmailOtpSignIn: "E-Mail-Anmeldung öffnen",
    emailLabel: "E-Mail-Adresse",
    emailPlaceholder: "sie@beispiel.de",
    emailOtpHelp: "Ein Einmalcode wird im lokalen Auth-Posteingang von Docker abgelegt.",
    sendCode: "Code senden",
    sendingCode: "Code wird gesendet…",
    emailOtpSent: "Geben Sie den sechsstelligen Code aus dem lokalen Auth-Posteingang ein.",
    codeLabel: "Einmalcode",
    codePlaceholder: "123456",
    verifyCode: "Code prüfen",
    verifyingCode: "Code wird geprüft…",
    changeEmail: "Andere E-Mail verwenden",
    protectedWorkspace: "Geschützter Arbeitsbereich",
    signInToContinue: "Zum Fortfahren anmelden",
    signInToContinueCopy: "Ihr Konto und Ihr Mandantenzugriff werden geprüft, bevor geschützte Aktionen verfügbar werden.",
    signInInHeader: "Verwenden Sie zum Fortfahren die Anmeldesteuerung in der Kopfzeile.",
  },
  workspaceStatus: (label) => `Arbeitsbereich: ${label}`,
};

const ARABIC: ShellMessages = {
  groups: {
    "Daily work": "العمل اليومي",
    "Rules & access": "القواعد والوصول",
    "Money movement": "حركة الأموال",
    "Records & issues": "السجلات والمشكلات",
  },
  navigation: {
    quote: { label: "عروض ضريبية", description: "إعداد عرض للعميل مع تفصيل واضح للضريبة." },
    preview: { label: "محاكي الضرائب", description: "استكشاف سيناريو دون تغيير القواعد المنشورة." },
    "document-quote": { label: "ضريبة الفاتورة", description: "حساب الضريبة على الفاتورة وبنودها." },
    checkout: { label: "مراجعة الدفع", description: "مراجعة تفاصيل الدفع قبل توقيع العميل." },
    "document-checkout": { label: "دفع المستند", description: "مراجعة دفع الفاتورة والموافقة عليه." },
    governance: { label: "القواعد الضريبية", description: "إنشاء القواعد الضريبية ومراجعتها والموافقة عليها ونشرها." },
    "access-control": { label: "الوصول والموافقات", description: "معرفة من يمكنه التصرف وما يحتاج إلى موافقة والسبب." },
    "vault-release": { label: "الخزانة", description: "إعداد دفعة خزانة خاضعة للرقابة ومراجعتها." },
    "vault-reconciliation": { label: "فحوصات الأرصدة", description: "مقارنة الأرصدة المسجلة بآخر ملاحظة تم التحقق منها." },
    "revenue-streaming": { label: "مدفوعات الشركاء", description: "مراجعة خطط الدفع المُدارة وإعداد دفعة تم التحقق منها." },
    adjustments: { label: "التسويات", description: "إعداد تصحيح لمعاملة مكتملة." },
    "refund-claims": { label: "طلبات الاسترداد", description: "إعداد طلب استرداد للمراجعة." },
    "refund-cancellations": { label: "إلغاء عمليات الاسترداد", description: "إيقاف طلب استرداد مع سجل موافقة." },
    "refund-executions": { label: "دفعات الاسترداد", description: "إطلاق استرداد معتمد بعد اجتياز جميع الفحوصات." },
    "replay-ops": { label: "الاستثناءات والاسترداد", description: "التحقيق في السجلات التي تحتاج إلى اهتمام واستردادها." },
    "evidence-access": { label: "الأدلة والتدقيق", description: "مراجعة نشاط الوصول إلى الأدلة المحمية لهذا الحساب." },
  },
  riskLabels: {
    read: "مساحة عمل للقراءة فقط",
    write: "إجراء تغيير",
    financial: "إجراء مالي حساس",
    governance: "قد تكون الموافقة مطلوبة",
  },
  accessStatus: {
    available: "متاح",
    unavailable: "غير متاح",
    needsCheck: "يلزم التحقق من الوصول",
    signInToCheck: "سجّل الدخول للتحقق",
    unknownPolicy: "تعذر وصف الوصول",
  },
  accessReason: (reasonCode) => ARABIC_ACCESS_REASONS[reasonCode],
  chrome: {
    skipToWorkspace: "انتقل إلى محتوى مساحة العمل",
    productNavigation: "تنقل المنتج",
    primaryNavigation: "التنقل الرئيسي",
    account: "الحساب",
    openNavigation: "فتح التنقل",
    closeNavigation: "إغلاق التنقل",
    collapseNavigation: "طي التنقل",
    expandNavigation: "توسيع التنقل",
    operations: "العمليات",
    breadcrumb: "مسار التنقل",
    displaySettings: "إعدادات العرض",
    openDisplaySettings: "فتح إعدادات العرض",
    language: "اللغة",
    timeZone: "المنطقة الزمنية",
    theme: "السمة",
    themeSystem: "إعداد النظام الافتراضي",
    themeLight: "فاتح",
    themeDark: "داكن",
    dateTimeHelp: (timeZone) => `تُعرض التواريخ والأوقات في ${timeZone}. تتبع قيم العملات اللغة المحددة وإعدادات الحساب.`,
    themeHelp: "تغيّر السمة المظهر فقط.",
    unknownTimeZone: (fallback) => `المنطقة الزمنية غير معروفة؛ يُستخدم ${fallback} للعرض.`,
    workspaces: (count) => `${count} مساحات عمل`,
    yourWorkArea: "مساحة عملك",
    sidebarLegal: "الاقتطاع التلقائي والأدلة—ليس إقرارًا ضريبيًا ولا استشارة قانونية.",
  },
  session: {
    localSignIn: "تسجيل الدخول المحلي",
    openLocalSignIn: "فتح تسجيل الدخول المحلي",
    signInCredential: "بيانات تسجيل الدخول (محفوظة في الذاكرة)",
    localCredentialPlaceholder: "ألصق بيانات اعتماد الاختبار المحلي",
    localDevelopmentHelp: "للتطوير المحلي فقط. هذا الحقل غير متاح في بيئات الاختبار والإنتاج.",
    signedInSession: "جلسة تسجيل دخول",
    signedIn: "تم تسجيل الدخول",
    signOut: "تسجيل الخروج",
    signIn: "تسجيل الدخول",
    openingSignIn: "جارٍ فتح تسجيل الدخول…",
    signInSetupNeeded: "يلزم إعداد تسجيل الدخول",
    signInSetupNeededSummary: "يلزم إعداد تسجيل الدخول",
    secureSignInHelp: "اطلب من المسؤول إعداد تسجيل الدخول الآمن لهذا النشر.",
    emailOtpSignIn: "تسجيل الدخول بالبريد الإلكتروني",
    openEmailOtpSignIn: "فتح تسجيل الدخول بالبريد الإلكتروني",
    emailLabel: "عنوان البريد الإلكتروني",
    emailPlaceholder: "you@example.com",
    emailOtpHelp: "يُكتب الرمز لمرة واحدة في صندوق مصادقة Docker المحلي.",
    sendCode: "إرسال الرمز",
    sendingCode: "جارٍ إرسال الرمز…",
    emailOtpSent: "أدخل الرمز المكوّن من ستة أرقام من صندوق المصادقة المحلي.",
    codeLabel: "الرمز لمرة واحدة",
    codePlaceholder: "123456",
    verifyCode: "التحقق من الرمز",
    verifyingCode: "جارٍ التحقق من الرمز…",
    changeEmail: "استخدم بريدًا إلكترونيًا آخر",
    protectedWorkspace: "مساحة عمل محمية",
    signInToContinue: "سجّل الدخول للمتابعة",
    signInToContinueCopy: "يتم التحقق من حسابك ووصولك إلى المستأجر قبل إتاحة الإجراءات المحمية.",
    signInInHeader: "استخدم أداة تسجيل الدخول في الرأس للمتابعة.",
  },
  workspaceStatus: (label) => `مساحة العمل: ${label}`,
};

const JAPANESE: ShellMessages = {
  groups: {
    "Daily work": "日常業務",
    "Rules & access": "ルールとアクセス",
    "Money movement": "資金移動",
    "Records & issues": "記録と問題",
  },
  navigation: {
    quote: { label: "税額見積もり", description: "税額の内訳が明確な顧客向け見積もりを準備します。" },
    preview: { label: "税額シミュレーター", description: "公開済みのルールを変更せずにシナリオを確認します。" },
    "document-quote": { label: "請求書の税額", description: "請求書と明細の税額を計算します。" },
    checkout: { label: "支払い確認", description: "顧客が署名する前に支払いの詳細を確認します。" },
    "document-checkout": { label: "書類の支払い", description: "請求書の支払いを確認して承認します。" },
    governance: { label: "税ルール", description: "税ルールを作成、確認、承認、公開します。" },
    "access-control": { label: "アクセスと承認", description: "誰が操作できるか、何に承認が必要か、その理由を確認します。" },
    "vault-release": { label: "財務", description: "管理された財務資金の支払いを準備して確認します。" },
    "vault-reconciliation": { label: "残高確認", description: "記録された残高を最新の検証済み観測値と比較します。" },
    "revenue-streaming": { label: "パートナー支払い", description: "管理された支払い計画を確認し、検証済みの支払いを準備します。" },
    adjustments: { label: "調整", description: "完了した取引の訂正を準備します。" },
    "refund-claims": { label: "返金申請", description: "確認用の返金申請を準備します。" },
    "refund-cancellations": { label: "返金の取り消し", description: "承認記録を付けて返金申請を停止します。" },
    "refund-executions": { label: "返金支払い", description: "すべての確認後に承認済みの返金を実行します。" },
    "replay-ops": { label: "例外と復旧", description: "対応が必要な記録を調査して復旧します。" },
    "evidence-access": { label: "証拠と監査", description: "このアカウントの保護された証拠へのアクセス活動を確認します。" },
  },
  riskLabels: {
    read: "読み取り専用ワークスペース",
    write: "変更操作",
    financial: "重要な財務操作",
    governance: "承認が必要になる場合があります",
  },
  accessStatus: {
    available: "利用可能",
    unavailable: "利用不可",
    needsCheck: "アクセス確認が必要です",
    signInToCheck: "サインインして確認",
    unknownPolicy: "アクセスを説明できません",
  },
  accessReason: (reasonCode) => JAPANESE_ACCESS_REASONS[reasonCode],
  chrome: {
    skipToWorkspace: "ワークスペースのコンテンツへ移動",
    productNavigation: "プロダクトナビゲーション",
    primaryNavigation: "メインナビゲーション",
    account: "アカウント",
    openNavigation: "ナビゲーションを開く",
    closeNavigation: "ナビゲーションを閉じる",
    collapseNavigation: "ナビゲーションを折りたたむ",
    expandNavigation: "ナビゲーションを展開する",
    operations: "オペレーション",
    breadcrumb: "パンくずリスト",
    displaySettings: "表示設定",
    openDisplaySettings: "表示設定を開く",
    language: "言語",
    timeZone: "タイムゾーン",
    theme: "テーマ",
    themeSystem: "システム設定",
    themeLight: "ライト",
    themeDark: "ダーク",
    dateTimeHelp: (timeZone) => `日付と時刻は${timeZone}で表示されます。通貨の値は選択した言語とアカウント設定に従います。`,
    themeHelp: "テーマは表示だけを変更します。",
    unknownTimeZone: (fallback) => `不明なタイムゾーンです。表示には${fallback}を使用します。`,
    workspaces: (count) => `${count}件のワークスペース`,
    yourWorkArea: "作業エリア",
    sidebarLegal: "自動源泉徴収と証拠—税務申告や法的助言ではありません。",
  },
  session: {
    localSignIn: "ローカルサインイン",
    openLocalSignIn: "ローカルサインインを開く",
    signInCredential: "サインイン資格情報（メモリ内保持）",
    localCredentialPlaceholder: "ローカルテスト資格情報を貼り付け",
    localDevelopmentHelp: "ローカル開発専用です。この項目はステージングと本番ビルドでは利用できません。",
    signedInSession: "サインイン済みセッション",
    signedIn: "サインイン済み",
    signOut: "サインアウト",
    signIn: "サインイン",
    openingSignIn: "サインインを開いています…",
    signInSetupNeeded: "サインイン設定が必要です",
    signInSetupNeededSummary: "サインイン設定が必要です",
    secureSignInHelp: "管理者にこのデプロイメントの安全なサインイン設定を依頼してください。",
    emailOtpSignIn: "メールでサインイン",
    openEmailOtpSignIn: "メールサインインを開く",
    emailLabel: "メールアドレス",
    emailPlaceholder: "you@example.com",
    emailOtpHelp: "ワンタイムコードはDockerのローカル認証受信箱に書き込まれます。",
    sendCode: "コードを送信",
    sendingCode: "コードを送信しています…",
    emailOtpSent: "ローカル認証受信箱の6桁コードを入力してください。",
    codeLabel: "ワンタイムコード",
    codePlaceholder: "123456",
    verifyCode: "コードを確認",
    verifyingCode: "コードを確認しています…",
    changeEmail: "別のメールを使用",
    protectedWorkspace: "保護されたワークスペース",
    signInToContinue: "続行するにはサインインしてください",
    signInToContinueCopy: "保護された操作を利用できるようにする前に、アカウントとテナントへのアクセスを確認します。",
    signInInHeader: "続行するにはヘッダーのサインイン操作を使用してください。",
  },
  workspaceStatus: (label) => `${label}ワークスペース`,
};

const CATALOGS: Readonly<Partial<Record<SupportedLocale, ShellMessages>>> = Object.freeze({
  "en-US": ENGLISH,
  "de-DE": GERMAN,
  "ar-EG": ARABIC,
  "ja-JP": JAPANESE,
});

export function shellMessagesFor(locale: SupportedLocale | string): ShellMessages {
  return CATALOGS[locale as SupportedLocale] ?? ENGLISH;
}
