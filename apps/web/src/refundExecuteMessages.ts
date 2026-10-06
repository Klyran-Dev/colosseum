import { SupportedLocale } from "./formatters";

export interface RefundExecuteMessages {
  proposal: {
    ariaLabel: string;
    eyebrow: string;
    title: string;
    badge: string;
    description: string;
    finalizedRequest: string;
    finalizedPlaceholder: string;
    requestReference: string;
    requestHelp: string;
    preparing: string;
    prepare: string;
    releaseReference: string;
    releasePlaceholder: string;
    load: string;
    serviceUnavailable: string;
  };
  review: {
    ariaLabel: string;
    eyebrow: string;
    title: string;
    noRelease: string;
    noReleaseCopy: string;
    onChainRecord: string;
    confirmedRequest: string;
    grossRefund: string;
    taxIncluded: string;
    refundAfterTax: string;
    asset: string;
    destination: string;
    finalApprover: string;
    refundAccount: string;
    submissionReference: string;
    noneRecorded: string;
  };
  wallet: {
    twoPersonNote: string;
    signApprover: string;
    signFunder: string;
    recovery: string;
    recoveryPlaceholder: string;
    recoveryHelp: string;
    sending: string;
    send: string;
  };
  status: {
    proofPending: string;
    finalized: string;
    lastError: string;
  };
  errors: RefundExecuteErrors;
}

export interface RefundExecuteErrors {
  authorizationRequired: string;
  claimReferenceRequired: string;
  preparationRequestFailed: (status: number) => string;
  preparationFailed: string;
  preparationReady: string;
  releaseReferenceRequired: string;
  recoveryRequestFailed: (status: number) => string;
  recoveryFailed: string;
  recoveryLoaded: string;
  prepareBeforeSigning: string;
  signableStateRequired: string;
  funderPrerequisite: string;
  signingFailed: string;
  bothSignaturesPresent: string;
  approverSignaturePresent: string;
  funderSignaturePresent: string;
  submissionRequestFailed: (status: number) => string;
  submissionFailed: string;
  prepareBeforeSubmission: string;
  signaturesRequired: string;
  submissionUnknown: string;
  submissionPending: string;
}

function displayHttpStatus(status: number): string {
  return Number.isInteger(status) && status >= 100 && status <= 599 ? String(status) : "unknown status";
}

const ENGLISH_ERRORS: RefundExecuteErrors = {
  authorizationRequired: "Sign in with an account allowed to release an approved refund. The credential is held in memory only.",
  claimReferenceRequired: "A finalized refund request reference is required; financial and recipient details are loaded by the service.",
  preparationRequestFailed: (status) => `Refund release preparation failed (${displayHttpStatus(status)}).`,
  preparationFailed: "Refund release preparation failed.",
  preparationReady: "The refund release is ready for review. Check the approval roles and recipient details before signing. No funds moved.",
  releaseReferenceRequired: "A refund release reference is required.",
  recoveryRequestFailed: (status) => `Refund release recovery failed (${displayHttpStatus(status)}).`,
  recoveryFailed: "Refund release recovery failed.",
  recoveryLoaded: "The durable release state was loaded. Previously signed bytes are not returned; paste the unchanged wallet artifact to resume after reload.",
  prepareBeforeSigning: "Prepare or recover a refund release before signing.",
  signableStateRequired: "This release is not in a wallet-signable state. Reconcile the durable state before taking another action.",
  funderPrerequisite: "The approver signature is required before the refund-funder wallet can sign.",
  signingFailed: "Refund execution wallet signing failed.",
  bothSignaturesPresent: "Both execute wallet signatures are present in local memory. Submit these unchanged bytes; a signature is not a finalized refund proof.",
  approverSignaturePresent: "Approver signature collected. Switch to the configured refund-funder wallet and collect its signature.",
  funderSignaturePresent: "Wallet signature collected. The API still must verify and submit the complete transaction.",
  submissionRequestFailed: (status) => `Refund release submission failed (${displayHttpStatus(status)}).`,
  submissionFailed: "Refund release submission failed.",
  prepareBeforeSubmission: "Prepare or recover a refund release before submission.",
  signaturesRequired: "Both required wallet signatures must be present before submission.",
  submissionUnknown: "The provider outcome is unknown. Do not submit different bytes; reconcile against finalized transaction and balance evidence.",
  submissionPending: "The provider accepted the unchanged execute transaction. The refund remains proof-pending until finalized event and balance evidence are recorded.",
};

