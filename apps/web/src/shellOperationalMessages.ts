import { DEFAULT_LOCALE } from "./formatters";

export interface ShellOperationalMessages {
  system: {
    checking: string;
    ready: string;
    attention: string;
    label: string;
    details: string;
    deployment: string;
    confirmed: string;
    configureBuild: string;
    network: string;
    configureNetwork: string;
    api: string;
    checkingServices: string;
    operational: string;
    unavailable: string;
    serviceResponded: string;
    serviceChecking: string;
    retryLater: string;
    contextNote: string;
    session: string;
    signedIn: string;
    signInRequired: string;
    protectedRefresh: string;
    signInToSee: string;
    itemsNeedAttention: (count: number) => string;
  };
  account: {
    openMenu: string;
    label: string;
    accessNeeded: string;
    publicAccess: string;
    publicAccessDescription: string;
    accessUnavailable: string;
    accessChecked: string;
    accessChecking: string;
    continueAfterSignIn: string;
    protectedUnavailable: string;
    waitingForService: string;
    accessPrefix: string;
    sensitivityPrefix: string;
    signInRequired: string;
    confirmed: string;
    contextAfterSignIn: string;
    identityNotConfirmed: string;
    notSignedIn: string;
    copy: string;
    signedInAs: string;
    accessCheck: string;
    responsibilities: string;
    roleLabel: (role: string) => string;
    reviewAccess: string;
    signOut: string;
    signInToManage: string;
  };
  authorization: {
    unavailable: string;
    expired: string;
    missingExpiry: string;
  };
  protection: {
    yourAccess: string;
    protectedActions: string;
    open: string;
    label: string;
    accountAccess: string;
    decisionRecords: string;
    everyDecision: string;
    sensitiveActions: string;
    sensitiveNeed: string;
    walletCustody: string;
    walletsOwner: string;
  };
  footer: { product: string; operations: string; traceable: string };
  loading: { title: (workspace: string) => string; copy: string };
  error: { eyebrow: string; title: string; copy: (workspace: string) => string; reload: string };
}

function localizedRoleLabel(role: string, labels: Readonly<Record<string, string>>): string {
  const knownLabel = labels[role];
  if (knownLabel) return knownLabel;
  const boundedRole = role.slice(0, 64);
  return boundedRole.replace(/_/gu, " ");
}

const ENGLISH_ROLE_LABELS = {
  administrator: "Administrator",
  merchant_operator: "Operator",
  moderator: "Moderator",
  tax_rule_author: "Tax rule author",
  tax_rule_reviewer: "Tax rule reviewer",
  tax_rule_approver: "Tax rule approver",
  tax_applicability_author: "Tax treatment author",
  tax_applicability_reviewer: "Tax treatment reviewer",
  tax_applicability_approver: "Tax treatment approver",
  vault_release_proposer: "Treasury release proposer",
  treasury_proposer: "Treasury proposer",
  treasury_approver: "Treasury approver",
  revenue_stream_proposer: "Revenue stream proposer",
  revenue_stream_approver: "Revenue stream approver",
  revenue_stream_operator: "Revenue stream operator",
  revenue_stream_governance: "Revenue stream governance",
  revenue_stream_auditor: "Revenue stream auditor",
  settlement_adjustment_proposer: "Adjustment proposer",
  settlement_adjustment_governance: "Adjustment governance",
  settlement_adjustment_approver: "Adjustment approver",
  indexer_operator: "Indexer operator",
  revenue_stream_payout: "Revenue stream payout",
  refund_claim_operator: "Refund claim operator",
  refund_executor: "Refund executor",
  evidence_reviewer: "Evidence reviewer",
} as const;

