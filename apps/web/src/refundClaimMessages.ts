import { SupportedLocale } from "./formatters";

export interface RefundClaimMessages {
  proposal: {
    ariaLabel: string;
    eyebrow: string;
    title: string;
    badge: string;
    description: string;
    correctionReference: string;
    correctionPlaceholder: string;
    requestReference: string;
    requestHelp: string;
    preparing: string;
    prepare: string;
    refundRequestReference: string;
    savedRequestPlaceholder: string;
    load: string;
    serviceUnavailable: string;
  };
  review: {
    ariaLabel: string;
    eyebrow: string;
    title: string;
    noRequest: string;
    noRequestCopy: string;
    grossRefund: string;
    taxIncluded: string;
    refundAfterTax: string;
    asset: string;
    destination: string;
    onChainRecord: string;
    refundAccount: string;
    expiry: string;
    submissionReference: string;
    noneRecorded: string;
    finalizedTitle: string;
    finalizedCopy: string;
    finalizedProof: string;
    event: string;
    finalizedSlot: string;
    eventIndex: string;
    eventSignature: string;
    grossTaxRefund: string;
    evidenceDigest: string;
  };
  wallet: {
    twoPersonNote: string;
    signProposer: string;
    signFunder: string;
    recovery: string;
    recoveryPlaceholder: string;
    recoveryHelp: string;
    sending: string;
    send: string;
  };
  evidence: {
    reconcileWarning: string;
    reconcile: string;
    lastError: string;
  };
  errors: RefundClaimErrors;
}

export interface RefundClaimErrors {
  authorizationRequired: string;
  correctionReferenceRequired: string;
  preparationRequestFailed: (status: number) => string;
  preparationFailed: string;
  preparationReady: string;
  requestReferenceRequired: string;
  recoveryRequestFailed: (status: number) => string;
  recoveryFailed: string;
  recoveryLoaded: string;
  prepareBeforeSigning: string;
  signableStateRequired: string;
  funderPrerequisite: string;
  signingFailed: string;
  bothSignaturesPresent: string;
  proposerSignaturePresent: string;
  funderSignaturePresent: string;
  submissionRequestFailed: (status: number) => string;
  submissionFailed: string;
  prepareBeforeSubmission: string;
  signaturesRequired: string;
  submissionUnknown: string;
  submissionPending: string;
  prepareBeforeReconciliation: string;
  reconciliationRequestFailed: (status: number) => string;
  reconciliationFailed: string;
  reconciliationReady: string;
}

function displayHttpStatus(status: number): string {
  return Number.isInteger(status) && status >= 100 && status <= 599 ? String(status) : "unknown status";
}