const GERMAN_ERRORS: RefundExecuteErrors = {
  authorizationRequired: "Melden Sie sich mit einem Konto an, das eine genehmigte Rückerstattung freigeben darf. Das Zugriffstoken bleibt nur im Speicher.",
  claimReferenceRequired: "Eine bestätigte Rückerstattungsreferenz ist erforderlich; Finanz- und Empfängerdaten lädt der Dienst.",
  preparationRequestFailed: (status) => `Die Vorbereitung der Rückerstattungsfreigabe ist fehlgeschlagen (${displayHttpStatus(status)}).`,
  preparationFailed: "Die Vorbereitung der Rückerstattungsfreigabe ist fehlgeschlagen.",
  preparationReady: "Die Rückerstattungsfreigabe ist zur Prüfung bereit. Prüfen Sie Rollen und Empfänger vor dem Signieren. Es wurden keine Mittel bewegt.",
  releaseReferenceRequired: "Eine Referenz der Rückerstattungsfreigabe ist erforderlich.",
  recoveryRequestFailed: (status) => `Die Wiederherstellung der Rückerstattungsfreigabe ist fehlgeschlagen (${displayHttpStatus(status)}).`,
  recoveryFailed: "Die Wiederherstellung der Rückerstattungsfreigabe ist fehlgeschlagen.",
  recoveryLoaded: "Der dauerhafte Freigabestatus wurde geladen. Signierte Bytes werden nicht zurückgegeben; fügen Sie das unveränderte Wallet-Artefakt ein.",
  prepareBeforeSigning: "Bereiten Sie eine Rückerstattungsfreigabe vor oder stellen Sie sie wieder her, bevor Sie signieren.",
  signableStateRequired: "Diese Freigabe ist nicht in einem signierbaren Wallet-Status. Stimmen Sie den dauerhaften Status ab, bevor Sie fortfahren.",
  funderPrerequisite: "Die Genehmigersignatur ist erforderlich, bevor das Rückerstattungs-Wallet signieren kann.",
  signingFailed: "Die Wallet-Signatur für die Rückerstattung ist fehlgeschlagen.",
  bothSignaturesPresent: "Beide Wallet-Signaturen sind im Speicher vorhanden. Senden Sie diese unveränderten Bytes; eine Signatur ist kein endgültiger Rückerstattungsnachweis.",
  approverSignaturePresent: "Genehmigersignatur erfasst. Wechseln Sie zum konfigurierten Rückerstattungs-Wallet.",
  funderSignaturePresent: "Wallet-Signatur erfasst. Der Dienst muss die vollständige Transaktion weiterhin prüfen und senden.",
  submissionRequestFailed: (status) => `Die Rückerstattungsfreigabe konnte nicht gesendet werden (${displayHttpStatus(status)}).`,
  submissionFailed: "Die Rückerstattungsfreigabe konnte nicht gesendet werden.",
  prepareBeforeSubmission: "Bereiten Sie eine Rückerstattungsfreigabe vor oder stellen Sie sie wieder her, bevor Sie sie senden.",
  signaturesRequired: "Vor dem Senden müssen beide erforderlichen Wallet-Signaturen vorhanden sein.",
  submissionUnknown: "Das Providerergebnis ist unbekannt. Senden Sie keine anderen Bytes; gleichen Sie mit endgültigen Transaktions- und Saldennachweisen ab.",
  submissionPending: "Der Provider hat die unveränderte Transaktion angenommen. Die Rückerstattung bleibt ausstehend, bis Ereignis- und Saldennachweise final erfasst sind.",
};