const GERMAN_ROLE_LABELS = {
  administrator: "Administrator",
  merchant_operator: "Operator",
  moderator: "Moderator",
  tax_rule_reviewer: "Prüfer für Steuerregeln",
  tax_rule_approver: "Genehmiger für Steuerregeln",
  treasury_proposer: "Treasury-Vorschlagender",
  treasury_approver: "Treasury-Genehmiger",
  revenue_stream_proposer: "Vorschlagender für Einnahmenströme",
  revenue_stream_payout: "Auszahlung für Einnahmenströme",
  refund_claim_operator: "Bearbeiter für Erstattungsanträge",
  refund_executor: "Erstattungs-Ausführer",
  evidence_reviewer: "Nachweisprüfer",
} as const;

const ARABIC_ROLE_LABELS = {
  administrator: "مسؤول النظام",
  merchant_operator: "مشغّل",
  moderator: "مشرف",
  tax_rule_reviewer: "مراجع قواعد الضرائب",
  tax_rule_approver: "معتمد قواعد الضرائب",
  treasury_proposer: "مقترح الخزانة",
  treasury_approver: "معتمد الخزانة",
  revenue_stream_proposer: "مقترح تدفق الإيرادات",
  revenue_stream_payout: "مدفوعات تدفق الإيرادات",
  refund_claim_operator: "مشغّل مطالبات الاسترداد",
  refund_executor: "منفّذ الاسترداد",
  evidence_reviewer: "مراجع الأدلة",
} as const;

const JAPANESE_ROLE_LABELS = {
  administrator: "管理者",
  merchant_operator: "オペレーター",
  moderator: "モデレーター",
  tax_rule_reviewer: "税務ルール審査担当",
  tax_rule_approver: "税務ルール承認担当",
  treasury_proposer: "財務提案担当",
  treasury_approver: "財務承認担当",
  revenue_stream_proposer: "収益ストリーム提案担当",
  revenue_stream_payout: "収益ストリーム支払担当",
  refund_claim_operator: "返金申請担当",
  refund_executor: "返金実行担当",
  evidence_reviewer: "証跡審査担当",
} as const;

const ENGLISH: ShellOperationalMessages = {
  system: { checking: "Checking", ready: "Ready", attention: "Needs attention", label: "System status", details: "System status details", deployment: "Deployment environment", confirmed: "Release context confirmed.", configureBuild: "Ask the release owner to configure this build.", network: "Solana network", configureNetwork: "Ask the release owner to configure the target network.", api: "API services", checkingServices: "Checking services", operational: "All systems operational", unavailable: "Some services unavailable", serviceResponded: "The service responded to the latest check.", serviceChecking: "The service check is still running.", retryLater: "Try again later; no client-side fallback is used.", contextNote: "These details describe the current release and service state. They do not grant access.", session: "Account session", signedIn: "Signed in", signInRequired: "Sign-in required", protectedRefresh: "Protected actions still receive a fresh server decision.", signInToSee: "Sign in to see account actions.", itemsNeedAttention: (count) => `${count} items need attention` },
  account: { openMenu: "Open account menu", label: "Account", accessNeeded: "Sign-in needed", publicAccess: "Available without sign-in", publicAccessDescription: "This simulator does not change live rules or authorize payment.", accessUnavailable: "Access check unavailable", accessChecked: "Access checked", accessChecking: "Checking access", continueAfterSignIn: "Sign in to continue", protectedUnavailable: "Protected actions stay unavailable", waitingForService: "Waiting for the service", accessPrefix: "Access", sensitivityPrefix: "Action sensitivity", signInRequired: "Sign-in required", confirmed: "Account confirmed by the service", contextAfterSignIn: "Account context appears after sign-in", identityNotConfirmed: "Identity not confirmed", notSignedIn: "Not signed in", copy: "This menu shows session context; it cannot grant access or approve an action.", signedInAs: "Signed in as", accessCheck: "Access check", responsibilities: "Responsibilities", roleLabel: (role) => localizedRoleLabel(role, ENGLISH_ROLE_LABELS), reviewAccess: "Review access & approvals", signOut: "Sign out", signInToManage: "Sign in to manage this account." },
  authorization: { unavailable: "We couldn't confirm access right now. Protected actions stay unavailable.", expired: "Your access check is out of date. We are refreshing access before you continue.", missingExpiry: "We couldn't confirm how long your access is valid. Protected actions stay unavailable until it is refreshed." },
  protection: { yourAccess: "Your access", protectedActions: "Protected actions may need review or approval.", open: "Open protection details", label: "Protection details", accountAccess: "Account access", decisionRecords: "Decision records", everyDecision: "Every decision has a record.", sensitiveActions: "Sensitive actions", sensitiveNeed: "Sensitive actions need approval.", walletCustody: "Wallet custody", walletsOwner: "Wallets stay with their owners." },
  footer: { product: "Solana Tax Protocol", operations: "tax operations", traceable: "Every important action has a traceable record." }, loading: { title: (workspace) => `Loading ${workspace}…`, copy: "This workspace is loading its server-owned controls. No client-side authority is inferred." }, error: { eyebrow: "Workspace unavailable", title: "This workspace could not be loaded", copy: (workspace) => `The ${workspace} workspace failed before it could render. No calculation, wallet action, settlement, or evidence state was inferred.`, reload: "Reload application" },
};

