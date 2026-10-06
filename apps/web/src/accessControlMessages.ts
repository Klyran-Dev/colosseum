import type { WorkspaceGroup } from "./authorization";
import { DEFAULT_LOCALE } from "./formatters";

interface PolicyCopy {
  label: string;
  description: string;
}

export interface AccessControlMessages {
  ariaLabel: string;
  overview: {
    eyebrow: string;
    title: string;
    status: { signInNeeded: string; unavailable: string; checked: string; checking: string };
    copy: string;
    person: string;
    notSignedIn: string;
    account: string;
    accountConfirmed: string;
    accountNeedsSignIn: string;
    permission: string;
    permissionUnavailable: string;
    permissionChecked: string;
    permissionWaiting: string;
    permissionSignIn: string;
    dimensions: { roles: string; rolesCopy: string; permissions: string; permissionsCopy: string; conditions: string; conditionsCopy: string };
    technicalSummary: string;
    technicalCopy: string;
    safeTitle: string;
    safeUnavailable: string;
    safeDefault: string;
    noBrowserAuthority: string;
  };
  rules: {
    eyebrow: string;
    title: string;
    count: (count: number) => string;
    copy: string;
    technicalSummary: string;
    none: string;
  };
  groups: Record<WorkspaceGroup, string>;
  decisions: { available: string; unavailable: string; needsContext: string; checking: string; signIn: string };
  risks: { financial: string; governance: string; write: string; read: string };
  policies: Record<string, PolicyCopy>;
}

const ENGLISH: AccessControlMessages = {
  ariaLabel: "Access and approvals workspace",
  overview: {
    eyebrow: "Access & approvals", title: "Who can do what",
    status: { signInNeeded: "Sign in needed", unavailable: "Access check unavailable", checked: "Access checked when you act", checking: "Checking access" },
    copy: "Your access is checked every time you open a protected action. People can have different responsibilities, some actions need a second approval, and access can depend on the account, environment, and record.",
    person: "Signed-in person", notSignedIn: "Not signed in", account: "Account", accountConfirmed: "Confirmed for this account", accountNeedsSignIn: "Confirmed when you sign in", permission: "Permission status", permissionUnavailable: "Unavailable; actions stay closed", permissionChecked: "Checked by the service", permissionWaiting: "Waiting for the service", permissionSignIn: "Waiting for sign-in",
    dimensions: { roles: "Roles", rolesCopy: "Your responsibilities determine which work you can own or approve.", permissions: "Permissions", permissionsCopy: "Each action has a specific permission instead of broad access.", conditions: "Conditions", conditionsCopy: "Account, environment, record status, and risk can change the decision." },
    technicalSummary: "How access is decided", technicalCopy: "RBAC checks responsibilities, PBAC checks named permissions, and ABAC checks the account and record conditions. These are technical names for the three plain-language checks above.", safeTitle: "Safe by default.", safeUnavailable: "The access check is unavailable, so protected actions stay unavailable.", safeDefault: "If the service cannot confirm access, the action stays unavailable.", noBrowserAuthority: "No approval, payment, or record is created in the browser.",
  },
  rules: { eyebrow: "Access rules", title: "Actions and approvals", count: (count) => `${count} rules`, copy: "These rules explain which work is available and where another person or a verified record is required.", technicalSummary: "Technical details for administrators", none: "None" },
  groups: { "Daily work": "Daily work", "Rules & access": "Rules & access", "Money movement": "Money movement", "Records & issues": "Records & issues" },
  decisions: { available: "Available", unavailable: "Not available", needsContext: "Needs context", checking: "Checking", signIn: "Sign in to check" },
  risks: { financial: "Sensitive", governance: "Approval", write: "Change", read: "View" },
  policies: {
    "tax.quote.create": { label: "Create tax quote", description: "Prepare a customer quote with a clear tax breakdown." },
    "tax.document.quote.create": { label: "Calculate invoice tax", description: "Calculate tax across an invoice and its lines." },
    "tax.preview.run": { label: "Explore a scenario", description: "Try a scenario without changing published rules or payments." },
    "tax.rule.govern": { label: "Manage tax rules", description: "Create, review, approve, and publish tax rules." },
    "settlement.wallet": { label: "Review payment", description: "Check the payment details before the customer signs." },
    "settlement.document.wallet": { label: "Review document payment", description: "Review and approve payment for an invoice." },
    "vault.reconciliation": { label: "Review balance checks", description: "Compare recorded balances with the latest verified observation." },
    "vault.release": { label: "Release treasury funds", description: "Prepare a controlled treasury release for approval." },
    "revenue_stream.read": { label: "View partner payout plans", description: "Review governed partner payout plans and their verified balances." },
    "revenue_stream.propose": { label: "Create partner payout plan", description: "Propose a bounded payout plan for a partner." },
    "revenue_stream.govern": { label: "Approve partner payout plan", description: "Review and govern a partner payout plan." },
    "revenue_stream.claim.prepare": { label: "Prepare partner payout release", description: "Reserve an available amount for a verified release." },
    "settlement.adjustment": { label: "Prepare an adjustment", description: "Correct a completed transaction with a clear record." },
    "refund.claim.create": { label: "Request a refund", description: "Prepare a refund request for review." },
    "refund.claim.cancel": { label: "Cancel a refund request", description: "Stop a refund request with an approval record." },
    "refund.payment.release": { label: "Release a refund payment", description: "Release an approved refund after all checks pass." },
    "evidence.replay": { label: "Review exceptions", description: "Investigate and recover records that need attention." },
    "access.control": { label: "View access rules", description: "See who can act and what requires approval." },
  },
};