const ARABIC_ERRORS: RefundExecuteErrors = {
  authorizationRequired: "سجّل الدخول بحساب مسموح له بإصدار استرداد معتمد. يبقى الاعتماد في الذاكرة فقط.",
  claimReferenceRequired: "مرجع طلب استرداد مؤكد مطلوب؛ تحمّل الخدمة التفاصيل المالية وتفاصيل المستلم.",
  preparationRequestFailed: (status) => `تعذر إعداد إصدار الاسترداد (${displayHttpStatus(status)}).`,
  preparationFailed: "تعذر إعداد إصدار الاسترداد.",
  preparationReady: "إصدار الاسترداد جاهز للمراجعة. تحقق من الأدوار والمستلم قبل التوقيع. لم تتحرك أي أموال.",
  releaseReferenceRequired: "مرجع إصدار الاسترداد مطلوب.",
  recoveryRequestFailed: (status) => `تعذر استرداد إصدار الاسترداد (${displayHttpStatus(status)}).`,
  recoveryFailed: "تعذر استرداد إصدار الاسترداد.",
  recoveryLoaded: "تم تحميل حالة الإصدار الدائمة. لا تعيد الخدمة وحدات البايت الموقعة؛ ألصق أثر المحفظة غير المتغير للمتابعة.",
  prepareBeforeSigning: "أعد إصدار الاسترداد أو استرجعه قبل التوقيع.",
  signableStateRequired: "هذا الإصدار ليس في حالة تسمح بتوقيع المحفظة. طابق الحالة الدائمة قبل اتخاذ إجراء آخر.",
  funderPrerequisite: "توقيع الموافق مطلوب قبل أن توقع محفظة ممول الاسترداد.",
  signingFailed: "فشل توقيع محفظة تنفيذ الاسترداد.",
  bothSignaturesPresent: "توقيعا محفظتي التنفيذ موجودان في الذاكرة. أرسل وحدات البايت دون تغيير؛ التوقيع ليس دليلاً نهائياً على الاسترداد.",
  approverSignaturePresent: "تم جمع توقيع الموافق. بدّل إلى محفظة ممول الاسترداد المكوّنة واجمع توقيعها.",
  funderSignaturePresent: "تم جمع توقيع المحفظة. لا تزال الخدمة مطالبة بالتحقق من المعاملة الكاملة وإرسالها.",
  submissionRequestFailed: (status) => `تعذر إرسال إصدار الاسترداد (${displayHttpStatus(status)}).`,
  submissionFailed: "تعذر إرسال إصدار الاسترداد.",
  prepareBeforeSubmission: "أعد إصدار الاسترداد أو استرجعه قبل الإرسال.",
  signaturesRequired: "يجب وجود توقيعي المحفظة المطلوبين قبل الإرسال.",
  submissionUnknown: "نتيجة المزوّد غير معروفة. لا ترسل وحدات بايت مختلفة؛ طابق دليل المعاملة والرصيد النهائي.",
  submissionPending: "قبل المزوّد المعاملة دون تغيير. يظل الاسترداد بانتظار الدليل حتى تسجيل دليل الحدث والرصيد النهائي.",
};

const JAPANESE_ERRORS: RefundExecuteErrors = {
  authorizationRequired: "承認済み返金をリリースできるアカウントでサインインしてください。認証情報はメモリ内だけに保持されます。",
  claimReferenceRequired: "確定済み返金申請参照が必要です。金額と受取先はサービスが読み込みます。",
  preparationRequestFailed: (status) => `返金リリースの準備に失敗しました（${displayHttpStatus(status)}）。`,
  preparationFailed: "返金リリースの準備に失敗しました。",
  preparationReady: "返金リリースの確認準備ができました。署名前に承認者と受取先を確認してください。資金は移動していません。",
  releaseReferenceRequired: "返金リリース参照が必要です。",
  recoveryRequestFailed: (status) => `返金リリースを復元できませんでした（${displayHttpStatus(status)}）。`,
  recoveryFailed: "返金リリースを復元できませんでした。",
  recoveryLoaded: "永続化されたリリース状態を読み込みました。署名済みバイトは返されないため、変更していないウォレットアーティファクトを貼り付けてください。",
  prepareBeforeSigning: "署名前に返金リリースを準備または復元してください。",
  signableStateRequired: "このリリースはウォレット署名可能な状態ではありません。別の操作の前に永続状態を照合してください。",
  funderPrerequisite: "返金資金提供者のウォレットが署名する前に承認者の署名が必要です。",
  signingFailed: "返金実行ウォレットの署名に失敗しました。",
  bothSignaturesPresent: "2つの実行ウォレット署名がメモリ内にあります。変更していないバイトを送信してください。署名は確定返金の証拠ではありません。",
  approverSignaturePresent: "承認者の署名を取得しました。設定済みの返金資金提供者ウォレットに切り替えて署名してください。",
  funderSignaturePresent: "ウォレット署名を取得しました。サービスは完全なトランザクションを検証して送信する必要があります。",
  submissionRequestFailed: (status) => `返金リリースを送信できませんでした（${displayHttpStatus(status)}）。`,
  submissionFailed: "返金リリースを送信できませんでした。",
  prepareBeforeSubmission: "送信前に返金リリースを準備または復元してください。",
  signaturesRequired: "送信前に必要な2つのウォレット署名を用意してください。",
  submissionUnknown: "プロバイダーの結果が不明です。別のバイトを送信せず、確定トランザクションと残高証拠を照合してください。",
  submissionPending: "変更していない実行トランザクションがプロバイダーに受理されました。確定イベントと残高証拠が記録されるまで返金は証拠待ちです。",
};