const GERMAN: ShellOperationalMessages = {
  system: { checking: "Wird geprüft", ready: "Bereit", attention: "Aufmerksamkeit erforderlich", label: "Systemstatus", details: "Details zum Systemstatus", deployment: "Bereitstellungsumgebung", confirmed: "Release-Kontext bestätigt.", configureBuild: "Bitten Sie die Release-Verantwortlichen, diesen Build zu konfigurieren.", network: "Solana-Netzwerk", configureNetwork: "Bitten Sie die Release-Verantwortlichen, das Zielnetzwerk zu konfigurieren.", api: "API-Dienste", checkingServices: "Dienste werden geprüft", operational: "Alle Systeme betriebsbereit", unavailable: "Einige Dienste nicht verfügbar", serviceResponded: "Der Dienst hat auf die letzte Prüfung geantwortet.", serviceChecking: "Die Dienstprüfung läuft noch.", retryLater: "Versuchen Sie es später erneut; es wird kein Fallback im Browser verwendet.", contextNote: "Diese Details beschreiben den aktuellen Release- und Dienststatus. Sie gewähren keinen Zugriff.", session: "Kontositzung", signedIn: "Angemeldet", signInRequired: "Anmeldung erforderlich", protectedRefresh: "Geschützte Aktionen erhalten weiterhin eine neue Entscheidung vom Dienst.", signInToSee: "Melden Sie sich an, um Kontaktionen zu sehen.", itemsNeedAttention: (count) => `${count} Punkte benötigen Aufmerksamkeit` },
  account: { openMenu: "Kontomenü öffnen", label: "Konto", accessNeeded: "Anmeldung erforderlich", publicAccess: "Ohne Anmeldung verfügbar", publicAccessDescription: "Dieser Simulator ändert keine aktiven Regeln und genehmigt keine Zahlung.", accessUnavailable: "Zugriffsprüfung nicht verfügbar", accessChecked: "Zugriff geprüft", accessChecking: "Zugriff wird geprüft", continueAfterSignIn: "Zur Fortsetzung anmelden", protectedUnavailable: "Geschützte Aktionen bleiben nicht verfügbar", waitingForService: "Warten auf den Dienst", accessPrefix: "Zugriff", sensitivityPrefix: "Aktionssensibilität", signInRequired: "Anmeldung erforderlich", confirmed: "Konto vom Dienst bestätigt", contextAfterSignIn: "Kontokontext erscheint nach der Anmeldung", identityNotConfirmed: "Identität nicht bestätigt", notSignedIn: "Nicht angemeldet", copy: "Dieses Menü zeigt den Sitzungskontext; es kann keinen Zugriff gewähren oder eine Aktion genehmigen.", signedInAs: "Angemeldet als", accessCheck: "Zugriffsprüfung", responsibilities: "Verantwortlichkeiten", roleLabel: (role) => localizedRoleLabel(role, GERMAN_ROLE_LABELS), reviewAccess: "Zugriff & Genehmigungen prüfen", signOut: "Abmelden", signInToManage: "Melden Sie sich an, um dieses Konto zu verwalten." },
  authorization: { unavailable: "Der Zugriff konnte gerade nicht bestätigt werden. Geschützte Aktionen bleiben nicht verfügbar.", expired: "Ihre Zugriffsprüfung ist nicht mehr aktuell. Der Zugriff wird aktualisiert, bevor Sie fortfahren.", missingExpiry: "Die Gültigkeitsdauer Ihres Zugriffs konnte nicht bestätigt werden. Geschützte Aktionen bleiben bis zur Aktualisierung nicht verfügbar." },
  protection: { yourAccess: "Ihr Zugriff", protectedActions: "Geschützte Aktionen benötigen möglicherweise Prüfung oder Genehmigung.", open: "Schutzdetails öffnen", label: "Schutzdetails", accountAccess: "Kontozugriff", decisionRecords: "Entscheidungsnachweise", everyDecision: "Jede Entscheidung hat einen Nachweis.", sensitiveActions: "Sensible Aktionen", sensitiveNeed: "Sensible Aktionen benötigen eine Genehmigung.", walletCustody: "Wallet-Verwahrung", walletsOwner: "Wallets bleiben bei ihren Eigentümern." },
  footer: { product: "Solana Tax Protocol", operations: "Steuervorgänge", traceable: "Jede wichtige Aktion hat einen nachvollziehbaren Nachweis." }, loading: { title: (workspace) => `${workspace} wird geladen…`, copy: "Dieser Arbeitsbereich lädt seine vom Dienst gesteuerten Funktionen. Im Browser wird keine Autorität abgeleitet." }, error: { eyebrow: "Arbeitsbereich nicht verfügbar", title: "Dieser Arbeitsbereich konnte nicht geladen werden", copy: (workspace) => `Der Arbeitsbereich ${workspace} konnte vor dem Anzeigen nicht geladen werden. Es wurde kein Rechen-, Wallet-, Abwicklungs- oder Nachweisstatus abgeleitet.`, reload: "Anwendung neu laden" },
};