const ENGLISH_ERRORS: RefundClaimErrors = {
  authorizationRequired: "Sign in with an account allowed to prepare refund requests. The credential is held in memory only.",
  correctionReferenceRequired: "An approved correction reference is required; financial values are loaded by the service.",
  preparationRequestFailed: (status) => `Refund request preparation failed (${displayHttpStatus(status)}).`,
  preparationFailed: "Refund request preparation failed.",
  preparationReady: "Your refund request is ready for review. Check the amounts, recipient, approval roles, and expiry before signing. No funds moved.",
  requestReferenceRequired: "A refund request reference is required.",
  recoveryRequestFailed: (status) => `Refund request recovery failed (${displayHttpStatus(status)}).`,
  recoveryFailed: "Refund request recovery failed.",
  recoveryLoaded: "The saved refund request was loaded. Signed wallet data is not returned; paste the unchanged wallet artifact to continue.",
  prepareBeforeSigning: "Prepare or recover a refund request before signing.",
  signableStateRequired: "This request is not in a wallet-signable state. Reconcile the durable state before taking another action.",
  funderPrerequisite: "The proposer signature is required before the refund-funder wallet can sign.",
  signingFailed: "Refund request wallet signing failed.",
  bothSignaturesPresent: "Both wallet signatures are present in the local transaction artifact. Submit these unchanged bytes; a signature is not a refund confirmation.",
  proposerSignaturePresent: "Proposer signature collected. Switch to the configured refund-funder wallet and collect its signature before submission.",
  funderSignaturePresent: "Wallet signature collected. The API still must verify and submit the complete transaction.",
  submissionRequestFailed: (status) => `Refund request submission failed (${displayHttpStatus(status)}).`,
  submissionFailed: "Refund request submission failed.",
  prepareBeforeSubmission: "Prepare or recover a refund request before submission.",
  signaturesRequired: "Both required wallet signatures must be present before submission.",
  submissionUnknown: "The provider outcome is unknown. Do not sign or submit different bytes; reconcile the durable claim against finalized chain evidence.",
  submissionPending: "The provider accepted the unchanged transaction bytes. This is transport evidence only; the claim is not complete until finalized event and balance proof are recorded.",
  prepareBeforeReconciliation: "Prepare or recover a refund request before reconciliation.",
  reconciliationRequestFailed: (status) => `Refund claim reconciliation failed (${displayHttpStatus(status)}).`,
  reconciliationFailed: "Refund claim reconciliation failed.",
  reconciliationReady: "Finalized claim event evidence matched the durable artifact. Execute preparation is now eligible for the separately authorized approver workflow.",
};

const GERMAN_ERRORS: RefundClaimErrors = {
  authorizationRequired: "Melden Sie sich mit einem Konto an, das Rückerstattungsanträge vorbereiten darf. Das Zugriffstoken bleibt nur im Speicher.",
  correctionReferenceRequired: "Eine genehmigte Korrekturreferenz ist erforderlich; Finanzwerte lädt der Dienst.",
  preparationRequestFailed: (status) => `Die Vorbereitung des Rückerstattungsantrags ist fehlgeschlagen (${displayHttpStatus(status)}).`,
  preparationFailed: "Die Vorbereitung des Rückerstattungsantrags ist fehlgeschlagen.",
  preparationReady: "Ihr Rückerstattungsantrag ist zur Prüfung bereit. Prüfen Sie Beträge, Empfänger, Rollen und Ablauf vor dem Signieren. Es wurden keine Mittel bewegt.",
  requestReferenceRequired: "Eine Referenz des Rückerstattungsantrags ist erforderlich.",
  recoveryRequestFailed: (status) => `Die Wiederherstellung des Rückerstattungsantrags ist fehlgeschlagen (${displayHttpStatus(status)}).`,
  recoveryFailed: "Die Wiederherstellung des Rückerstattungsantrags ist fehlgeschlagen.",
  recoveryLoaded: "Der gespeicherte Rückerstattungsantrag wurde geladen. Signierte Wallet-Daten werden nicht zurückgegeben; fügen Sie das unveränderte Artefakt ein.",
  prepareBeforeSigning: "Bereiten Sie einen Rückerstattungsantrag vor oder stellen Sie ihn wieder her, bevor Sie signieren.",
  signableStateRequired: "Dieser Antrag ist nicht in einem signierbaren Wallet-Status. Stimmen Sie den dauerhaften Status vor dem nächsten Schritt ab.",
  funderPrerequisite: "Die Antragstellersignatur ist erforderlich, bevor das Rückerstattungs-Wallet signieren kann.",
  signingFailed: "Die Wallet-Signatur des Rückerstattungsantrags ist fehlgeschlagen.",
  bothSignaturesPresent: "Beide Wallet-Signaturen sind im Transaktionsartefakt vorhanden. Senden Sie unveränderte Bytes; eine Signatur bestätigt keine Rückerstattung.",
  proposerSignaturePresent: "Antragstellersignatur erfasst. Wechseln Sie zum konfigurierten Rückerstattungs-Wallet und erfassen Sie dessen Signatur.",
  funderSignaturePresent: "Wallet-Signatur erfasst. Der Dienst muss die vollständige Transaktion weiterhin prüfen und senden.",
  submissionRequestFailed: (status) => `Der Rückerstattungsantrag konnte nicht gesendet werden (${displayHttpStatus(status)}).`,
  submissionFailed: "Der Rückerstattungsantrag konnte nicht gesendet werden.",
  prepareBeforeSubmission: "Bereiten Sie einen Rückerstattungsantrag vor oder stellen Sie ihn wieder her, bevor Sie ihn senden.",
  signaturesRequired: "Vor dem Senden müssen beide erforderlichen Wallet-Signaturen vorhanden sein.",
  submissionUnknown: "Das Providerergebnis ist unbekannt. Signieren oder senden Sie keine anderen Bytes; gleichen Sie den dauerhaften Antrag mit endgültigen Chain-Nachweisen ab.",
  submissionPending: "Der Provider hat die unveränderten Transaktionsbytes angenommen. Dies ist nur Transportnachweis; der Antrag ist erst mit endgültigem Ereignis- und Saldennachweis abgeschlossen.",
  prepareBeforeReconciliation: "Bereiten Sie einen Rückerstattungsantrag vor oder stellen Sie ihn wieder her, bevor Sie ihn abstimmen.",
  reconciliationRequestFailed: (status) => `Der Rückerstattungsnachweis konnte nicht abgeglichen werden (${displayHttpStatus(status)}).`,
  reconciliationFailed: "Der Rückerstattungsnachweis konnte nicht abgeglichen werden.",
  reconciliationReady: "Der endgültige Ereignisnachweis stimmt mit dem dauerhaften Artefakt überein. Die Ausführung kann nun im separat autorisierten Genehmiger-Workflow vorbereitet werden.",
};