const ENGLISH: RefundExecuteMessages = {
  proposal: {
    ariaLabel: "Refund execution wallet workspace",
    eyebrow: "Refund release / final approval",
    title: "Prepare refund release",
    badge: "Proof required",
    description: "A refund can be released only after the original request is confirmed. The service loads the approved payment and recipient details; the browser cannot change them.",
    finalizedRequest: "Finalized refund request reference",
    finalizedPlaceholder: "Paste the confirmed refund request reference",
    requestReference: "Request reference",
    requestHelp: "A unique reference prevents duplicate release requests when you retry.",
    preparing: "Preparing…",
    prepare: "Prepare refund release",
    releaseReference: "Refund release reference",
    releasePlaceholder: "Paste a saved release reference",
    load: "Load saved release",
    serviceUnavailable: "The service is unavailable. No client-side fallback or local settlement state is used.",
  },
  review: {
    ariaLabel: "Immutable refund release artifact review",
    eyebrow: "Immutable artifact review",
    title: "Execution authorization",
    noRelease: "No refund release loaded",
    noReleaseCopy: "Prepare or recover a release to review its confirmed payment and recipient details.",
    onChainRecord: "On-chain refund record",
    confirmedRequest: "Confirmed request reference",
    grossRefund: "Gross refund",
    taxIncluded: "Tax included",
    refundAfterTax: "Refund after tax",
    asset: "Asset",
    destination: "Destination",
    finalApprover: "Final approver",
    refundAccount: "Refund account",
    submissionReference: "Submission reference",
    noneRecorded: "None recorded",
  },
  wallet: {
    twoPersonNote: "Two different people must approve this release: the final approver first, then the configured refund account owner. The service checks that the approved details were not changed.",
    signApprover: "Sign as approver",
    signFunder: "Sign as refund funder",
    recovery: "Recovery data (advanced)",
    recoveryPlaceholder: "Paste the unchanged wallet file after a browser reload.",
    recoveryHelp: "This temporary data stays in memory and is sent unchanged for verification.",
    sending: "Sending…",
    send: "Send approved refund",
  },
  status: {
    proofPending: "Proof pending: provider acknowledgement is not a completed refund. Finalized transaction-bound event and balance conservation evidence are required.",
    finalized: "Finalized movement evidence is recorded. Review the linked event and balance proof before treating the refund as complete.",
    lastError: "Last durable error:",
  },
  errors: ENGLISH_ERRORS,
};