const ARABIC: ShellOperationalMessages = {
  system: { checking: "جارٍ التحقق", ready: "جاهز", attention: "يتطلب الانتباه", label: "حالة النظام", details: "تفاصيل حالة النظام", deployment: "بيئة النشر", confirmed: "تم تأكيد سياق الإصدار.", configureBuild: "اطلب من مسؤول الإصدار إعداد هذا البناء.", network: "شبكة Solana", configureNetwork: "اطلب من مسؤول الإصدار إعداد الشبكة المستهدفة.", api: "خدمات API", checkingServices: "جارٍ فحص الخدمات", operational: "جميع الأنظمة تعمل", unavailable: "بعض الخدمات غير متاحة", serviceResponded: "استجابت الخدمة لآخر فحص.", serviceChecking: "لا يزال فحص الخدمة جاريًا.", retryLater: "حاول مرة أخرى لاحقًا؛ لا يُستخدم بديل محلي في المتصفح.", contextNote: "توضح هذه التفاصيل الإصدار الحالي وحالة الخدمة، ولا تمنح الوصول.", session: "جلسة الحساب", signedIn: "تم تسجيل الدخول", signInRequired: "يلزم تسجيل الدخول", protectedRefresh: "ستتلقى الإجراءات المحمية قرارًا جديدًا من الخدمة.", signInToSee: "سجل الدخول لرؤية إجراءات الحساب.", itemsNeedAttention: (count) => `${count} عناصر تتطلب الانتباه` },
  account: { openMenu: "فتح قائمة الحساب", label: "الحساب", accessNeeded: "يلزم تسجيل الدخول", publicAccess: "متاح دون تسجيل الدخول", publicAccessDescription: "هذا المحاكي لا يغيّر القواعد الحية ولا يعتمد أي دفعة.", accessUnavailable: "فحص الوصول غير متاح", accessChecked: "تم فحص الوصول", accessChecking: "جارٍ فحص الوصول", continueAfterSignIn: "سجل الدخول للمتابعة", protectedUnavailable: "تبقى الإجراءات المحمية غير متاحة", waitingForService: "بانتظار الخدمة", accessPrefix: "الوصول", sensitivityPrefix: "حساسية الإجراء", signInRequired: "يلزم تسجيل الدخول", confirmed: "أكدت الخدمة الحساب", contextAfterSignIn: "يظهر سياق الحساب بعد تسجيل الدخول", identityNotConfirmed: "لم يتم تأكيد الهوية", notSignedIn: "لم يتم تسجيل الدخول", copy: "تعرض هذه القائمة سياق الجلسة؛ ولا يمكنها منح الوصول أو اعتماد إجراء.", signedInAs: "تم تسجيل الدخول باسم", accessCheck: "فحص الوصول", responsibilities: "المسؤوليات", roleLabel: (role) => localizedRoleLabel(role, ARABIC_ROLE_LABELS), reviewAccess: "مراجعة الوصول والموافقات", signOut: "تسجيل الخروج", signInToManage: "سجل الدخول لإدارة هذا الحساب." },
  authorization: { unavailable: "تعذر تأكيد الوصول الآن. ستبقى الإجراءات المحمية غير متاحة.", expired: "انتهت صلاحية فحص الوصول. جارٍ تحديث الوصول قبل المتابعة.", missingExpiry: "تعذر تأكيد مدة صلاحية الوصول. ستبقى الإجراءات المحمية غير متاحة حتى يتم التحديث." },
  protection: { yourAccess: "وصولك", protectedActions: "قد تحتاج الإجراءات المحمية إلى مراجعة أو موافقة.", open: "فتح تفاصيل الحماية", label: "تفاصيل الحماية", accountAccess: "الوصول إلى الحساب", decisionRecords: "سجلات القرار", everyDecision: "لكل قرار سجل.", sensitiveActions: "الإجراءات الحساسة", sensitiveNeed: "تحتاج الإجراءات الحساسة إلى موافقة.", walletCustody: "حيازة المحفظة", walletsOwner: "تبقى المحافظ مع مالكيها." },
  footer: { product: "Solana Tax Protocol", operations: "عمليات الضرائب", traceable: "لكل إجراء مهم سجل قابل للتتبع." }, loading: { title: (workspace) => `جارٍ تحميل ${workspace}…`, copy: "تُحمّل مساحة العمل عناصر التحكم التي تديرها الخدمة. لا يتم استنتاج أي سلطة من المتصفح." }, error: { eyebrow: "مساحة العمل غير متاحة", title: "تعذر تحميل مساحة العمل هذه", copy: (workspace) => `تعذر عرض مساحة العمل ${workspace}. لم يتم استنتاج أي حالة للحساب أو المحفظة أو التسوية أو الأدلة.`, reload: "إعادة تحميل التطبيق" },
};