const ARABIC_ERRORS: RefundClaimErrors = {
  authorizationRequired: "سجّل الدخول بحساب مسموح له بإعداد طلبات الاسترداد. يبقى الاعتماد في الذاكرة فقط.",
  correctionReferenceRequired: "مرجع تصحيح معتمد مطلوب؛ تحمّل الخدمة القيم المالية.",
  preparationRequestFailed: (status) => `تعذر إعداد طلب الاسترداد (${displayHttpStatus(status)}).`,
  preparationFailed: "تعذر إعداد طلب الاسترداد.",
  preparationReady: "طلب الاسترداد جاهز للمراجعة. تحقق من المبالغ والمستلم والأدوار والانتهاء قبل التوقيع. لم تتحرك أي أموال.",
  requestReferenceRequired: "مرجع طلب الاسترداد مطلوب.",
  recoveryRequestFailed: (status) => `تعذر استرداد طلب الاسترداد (${displayHttpStatus(status)}).`,
  recoveryFailed: "تعذر استرداد طلب الاسترداد.",
  recoveryLoaded: "تم تحميل طلب الاسترداد المحفوظ. لا تعيد الخدمة بيانات المحفظة الموقعة؛ ألصق أثر المحفظة غير المتغير للمتابعة.",
  prepareBeforeSigning: "أعد طلب الاسترداد أو استرجعه قبل التوقيع.",
  signableStateRequired: "هذا الطلب ليس في حالة تسمح بتوقيع المحفظة. طابق الحالة الدائمة قبل اتخاذ إجراء آخر.",
  funderPrerequisite: "توقيع مقدم الطلب مطلوب قبل أن توقع محفظة ممول الاسترداد.",
  signingFailed: "فشل توقيع محفظة طلب الاسترداد.",
  bothSignaturesPresent: "توقيعا المحفظة موجودان في أثر المعاملة المحلي. أرسل وحدات البايت دون تغيير؛ التوقيع ليس تأكيداً للاسترداد.",
  proposerSignaturePresent: "تم جمع توقيع مقدم الطلب. بدّل إلى محفظة ممول الاسترداد المكوّنة واجمع توقيعها قبل الإرسال.",
  funderSignaturePresent: "تم جمع توقيع المحفظة. لا تزال الخدمة مطالبة بالتحقق من المعاملة الكاملة وإرسالها.",
  submissionRequestFailed: (status) => `تعذر إرسال طلب الاسترداد (${displayHttpStatus(status)}).`,
  submissionFailed: "تعذر إرسال طلب الاسترداد.",
  prepareBeforeSubmission: "أعد طلب الاسترداد أو استرجعه قبل الإرسال.",
  signaturesRequired: "يجب وجود توقيعي المحفظة المطلوبين قبل الإرسال.",
  submissionUnknown: "نتيجة المزوّد غير معروفة. لا توقّع أو ترسل وحدات بايت مختلفة؛ طابق المطالبة الدائمة مع دليل الشبكة النهائي.",
  submissionPending: "قبل المزوّد وحدات المعاملة دون تغيير. هذا دليل نقل فقط؛ لا يكتمل الطلب حتى تسجيل دليل الحدث والرصيد النهائي.",
  prepareBeforeReconciliation: "أعد طلب الاسترداد أو استرجعه قبل المطابقة.",
  reconciliationRequestFailed: (status) => `تعذر مطابقة دليل طلب الاسترداد (${displayHttpStatus(status)}).`,
  reconciliationFailed: "تعذر مطابقة دليل طلب الاسترداد.",
  reconciliationReady: "تطابق دليل حدث الطلب النهائي مع الأثر الدائم. أصبح إعداد التنفيذ متاحاً لمسار الموافق المصرح به بشكل منفصل.",
};