const GERMAN: RefundExecuteMessages = {
  proposal: {
    ariaLabel: "Wallet-Arbeitsbereich für die Rückerstattungsfreigabe",
    eyebrow: "Rückerstattungsfreigabe / letzte Genehmigung",
    title: "Rückerstattung freigeben",
    badge: "Nachweis erforderlich",
    description: "Eine Rückerstattung kann erst freigegeben werden, wenn der ursprüngliche Antrag bestätigt ist. Der Dienst lädt genehmigte Zahlungs- und Empfängerdaten; der Browser kann sie nicht ändern.",
    finalizedRequest: "Referenz der bestätigten Rückerstattungsanfrage",
    finalizedPlaceholder: "Bestätigte Referenz der Rückerstattungsanfrage einfügen",
    requestReference: "Antragsreferenz",
    requestHelp: "Eine eindeutige Referenz verhindert doppelte Freigabeanträge bei Wiederholungen.",
    preparing: "Wird vorbereitet…",
    prepare: "Rückerstattung freigeben",
    releaseReference: "Referenz der Rückerstattungsfreigabe",
    releasePlaceholder: "Gespeicherte Freigabereferenz einfügen",
    load: "Gespeicherte Freigabe laden",
    serviceUnavailable: "Der Dienst ist nicht verfügbar. Es wird kein clientseitiger Ersatz oder lokaler Abwicklungsstatus verwendet.",
  },
  review: {
    ariaLabel: "Prüfung des unveränderlichen Rückerstattungsfreigabeartefakts",
    eyebrow: "Prüfung des unveränderlichen Artefakts",
    title: "Ausführungsautorisierung",
    noRelease: "Keine Rückerstattungsfreigabe geladen",
    noReleaseCopy: "Bereiten Sie eine Freigabe vor oder laden Sie sie, um bestätigte Zahlungs- und Empfängerdaten zu prüfen.",
    onChainRecord: "Rückerstattungsdatensatz auf der Chain",
    confirmedRequest: "Referenz des bestätigten Antrags",
    grossRefund: "Bruttorückerstattung",
    taxIncluded: "Enthaltene Steuer",
    refundAfterTax: "Rückerstattung nach Steuer",
    asset: "Asset",
    destination: "Ziel",
    finalApprover: "Letzter Genehmiger",
    refundAccount: "Rückerstattungskonto",
    submissionReference: "Übermittlungsreferenz",
    noneRecorded: "Keine erfasst",
  },
  wallet: {
    twoPersonNote: "Zwei verschiedene Personen müssen diese Freigabe genehmigen: zuerst der letzte Genehmiger, danach der Inhaber des konfigurierten Rückerstattungskontos. Der Dienst prüft, dass die genehmigten Daten unverändert sind.",
    signApprover: "Als Genehmiger signieren",
    signFunder: "Als Rückerstattungsfinanzierer signieren",
    recovery: "Wiederherstellungsdaten (erweitert)",
    recoveryPlaceholder: "Unveränderte Wallet-Datei nach einem Browser-Neuladen einfügen.",
    recoveryHelp: "Diese temporären Daten bleiben im Speicher und werden zur Prüfung unverändert übertragen.",
    sending: "Wird gesendet…",
    send: "Genehmigte Rückerstattung senden",
  },
  status: {
    proofPending: "Nachweis ausstehend: Die Providerbestätigung ist keine abgeschlossene Rückerstattung. Ein endgültiger, transaktionsgebundener Ereignis- und Saldennachweis ist erforderlich.",
    finalized: "Endgültiger Bewegungsnachweis erfasst. Prüfen Sie das verknüpfte Ereignis und den Saldennachweis, bevor Sie die Rückerstattung als abgeschlossen behandeln.",
    lastError: "Letzter dauerhafter Fehler:",
  },
  errors: GERMAN_ERRORS,
};

const ARABIC: RefundExecuteMessages = {
  proposal: {
    ariaLabel: "مساحة محفظة إصدار الاسترداد",
    eyebrow: "إصدار الاسترداد / الموافقة النهائية",
    title: "إعداد إصدار الاسترداد",
    badge: "الدليل مطلوب",
    description: "لا يمكن إصدار الاسترداد إلا بعد تأكيد الطلب الأصلي. تحمّل الخدمة تفاصيل الدفع والمستلم المعتمدة؛ ولا يمكن للمتصفح تغييرها.",
    finalizedRequest: "مرجع طلب الاسترداد المؤكد",
    finalizedPlaceholder: "ألصق مرجع طلب الاسترداد المؤكد",
    requestReference: "مرجع الطلب",
    requestHelp: "يمنع المرجع الفريد إنشاء طلبات إصدار مكررة عند إعادة المحاولة.",
    preparing: "جارٍ الإعداد…",
    prepare: "إعداد إصدار الاسترداد",
    releaseReference: "مرجع إصدار الاسترداد",
    releasePlaceholder: "ألصق مرجع إصدار محفوظاً",
    load: "تحميل الإصدار المحفوظ",
    serviceUnavailable: "الخدمة غير متاحة. لا يتم استخدام بديل محلي أو حالة تسوية محلية.",
  },
  review: {
    ariaLabel: "مراجعة أثر إصدار الاسترداد غير القابل للتغيير",
    eyebrow: "مراجعة الأثر غير القابل للتغيير",
    title: "تفويض التنفيذ",
    noRelease: "لم يتم تحميل إصدار استرداد",
    noReleaseCopy: "أعد إصداراً أو استرجعه لمراجعة تفاصيل الدفع والمستلم المؤكدة.",
    onChainRecord: "سجل الاسترداد على الشبكة",
    confirmedRequest: "مرجع الطلب المؤكد",
    grossRefund: "إجمالي الاسترداد",
    taxIncluded: "الضريبة المضمنة",
    refundAfterTax: "الاسترداد بعد الضريبة",
    asset: "الأصل",
    destination: "الوجهة",
    finalApprover: "الموافق النهائي",
    refundAccount: "حساب الاسترداد",
    submissionReference: "مرجع الإرسال",
    noneRecorded: "لا يوجد سجل",
  },
  wallet: {
    twoPersonNote: "يجب أن يوافق شخصان مختلفان على هذا الإصدار: الموافق النهائي أولاً، ثم مالك حساب الاسترداد المكوّن. تتحقق الخدمة من عدم تغيير التفاصيل المعتمدة.",
    signApprover: "التوقيع بصفة الموافق",
    signFunder: "التوقيع بصفة ممول الاسترداد",
    recovery: "بيانات الاسترداد (متقدم)",
    recoveryPlaceholder: "ألصق ملف المحفظة غير المتغير بعد إعادة تحميل المتصفح.",
    recoveryHelp: "تبقى هذه البيانات المؤقتة في الذاكرة وتُرسل دون تغيير للتحقق.",
    sending: "جارٍ الإرسال…",
    send: "إرسال الاسترداد المعتمد",
  },
  status: {
    proofPending: "الدليل قيد الانتظار: إقرار المزوّد ليس استرداداً مكتملًا. يلزم دليل حدث نهائي مرتبط بالمعاملة ودليل حفظ الأرصدة.",
    finalized: "تم تسجيل دليل الحركة النهائي. راجع الحدث المرتبط ودليل الرصيد قبل اعتبار الاسترداد مكتملًا.",
    lastError: "آخر خطأ دائم:",
  },
  errors: ARABIC_ERRORS,
};