const GERMAN: AccessControlMessages = {
  ...ENGLISH,
  ariaLabel: "Arbeitsbereich für Zugriff und Genehmigungen",
  overview: { ...ENGLISH.overview, eyebrow: "Zugriff & Genehmigungen", title: "Wer darf was?", status: { signInNeeded: "Anmeldung erforderlich", unavailable: "Zugriffsprüfung nicht verfügbar", checked: "Zugriff wird bei der Aktion geprüft", checking: "Zugriff wird geprüft" }, copy: "Ihr Zugriff wird geprüft, sobald Sie eine geschützte Aktion öffnen. Verantwortlichkeiten können unterschiedlich sein, manche Aktionen benötigen eine zweite Genehmigung, und die Entscheidung kann vom Konto, der Umgebung und dem Datensatz abhängen.", person: "Angemeldete Person", notSignedIn: "Nicht angemeldet", account: "Konto", accountConfirmed: "Für dieses Konto bestätigt", accountNeedsSignIn: "Bei der Anmeldung bestätigt", permission: "Berechtigungsstatus", permissionUnavailable: "Nicht verfügbar; Aktionen bleiben gesperrt", permissionChecked: "Vom Dienst geprüft", permissionWaiting: "Warten auf den Dienst", permissionSignIn: "Warten auf die Anmeldung", dimensions: { roles: "Rollen", rolesCopy: "Ihre Verantwortlichkeiten bestimmen, welche Arbeit Sie übernehmen oder genehmigen dürfen.", permissions: "Berechtigungen", permissionsCopy: "Jede Aktion hat eine bestimmte Berechtigung statt eines pauschalen Zugriffs.", conditions: "Bedingungen", conditionsCopy: "Konto, Umgebung, Datensatzstatus und Risiko können die Entscheidung ändern." }, technicalSummary: "So wird Zugriff entschieden", technicalCopy: "RBAC prüft Verantwortlichkeiten, PBAC benannte Berechtigungen und ABAC Konto- und Datensatzbedingungen. Das sind technische Namen für die drei verständlich beschriebenen Prüfungen oben.", safeTitle: "Sicher voreingestellt.", safeUnavailable: "Die Zugriffsprüfung ist nicht verfügbar; geschützte Aktionen bleiben daher nicht verfügbar.", safeDefault: "Wenn der Dienst den Zugriff nicht bestätigen kann, bleibt die Aktion nicht verfügbar.", noBrowserAuthority: "Im Browser wird keine Genehmigung, Zahlung oder Aufzeichnung erstellt." },
  rules: { ...ENGLISH.rules, eyebrow: "Zugriffsregeln", title: "Aktionen und Genehmigungen", count: (count) => `${count} Regeln`, copy: "Diese Regeln zeigen, welche Arbeit verfügbar ist und wann eine weitere Person oder ein verifizierter Datensatz erforderlich ist.", technicalSummary: "Technische Details für Administratoren", none: "Keine" },
  groups: { "Daily work": "Tagesarbeit", "Rules & access": "Regeln & Zugriff", "Money movement": "Geldbewegungen", "Records & issues": "Aufzeichnungen & Probleme" },
  decisions: { available: "Verfügbar", unavailable: "Nicht verfügbar", needsContext: "Kontext erforderlich", checking: "Wird geprüft", signIn: "Zur Prüfung anmelden" }, risks: { financial: "Sensibel", governance: "Genehmigung", write: "Änderung", read: "Ansicht" },
  policies: { "tax.quote.create": { label: "Steuerangebot erstellen", description: "Ein Kundenangebot mit klarer Steueraufteilung vorbereiten." }, "tax.document.quote.create": { label: "Rechnungssteuer berechnen", description: "Steuern für eine Rechnung und ihre Positionen berechnen." }, "tax.preview.run": { label: "Szenario erkunden", description: "Ein Szenario prüfen, ohne veröffentlichte Regeln oder Zahlungen zu ändern." }, "tax.rule.govern": { label: "Steuerregeln verwalten", description: "Steuerregeln erstellen, prüfen, genehmigen und veröffentlichen." }, "settlement.wallet": { label: "Zahlung prüfen", description: "Zahlungsdetails prüfen, bevor der Kunde signiert." }, "settlement.document.wallet": { label: "Rechnungszahlung prüfen", description: "Die Zahlung für eine Rechnung prüfen und genehmigen." }, "vault.reconciliation": { label: "Saldenprüfungen prüfen", description: "Erfasste Salden mit der letzten verifizierten Beobachtung vergleichen." }, "vault.release": { label: "Treasury-Mittel freigeben", description: "Eine kontrollierte Treasury-Freigabe zur Genehmigung vorbereiten." }, "revenue_stream.read": { label: "Partnerauszahlungspläne anzeigen", description: "Gesteuerte Partnerauszahlungspläne und verifizierte Salden prüfen." }, "revenue_stream.propose": { label: "Partnerauszahlungsplan erstellen", description: "Einen begrenzten Auszahlungsplan für einen Partner vorschlagen." }, "revenue_stream.govern": { label: "Partnerauszahlungsplan genehmigen", description: "Einen Partnerauszahlungsplan prüfen und steuern." }, "revenue_stream.claim.prepare": { label: "Partnerauszahlung vorbereiten", description: "Einen verfügbaren Betrag für eine verifizierte Freigabe reservieren." }, "settlement.adjustment": { label: "Korrektur vorbereiten", description: "Eine abgeschlossene Transaktion mit einem klaren Nachweis korrigieren." }, "refund.claim.create": { label: "Rückerstattung anfordern", description: "Eine Rückerstattungsanfrage zur Prüfung vorbereiten." }, "refund.claim.cancel": { label: "Rückerstattungsanfrage stornieren", description: "Eine Rückerstattungsanfrage mit Genehmigungsnachweis stoppen." }, "refund.payment.release": { label: "Rückerstattung freigeben", description: "Eine genehmigte Rückerstattung nach allen Prüfungen freigeben." }, "evidence.replay": { label: "Ausnahmen prüfen", description: "Aufmerksamkeit erfordernde Aufzeichnungen untersuchen und wiederherstellen." }, "access.control": { label: "Zugriffsregeln anzeigen", description: "Sehen, wer handeln darf und was eine Genehmigung benötigt." } },
};