const JAPANESE_ERRORS: RefundClaimErrors = {
  authorizationRequired: "返金申請を準備できるアカウントでサインインしてください。認証情報はメモリ内だけに保持されます。",
  correctionReferenceRequired: "承認済み訂正参照が必要です。金額はサービスが読み込みます。",
  preparationRequestFailed: (status) => `返金申請の準備に失敗しました（${displayHttpStatus(status)}）。`,
  preparationFailed: "返金申請の準備に失敗しました。",
  preparationReady: "返金申請の確認準備ができました。署名前に金額、受取先、承認者、期限を確認してください。資金は移動していません。",
  requestReferenceRequired: "返金申請参照が必要です。",
  recoveryRequestFailed: (status) => `返金申請を復元できませんでした（${displayHttpStatus(status)}）。`,
  recoveryFailed: "返金申請を復元できませんでした。",
  recoveryLoaded: "保存済み返金申請を読み込みました。署名済みウォレットデータは返されないため、変更していないアーティファクトを貼り付けてください。",
  prepareBeforeSigning: "署名前に返金申請を準備または復元してください。",
  signableStateRequired: "この申請はウォレット署名可能な状態ではありません。別の操作の前に永続状態を照合してください。",
  funderPrerequisite: "返金資金提供者のウォレットが署名する前に申請者の署名が必要です。",
  signingFailed: "返金申請ウォレットの署名に失敗しました。",
  bothSignaturesPresent: "2つのウォレット署名がローカルのトランザクションアーティファクトにあります。変更していないバイトを送信してください。署名は返金完了の証明ではありません。",
  proposerSignaturePresent: "申請者の署名を取得しました。設定済みの返金資金提供者ウォレットに切り替えて送信前に署名してください。",
  funderSignaturePresent: "ウォレット署名を取得しました。サービスは完全なトランザクションを検証して送信する必要があります。",
  submissionRequestFailed: (status) => `返金申請を送信できませんでした（${displayHttpStatus(status)}）。`,
  submissionFailed: "返金申請を送信できませんでした。",
  prepareBeforeSubmission: "送信前に返金申請を準備または復元してください。",
  signaturesRequired: "送信前に必要な2つのウォレット署名を用意してください。",
  submissionUnknown: "プロバイダーの結果が不明です。別のバイトを署名または送信せず、永続申請を確定済みチェーン証拠と照合してください。",
  submissionPending: "変更していないトランザクションバイトがプロバイダーに受理されました。これは送信証拠だけであり、確定イベントと残高証拠まで申請は完了しません。",
  prepareBeforeReconciliation: "照合前に返金申請を準備または復元してください。",
  reconciliationRequestFailed: (status) => `返金申請の証拠を照合できませんでした（${displayHttpStatus(status)}）。`,
  reconciliationFailed: "返金申請の証拠を照合できませんでした。",
  reconciliationReady: "確定済み申請イベント証拠が永続アーティファクトと一致しました。別途承認された実行ワークフローで準備できます。",
};