const JAPANESE: RefundExecuteMessages = {
  proposal: {
    ariaLabel: "返金リリースウォレットワークスペース",
    eyebrow: "返金リリース / 最終承認",
    title: "返金リリースを準備",
    badge: "証拠が必要",
    description: "元の申請が確認されるまで返金はリリースできません。承認済みの支払い情報と受取先をサービスが読み込み、ブラウザから変更できないようにします。",
    finalizedRequest: "確定済み返金申請参照",
    finalizedPlaceholder: "確認済みの返金申請参照を貼り付け",
    requestReference: "申請参照",
    requestHelp: "一意の参照により再試行時の重複リリース申請を防ぎます。",
    preparing: "準備中…",
    prepare: "返金リリースを準備",
    releaseReference: "返金リリース参照",
    releasePlaceholder: "保存したリリース参照を貼り付け",
    load: "保存したリリースを読み込む",
    serviceUnavailable: "サービスを利用できません。クライアント側の代替処理やローカル決済状態は使用されません。",
  },
  review: {
    ariaLabel: "不変の返金リリースアーティファクト確認",
    eyebrow: "不変アーティファクトの確認",
    title: "実行承認",
    noRelease: "返金リリースが読み込まれていません",
    noReleaseCopy: "リリースを準備または復元して、確認済みの支払い情報と受取先を確認します。",
    onChainRecord: "オンチェーン返金レコード",
    confirmedRequest: "確認済み申請参照",
    grossRefund: "返金総額",
    taxIncluded: "税込み税額",
    refundAfterTax: "税引後返金",
    asset: "アセット",
    destination: "送付先",
    finalApprover: "最終承認者",
    refundAccount: "返金口座",
    submissionReference: "送信参照",
    noneRecorded: "記録なし",
  },
  wallet: {
    twoPersonNote: "2人の異なる担当者がこのリリースを承認する必要があります。最初に最終承認者、次に設定済み返金口座の所有者が承認します。サービスが承認済み情報の変更を確認します。",
    signApprover: "承認者として署名",
    signFunder: "返金資金提供者として署名",
    recovery: "復元データ（詳細）",
    recoveryPlaceholder: "ブラウザ再読み込み後に変更していないウォレットファイルを貼り付け",
    recoveryHelp: "この一時データはメモリ内に保持され、変更せずに検証へ送信されます。",
    sending: "送信中…",
    send: "承認済み返金を送信",
  },
  status: {
    proofPending: "証拠待ち: プロバイダーの確認は返金完了を意味しません。確定したトランザクションイベントと残高保全の証拠が必要です。",
    finalized: "確定した移動証拠が記録されています。返金を完了として扱う前に、関連イベントと残高証拠を確認してください。",
    lastError: "最後の永続エラー:",
  },
  errors: JAPANESE_ERRORS,
};

const CATALOG: Record<SupportedLocale, RefundExecuteMessages> = {
  "en-US": ENGLISH,
  "de-DE": GERMAN,
  "ar-EG": ARABIC,
  "ja-JP": JAPANESE,
};

export function refundExecuteMessagesFor(locale: string): RefundExecuteMessages {
  return CATALOG[locale as SupportedLocale] ?? ENGLISH;
}