const ARABIC: AccessControlMessages = {
  ...ENGLISH,
  ariaLabel: "مساحة الوصول والموافقات",
  overview: { ...ENGLISH.overview, eyebrow: "الوصول والموافقات", title: "من يستطيع تنفيذ ماذا؟", status: { signInNeeded: "يلزم تسجيل الدخول", unavailable: "فحص الوصول غير متاح", checked: "يُفحص الوصول عند تنفيذ الإجراء", checking: "جارٍ فحص الوصول" }, copy: "يُفحص وصولك كلما فتحت إجراءً محميًا. قد تختلف المسؤوليات، وقد تحتاج بعض الإجراءات إلى موافقة ثانية، وقد يعتمد القرار على الحساب والبيئة والسجل.", person: "الشخص المسجل دخوله", notSignedIn: "لم يتم تسجيل الدخول", account: "الحساب", accountConfirmed: "تم التأكيد لهذا الحساب", accountNeedsSignIn: "يُؤكد عند تسجيل الدخول", permission: "حالة الإذن", permissionUnavailable: "غير متاح؛ ستبقى الإجراءات مغلقة", permissionChecked: "تم التحقق بواسطة الخدمة", permissionWaiting: "بانتظار الخدمة", permissionSignIn: "بانتظار تسجيل الدخول", dimensions: { roles: "الأدوار", rolesCopy: "تحدد مسؤولياتك العمل الذي يمكنك توليه أو الموافقة عليه.", permissions: "الأذونات", permissionsCopy: "لكل إجراء إذن محدد بدلًا من وصول واسع.", conditions: "الشروط", conditionsCopy: "قد يغير الحساب والبيئة وحالة السجل والمخاطر القرار." }, technicalSummary: "كيف يُقرر الوصول", technicalCopy: "يفحص RBAC المسؤوليات، ويفحص PBAC الأذونات المسماة، ويفحص ABAC شروط الحساب والسجل. هذه أسماء تقنية للفحوصات المبسطة أعلاه.", safeTitle: "آمن افتراضيًا.", safeUnavailable: "فحص الوصول غير متاح، لذلك تبقى الإجراءات المحمية غير متاحة.", safeDefault: "إذا تعذر على الخدمة تأكيد الوصول، تبقى الإجراءات غير متاحة.", noBrowserAuthority: "لا ينشئ المتصفح موافقة أو دفعة أو سجلًا." },
  rules: { ...ENGLISH.rules, eyebrow: "قواعد الوصول", title: "الإجراءات والموافقات", count: (count) => `${count} قواعد`, copy: "توضح هذه القواعد العمل المتاح ومتى يلزم شخص آخر أو سجل تم التحقق منه.", technicalSummary: "تفاصيل تقنية للمسؤولين", none: "لا يوجد" },
  groups: { "Daily work": "العمل اليومي", "Rules & access": "القواعد والوصول", "Money movement": "حركة الأموال", "Records & issues": "السجلات والمشكلات" },
  decisions: { available: "متاح", unavailable: "غير متاح", needsContext: "يلزم السياق", checking: "جارٍ التحقق", signIn: "سجل الدخول للتحقق" }, risks: { financial: "حساس", governance: "موافقة", write: "تغيير", read: "عرض" },
  policies: {
    "tax.quote.create": { label: "إنشاء عرض ضريبي", description: "إعداد عرض للعميل مع تفصيل واضح للضريبة." },
    "tax.document.quote.create": { label: "حساب ضريبة الفاتورة", description: "حساب الضريبة على الفاتورة وبنودها." },
    "tax.preview.run": { label: "استكشاف سيناريو", description: "تجربة سيناريو دون تغيير القواعد المنشورة أو المدفوعات." },
    "tax.rule.govern": { label: "إدارة القواعد الضريبية", description: "إنشاء القواعد الضريبية ومراجعتها واعتمادها ونشرها." },
    "settlement.wallet": { label: "مراجعة الدفعة", description: "فحص تفاصيل الدفع قبل توقيع العميل." },
    "settlement.document.wallet": { label: "مراجعة دفعة المستند", description: "مراجعة دفعة فاتورة واعتمادها." },
    "vault.reconciliation": { label: "مراجعة فحوصات الرصيد", description: "مقارنة الأرصدة المسجلة بأحدث ملاحظة تم التحقق منها." },
    "vault.release": { label: "تحرير أموال الخزينة", description: "إعداد تحرير مضبوط لأموال الخزينة من أجل الاعتماد." },
    "revenue_stream.read": { label: "عرض خطط مدفوعات الشركاء", description: "مراجعة خطط مدفوعات الشركاء الخاضعة للحوكمة وأرصدتها الموثقة." },
    "revenue_stream.propose": { label: "إنشاء خطة دفع للشريك", description: "اقتراح خطة دفع محدودة لشريك." },
    "revenue_stream.govern": { label: "اعتماد خطة دفع للشريك", description: "مراجعة خطة دفع للشريك وإدارتها." },
    "revenue_stream.claim.prepare": { label: "إعداد تحرير دفعة الشريك", description: "حجز مبلغ متاح لتحرير موثق." },
    "settlement.adjustment": { label: "إعداد تصحيح", description: "تصحيح معاملة مكتملة مع سجل واضح." },
    "refund.claim.create": { label: "طلب استرداد", description: "إعداد طلب استرداد للمراجعة." },
    "refund.claim.cancel": { label: "إلغاء طلب استرداد", description: "إيقاف طلب استرداد مع سجل موافقة." },
    "refund.payment.release": { label: "تحرير دفعة استرداد", description: "تحرير استرداد معتمد بعد اجتياز جميع الفحوصات." },
    "evidence.replay": { label: "مراجعة الاستثناءات", description: "التحقيق في السجلات التي تحتاج إلى انتباه واستعادتها." },
    "access.control": { label: "عرض قواعد الوصول", description: "معرفة من يستطيع التنفيذ وما يتطلب موافقة." },
  },
};