const ENGLISH: RefundClaimMessages = {
  proposal: {
    ariaLabel: "Refund claim wallet workspace",
    eyebrow: "Refund request / two-person approval",
    title: "Prepare a refund request",
    badge: "You keep custody",
    description: "The service loads the approved payment details and prepares the refund for review. You can review the result, but cannot change its financial or recipient details in the browser.",
    correctionReference: "Approved correction reference",
    correctionPlaceholder: "Paste the approved correction reference",
    requestReference: "Request reference",
    requestHelp: "A unique reference prevents duplicate requests when you retry.",
    preparing: "Preparing…",
    prepare: "Prepare refund request",
    refundRequestReference: "Refund request reference",
    savedRequestPlaceholder: "Paste a saved request reference",
    load: "Load saved request",
    serviceUnavailable: "The service is unavailable. No client-side fallback or local settlement state is used.",
  },
  review: {
    ariaLabel: "Immutable refund artifact review",
    eyebrow: "Immutable artifact review",
    title: "Wallet authorization",
    noRequest: "No refund request loaded",
    noRequestCopy: "Prepare or recover a request to review its payment details and approval status.",
    grossRefund: "Gross refund",
    taxIncluded: "Tax included",
    refundAfterTax: "Refund after tax",
    asset: "Asset",
    destination: "Destination",
    onChainRecord: "On-chain refund record",
    refundAccount: "Refund account",
    expiry: "Expiry",
    submissionReference: "Submission reference",
    noneRecorded: "None recorded",
    finalizedTitle: "Finalized claim event evidence matched the durable artifact.",
    finalizedCopy: "The event proof is bound to the recorded signature, finalized slot, claim PDA, amounts, and immutable evidence digest. Finality is movement evidence, not a legal or compliance determination.",
    finalizedProof: "Finalized event proof",
    event: "Event",
    finalizedSlot: "Finalized slot",
    eventIndex: "Event index",
    eventSignature: "Event signature",
    grossTaxRefund: "Gross / tax / refund",
    evidenceDigest: "Evidence digest",
  },
  wallet: {
    twoPersonNote: "Two different people must approve this refund: the request owner first, then the configured refund account owner. The service checks that the approved payment details were not changed.",
    signProposer: "Sign as proposer",
    signFunder: "Sign as refund funder",
    recovery: "Recovery data (advanced)",
    recoveryPlaceholder: "Paste the unchanged wallet file after a browser reload.",
    recoveryHelp: "This temporary data stays in memory and is sent unchanged for verification.",
    sending: "Sending…",
    send: "Send approved refund",
  },
  evidence: {
    reconcileWarning: "This state requires durable chain reconciliation. A provider response or wallet signature is not proof of a completed refund.",
    reconcile: "Reconcile finalized claim evidence",
    lastError: "Last durable error:",
  },
  errors: ENGLISH_ERRORS,
};