const JAPANESE: ShellOperationalMessages = {
  system: { checking: "確認中", ready: "準備完了", attention: "要対応", label: "システム状態", details: "システム状態の詳細", deployment: "デプロイ環境", confirmed: "リリースコンテキストを確認済みです。", configureBuild: "リリース担当者にこのビルドの設定を依頼してください。", network: "Solanaネットワーク", configureNetwork: "リリース担当者に対象ネットワークの設定を依頼してください。", api: "APIサービス", checkingServices: "サービスを確認中", operational: "すべてのシステムが稼働中", unavailable: "一部のサービスを利用できません", serviceResponded: "サービスは最新の確認に応答しました。", serviceChecking: "サービス確認はまだ実行中です。", retryLater: "後でもう一度お試しください。ブラウザー側の代替処理は使用しません。", contextNote: "これらの詳細は現在のリリースとサービス状態を示します。アクセス権を付与するものではありません。", session: "アカウントセッション", signedIn: "サインイン済み", signInRequired: "サインインが必要です", protectedRefresh: "保護された操作には引き続き新しいサービス判定が適用されます。", signInToSee: "サインインするとアカウント操作を確認できます。", itemsNeedAttention: (count) => `${count} 件の対応項目` },
  account: { openMenu: "アカウントメニューを開く", label: "アカウント", accessNeeded: "サインインが必要です", publicAccess: "サインインなしで利用可能", publicAccessDescription: "このシミュレーターは実稼働ルールを変更せず、支払いも承認しません。", accessUnavailable: "アクセス確認を利用できません", accessChecked: "アクセス確認済み", accessChecking: "アクセスを確認中", continueAfterSignIn: "サインインして続行", protectedUnavailable: "保護された操作は利用できません", waitingForService: "サービスを待機中", accessPrefix: "アクセス", sensitivityPrefix: "操作の重要度", signInRequired: "サインインが必要です", confirmed: "サービスでアカウントを確認済み", contextAfterSignIn: "サインイン後にアカウントコンテキストを表示します", identityNotConfirmed: "本人確認が完了していません", notSignedIn: "サインインしていません", copy: "このメニューはセッションコンテキストを表示します。アクセスを付与したり操作を承認したりするものではありません。", signedInAs: "サインイン中の担当者", accessCheck: "アクセス確認", responsibilities: "担当責任", roleLabel: (role) => localizedRoleLabel(role, JAPANESE_ROLE_LABELS), reviewAccess: "アクセスと承認を確認", signOut: "サインアウト", signInToManage: "サインインしてアカウントを管理してください。" },
  authorization: { unavailable: "現在、アクセスを確認できません。保護された操作は利用できません。", expired: "アクセス確認の有効期限が切れています。続行する前にアクセスを更新しています。", missingExpiry: "アクセスの有効期間を確認できません。更新されるまで保護された操作は利用できません。" },
  protection: { yourAccess: "アクセス権", protectedActions: "保護された操作には確認または承認が必要な場合があります。", open: "保護の詳細を開く", label: "保護の詳細", accountAccess: "アカウントアクセス", decisionRecords: "判断記録", everyDecision: "すべての判断に記録があります。", sensitiveActions: "重要な操作", sensitiveNeed: "重要な操作には承認が必要です。", walletCustody: "ウォレット管理", walletsOwner: "ウォレットは所有者が管理します。" },
  footer: { product: "Solana Tax Protocol", operations: "税務オペレーション", traceable: "重要な操作には追跡可能な記録があります。" }, loading: { title: (workspace) => `${workspace}を読み込み中…`, copy: "このワークスペースはサービス管理の操作を読み込んでいます。ブラウザー側で権限を推測しません。" }, error: { eyebrow: "ワークスペースを利用できません", title: "このワークスペースを読み込めませんでした", copy: (workspace) => `${workspace}ワークスペースを表示できませんでした。計算、ウォレット操作、決済、証拠の状態は推測していません。`, reload: "アプリケーションを再読み込み" },
};

const CATALOGS: Record<string, ShellOperationalMessages> = { "en-US": ENGLISH, "de-DE": GERMAN, "ar-EG": ARABIC, "ja-JP": JAPANESE };

export function shellOperationalMessagesFor(locale: string): ShellOperationalMessages {
  return CATALOGS[locale] ?? CATALOGS[DEFAULT_LOCALE] ?? ENGLISH;
}