const JAPANESE: AccessControlMessages = {
  ...ENGLISH,
  ariaLabel: "アクセスと承認のワークスペース",
  overview: { ...ENGLISH.overview, eyebrow: "アクセスと承認", title: "誰が何を実行できるか", status: { signInNeeded: "サインインが必要です", unavailable: "アクセス確認を利用できません", checked: "操作時にアクセスを確認します", checking: "アクセスを確認中" }, copy: "保護された操作を開くたびにアクセスを確認します。担当範囲や二重承認の要否は異なり、アカウント、環境、レコードによって判断が変わる場合があります。", person: "サインイン中の担当者", notSignedIn: "サインインしていません", account: "アカウント", accountConfirmed: "このアカウントで確認済み", accountNeedsSignIn: "サインイン時に確認", permission: "権限の状態", permissionUnavailable: "利用不可。操作は閉じたままです", permissionChecked: "サービスで確認済み", permissionWaiting: "サービスを待機中", permissionSignIn: "サインインを待機中", dimensions: { roles: "役割", rolesCopy: "担当範囲によって担当または承認できる作業が決まります。", permissions: "権限", permissionsCopy: "各操作には広範なアクセスではなく固有の権限があります。", conditions: "条件", conditionsCopy: "アカウント、環境、レコード状態、リスクによって判断が変わる場合があります。" }, technicalSummary: "アクセスの判断方法", technicalCopy: "RBACは担当範囲、PBACは名前付き権限、ABACはアカウントとレコードの条件を確認します。上記の3つの平易な確認を表す技術用語です。", safeTitle: "安全側が初期状態です。", safeUnavailable: "アクセス確認を利用できないため、保護された操作は利用できません。", safeDefault: "サービスがアクセスを確認できない場合、操作は利用できません。", noBrowserAuthority: "ブラウザーで承認、支払い、レコードは作成されません。" },
  rules: { ...ENGLISH.rules, eyebrow: "アクセスルール", title: "操作と承認", count: (count) => `${count} 件のルール`, copy: "利用できる作業と、別の担当者または確認済みレコードが必要な場所を説明します。", technicalSummary: "管理者向け技術詳細", none: "なし" },
  groups: { "Daily work": "日常業務", "Rules & access": "ルールとアクセス", "Money movement": "資金移動", "Records & issues": "記録と問題" },
  decisions: { available: "利用可能", unavailable: "利用不可", needsContext: "コンテキストが必要", checking: "確認中", signIn: "サインインして確認" }, risks: { financial: "重要", governance: "承認", write: "変更", read: "表示" },
  policies: {
    "tax.quote.create": { label: "税額見積を作成", description: "明確な税の内訳を含む顧客見積を準備します。" },
    "tax.document.quote.create": { label: "請求書の税額を計算", description: "請求書と各明細の税額を計算します。" },
    "tax.preview.run": { label: "シナリオを確認", description: "公開済みルールや支払いを変更せずにシナリオを試します。" },
    "tax.rule.govern": { label: "税ルールを管理", description: "税ルールを作成、確認、承認、公開します。" },
    "settlement.wallet": { label: "支払いを確認", description: "顧客が署名する前に支払いの詳細を確認します。" },
    "settlement.document.wallet": { label: "書類支払いを確認", description: "請求書の支払いを確認して承認します。" },
    "vault.reconciliation": { label: "残高確認を確認", description: "記録された残高を最新の検証済み観測値と比較します。" },
    "vault.release": { label: "Treasury資金をリリース", description: "承認用の管理されたTreasuryリリースを準備します。" },
    "revenue_stream.read": { label: "パートナー支払いプランを表示", description: "管理されたパートナー支払いプランと検証済み残高を確認します。" },
    "revenue_stream.propose": { label: "パートナー支払いプランを作成", description: "パートナー向けの上限付き支払いプランを提案します。" },
    "revenue_stream.govern": { label: "パートナー支払いプランを承認", description: "パートナー支払いプランを確認して管理します。" },
    "revenue_stream.claim.prepare": { label: "パートナー支払いリリースを準備", description: "検証済みリリースのために利用可能額を予約します。" },
    "settlement.adjustment": { label: "調整を準備", description: "明確な記録を残して完了済み取引を修正します。" },
    "refund.claim.create": { label: "返金を申請", description: "確認用の返金申請を準備します。" },
    "refund.claim.cancel": { label: "返金申請をキャンセル", description: "承認記録を残して返金申請を停止します。" },
    "refund.payment.release": { label: "返金支払いをリリース", description: "すべての確認が完了した承認済み返金をリリースします。" },
    "evidence.replay": { label: "例外を確認", description: "対応が必要な記録を調査して復旧します。" },
    "access.control": { label: "アクセスルールを表示", description: "誰が操作でき、何に承認が必要かを確認します。" },
  },
};

const CATALOGS: Record<string, AccessControlMessages> = { "en-US": ENGLISH, "de-DE": GERMAN, "ar-EG": ARABIC, "ja-JP": JAPANESE };

export function accessControlMessagesFor(locale: string): AccessControlMessages {
  return CATALOGS[locale] ?? CATALOGS[DEFAULT_LOCALE] ?? ENGLISH;
}