const GERMAN: RefundClaimMessages = {
  proposal: {
    ariaLabel: "Wallet-Arbeitsbereich für Rückerstattungsanträge",
    eyebrow: "Rückerstattungsantrag / Vier-Augen-Freigabe",
    title: "Rückerstattungsantrag vorbereiten",
    badge: "Sie behalten die Verwahrung",
    description: "Der Dienst lädt die genehmigten Zahlungsdetails und bereitet die Rückerstattung zur Prüfung vor. Sie können das Ergebnis prüfen, aber finanzielle oder Empfängerdaten im Browser nicht ändern.",
    correctionReference: "Genehmigte Korrekturreferenz",
    correctionPlaceholder: "Genehmigte Korrekturreferenz einfügen",
    requestReference: "Antragsreferenz",
    requestHelp: "Eine eindeutige Referenz verhindert doppelte Anträge bei Wiederholungen.",
    preparing: "Wird vorbereitet…",
    prepare: "Rückerstattungsantrag vorbereiten",
    refundRequestReference: "Referenz des Rückerstattungsantrags",
    savedRequestPlaceholder: "Gespeicherte Antragsreferenz einfügen",
    load: "Gespeicherten Antrag laden",
    serviceUnavailable: "Der Dienst ist nicht verfügbar. Es wird kein clientseitiger Ersatz oder lokaler Abwicklungsstatus verwendet.",
  },
  review: {
    ariaLabel: "Prüfung des unveränderlichen Rückerstattungsartefakts",
    eyebrow: "Prüfung des unveränderlichen Artefakts",
    title: "Wallet-Autorisierung",
    noRequest: "Kein Rückerstattungsantrag geladen",
    noRequestCopy: "Bereiten Sie einen Antrag vor oder laden Sie ihn, um Zahlungsdetails und Genehmigungsstatus zu prüfen.",
    grossRefund: "Bruttorückerstattung",
    taxIncluded: "Enthaltene Steuer",
    refundAfterTax: "Rückerstattung nach Steuer",
    asset: "Asset",
    destination: "Ziel",
    onChainRecord: "Rückerstattungsdatensatz auf der Chain",
    refundAccount: "Rückerstattungskonto",
    expiry: "Ablauf",
    submissionReference: "Übermittlungsreferenz",
    noneRecorded: "Keine erfasst",
    finalizedTitle: "Der endgültige Nachweis des Rückerstattungsereignisses stimmt mit dem dauerhaften Artefakt überein.",
    finalizedCopy: "Der Ereignisnachweis ist an die aufgezeichnete Signatur, den endgültigen Slot, die Claim-PDA, Beträge und den unveränderlichen Nachweis-Digest gebunden. Finalität ist ein Bewegungsnachweis, keine rechtliche oder Compliance-Feststellung.",
    finalizedProof: "Nachweis des endgültigen Ereignisses",
    event: "Ereignis",
    finalizedSlot: "Endgültiger Slot",
    eventIndex: "Ereignisindex",
    eventSignature: "Ereignissignatur",
    grossTaxRefund: "Brutto / Steuer / Rückerstattung",
    evidenceDigest: "Nachweis-Digest",
  },
  wallet: {
    twoPersonNote: "Zwei verschiedene Personen müssen diese Rückerstattung genehmigen: zuerst der Antragsteller, danach der Inhaber des konfigurierten Rückerstattungskontos. Der Dienst prüft, dass die genehmigten Zahlungsdetails unverändert sind.",
    signProposer: "Als Antragsteller signieren",
    signFunder: "Als Rückerstattungsfinanzierer signieren",
    recovery: "Wiederherstellungsdaten (erweitert)",
    recoveryPlaceholder: "Unveränderte Wallet-Datei nach einem Browser-Neuladen einfügen.",
    recoveryHelp: "Diese temporären Daten bleiben im Speicher und werden zur Prüfung unverändert übertragen.",
    sending: "Wird gesendet…",
    send: "Genehmigte Rückerstattung senden",
  },
  evidence: {
    reconcileWarning: "Dieser Zustand erfordert einen dauerhaften Chain-Abgleich. Eine Providerantwort oder Wallet-Signatur beweist keine abgeschlossene Rückerstattung.",
    reconcile: "Endgültigen Rückerstattungsnachweis abgleichen",
    lastError: "Letzter dauerhafter Fehler:",
  },
  errors: GERMAN_ERRORS,
};

const ARABIC: RefundClaimMessages = {
  proposal: {
    ariaLabel: "مساحة محفظة طلبات استرداد الأموال",
    eyebrow: "طلب استرداد / موافقة شخصين",
    title: "إعداد طلب استرداد",
    badge: "تبقى الحيازة معك",
    description: "تحمّل الخدمة تفاصيل الدفع المعتمدة وتُعدّ الاسترداد للمراجعة. يمكنك مراجعة النتيجة، لكن لا يمكنك تغيير التفاصيل المالية أو تفاصيل المستلم في المتصفح.",
    correctionReference: "مرجع التصحيح المعتمد",
    correctionPlaceholder: "ألصق مرجع التصحيح المعتمد",
    requestReference: "مرجع الطلب",
    requestHelp: "يمنع المرجع الفريد إنشاء طلبات مكررة عند إعادة المحاولة.",
    preparing: "جارٍ الإعداد…",
    prepare: "إعداد طلب استرداد",
    refundRequestReference: "مرجع طلب الاسترداد",
    savedRequestPlaceholder: "ألصق مرجع طلب محفوظاً",
    load: "تحميل الطلب المحفوظ",
    serviceUnavailable: "الخدمة غير متاحة. لا يتم استخدام بديل محلي أو حالة تسوية محلية.",
  },
  review: {
    ariaLabel: "مراجعة أثر الاسترداد غير القابل للتغيير",
    eyebrow: "مراجعة الأثر غير القابل للتغيير",
    title: "تفويض المحفظة",
    noRequest: "لم يتم تحميل طلب استرداد",
    noRequestCopy: "أعد طلباً أو استرجعه لمراجعة تفاصيل الدفع وحالة الموافقة.",
    grossRefund: "إجمالي الاسترداد",
    taxIncluded: "الضريبة المضمنة",
    refundAfterTax: "الاسترداد بعد الضريبة",
    asset: "الأصل",
    destination: "الوجهة",
    onChainRecord: "سجل الاسترداد على الشبكة",
    refundAccount: "حساب الاسترداد",
    expiry: "الانتهاء",
    submissionReference: "مرجع الإرسال",
    noneRecorded: "لا يوجد سجل",
    finalizedTitle: "تطابق دليل حدث الاسترداد النهائي مع الأثر الدائم.",
    finalizedCopy: "يرتبط دليل الحدث بالتوقيع المسجل والموضع النهائي ومعرّف المطالبة والمبالغ وبصمة الدليل غير القابلة للتغيير. النهائية دليل على الحركة وليست حكماً قانونياً أو حكماً بالامتثال.",
    finalizedProof: "دليل الحدث النهائي",
    event: "الحدث",
    finalizedSlot: "الموضع النهائي",
    eventIndex: "فهرس الحدث",
    eventSignature: "توقيع الحدث",
    grossTaxRefund: "الإجمالي / الضريبة / الاسترداد",
    evidenceDigest: "بصمة الدليل",
  },
  wallet: {
    twoPersonNote: "يجب أن يوافق شخصان مختلفان على هذا الاسترداد: مالك الطلب أولاً، ثم مالك حساب الاسترداد المكوّن. تتحقق الخدمة من عدم تغيير تفاصيل الدفع المعتمدة.",
    signProposer: "التوقيع بصفة مقدم الطلب",
    signFunder: "التوقيع بصفة ممول الاسترداد",
    recovery: "بيانات الاسترداد (متقدم)",
    recoveryPlaceholder: "ألصق ملف المحفظة غير المتغير بعد إعادة تحميل المتصفح.",
    recoveryHelp: "تبقى هذه البيانات المؤقتة في الذاكرة وتُرسل دون تغيير للتحقق.",
    sending: "جارٍ الإرسال…",
    send: "إرسال الاسترداد المعتمد",
  },
  evidence: {
    reconcileWarning: "تتطلب هذه الحالة مطابقة دائمة مع الشبكة. لا تثبت استجابة المزوّد أو توقيع المحفظة اكتمال الاسترداد.",
    reconcile: "مطابقة دليل المطالبة النهائي",
    lastError: "آخر خطأ دائم:",
  },
  errors: ARABIC_ERRORS,
};

const JAPANESE: RefundClaimMessages = {
  proposal: {
    ariaLabel: "返金申請ウォレットワークスペース",
    eyebrow: "返金申請 / 二者承認",
    title: "返金申請を準備",
    badge: "保管権限はあなたに残ります",
    description: "サービスが承認済みの支払い情報を読み込み、返金を確認用に準備します。結果は確認できますが、ブラウザで金額や受取先を変更することはできません。",
    correctionReference: "承認済み訂正参照",
    correctionPlaceholder: "承認済み訂正参照を貼り付け",
    requestReference: "申請参照",
    requestHelp: "一意の参照により再試行時の重複申請を防ぎます。",
    preparing: "準備中…",
    prepare: "返金申請を準備",
    refundRequestReference: "返金申請参照",
    savedRequestPlaceholder: "保存した申請参照を貼り付け",
    load: "保存した申請を読み込む",
    serviceUnavailable: "サービスを利用できません。クライアント側の代替処理やローカル決済状態は使用されません。",
  },
  review: {
    ariaLabel: "不変の返金アーティファクト確認",
    eyebrow: "不変アーティファクトの確認",
    title: "ウォレット承認",
    noRequest: "返金申請が読み込まれていません",
    noRequestCopy: "申請を準備または復元して、支払い情報と承認状態を確認します。",
    grossRefund: "返金総額",
    taxIncluded: "税込み税額",
    refundAfterTax: "税引後返金",
    asset: "アセット",
    destination: "送付先",
    onChainRecord: "オンチェーン返金レコード",
    refundAccount: "返金口座",
    expiry: "有効期限",
    submissionReference: "送信参照",
    noneRecorded: "記録なし",
    finalizedTitle: "確定した返金イベント証拠が永続アーティファクトと一致しました。",
    finalizedCopy: "イベント証拠は記録済み署名、確定スロット、請求PDA、金額、不変の証拠ダイジェストに結び付いています。確定性は移動の証拠であり、法的またはコンプライアンスの判断ではありません。",
    finalizedProof: "確定イベント証拠",
    event: "イベント",
    finalizedSlot: "確定スロット",
    eventIndex: "イベントインデックス",
    eventSignature: "イベント署名",
    grossTaxRefund: "総額 / 税 / 返金",
    evidenceDigest: "証拠ダイジェスト",
  },
  wallet: {
    twoPersonNote: "2人の異なる担当者が返金を承認する必要があります。最初に申請者、次に設定済み返金口座の所有者が承認します。サービスが承認済み支払い情報の変更を確認します。",
    signProposer: "申請者として署名",
    signFunder: "返金資金提供者として署名",
    recovery: "復元データ（詳細）",
    recoveryPlaceholder: "ブラウザ再読み込み後に変更していないウォレットファイルを貼り付け",
    recoveryHelp: "この一時データはメモリ内に保持され、変更せずに検証へ送信されます。",
    sending: "送信中…",
    send: "承認済み返金を送信",
  },
  evidence: {
    reconcileWarning: "この状態には永続的なチェーン照合が必要です。プロバイダー応答やウォレット署名は返金完了の証明ではありません。",
    reconcile: "確定した請求証拠を照合",
    lastError: "最後の永続エラー:",
  },
  errors: JAPANESE_ERRORS,
};

const CATALOG: Record<SupportedLocale, RefundClaimMessages> = {
  "en-US": ENGLISH,
  "de-DE": GERMAN,
  "ar-EG": ARABIC,
  "ja-JP": JAPANESE,
};

export function refundClaimMessagesFor(locale: string): RefundClaimMessages {
  return CATALOG[locale as SupportedLocale] ?? ENGLISH;
}
