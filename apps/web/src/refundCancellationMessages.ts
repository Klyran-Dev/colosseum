import { SupportedLocale } from "./formatters";

export interface RefundCancellationMessages {
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
    cancellationReference: string;
    cancellationPlaceholder: string;
    load: string;
    serviceUnavailable: string;
  };
  review: {
    ariaLabel: string;
    eyebrow: string;
    title: string;
    noCancellation: string;
    noCancellationCopy: string;
    onChainRecord: string;
    approvalAuthority: string;
    merchant: string;
    asset: string;
    adjustmentRecord: string;
    releaseReferenceNumber: string;
    ledgerRegistry: string;
    instructionEvidence: string;
    requestEvidence: string;
    submissionReference: string;
    noneRecorded: string;
    noFundsMovedTitle: string;
    noFundsMovedCopy: string;
    finalizedTitle: string;
    finalizedCopy: string;
    finalizedProof: string;
    event: string;
    finalizedSlot: string;
    eventIndex: string;
    eventSignature: string;
    evidenceDigest: string;
  };
  wallet: {
    signNote: string;
    sign: string;
    recovery: string;
    recoveryPlaceholder: string;
    recoveryHelp: string;
    sending: string;
    send: string;
  };
  evidence: {
    providerNotProof: string;
    reconcile: string;
  };
  notices: {
    prepared: string;
    loaded: string;
    signed: string;
    noSignature: string;
    unknownSubmission: string;
    submitted: string;
    reconciled: string;
  };
  status: {
    lastError: string;
    finalizedIncomplete: string;
    labels: Record<string, string>;
  };
  errors: {
    auth: string;
    finalizedRequest: string;
    cancellationReference: string;
    prepare: string;
    recovery: string;
    sign: string;
    submit: string;
    reconcile: string;
    prepareBeforeSign: string;
    signableState: string;
    signatureRequired: string;
    httpError: (message: string, status: number) => string;
  };
}

function displayHttpStatus(status: number): string {
  return Number.isInteger(status) && status >= 100 && status <= 599 ? String(status) : "unknown status";
}

const ENGLISH: RefundCancellationMessages = {
  proposal: {
    ariaLabel: "Refund claim cancellation workspace",
    eyebrow: "Refund request / controlled cancellation",
    title: "Cancel a refund request",
    badge: "No money moved",
    description: "Cancellation stops a confirmed refund request from moving forward. The service loads the account details and binds this decision to the approved governance wallet.",
    finalizedRequest: "Finalized refund request reference",
    finalizedPlaceholder: "Paste the confirmed refund request reference",
    requestReference: "Request reference",
    requestHelp: "A unique reference prevents duplicate cancellation requests when you retry.",
    preparing: "Preparing…",
    prepare: "Prepare cancellation",
    cancellationReference: "Cancellation reference",
    cancellationPlaceholder: "Paste a saved cancellation reference",
    load: "Load saved cancellation",
    serviceUnavailable: "The service is unavailable. No client-side fallback or local cancellation state is used.",
  },
  review: {
    ariaLabel: "Immutable cancellation artifact review",
    eyebrow: "Immutable artifact review",
    title: "Governance wallet authorization",
    noCancellation: "No cancellation loaded",
    noCancellationCopy: "Prepare or recover a cancellation to review its approval details before signing.",
    onChainRecord: "On-chain refund record",
    approvalAuthority: "Approval authority",
    merchant: "Merchant",
    asset: "Asset",
    adjustmentRecord: "Adjustment record",
    releaseReferenceNumber: "Release reference number",
    ledgerRegistry: "Ledger registry",
    instructionEvidence: "Instruction evidence",
    requestEvidence: "Request evidence",
    submissionReference: "Submission reference",
    noneRecorded: "None recorded",
    noFundsMovedTitle: "Cancellation moves no funds.",
    noFundsMovedCopy: "The on-chain instruction only marks the refund claim cancelled after governance authorization. This UI does not infer finality from wallet signing or provider acknowledgement.",
    finalizedTitle: "Finalized cancellation event evidence matched the durable artifact.",
    finalizedCopy: "The claim is now durably cancelled. This is movement-state evidence only; it is not legal or tax advice.",
    finalizedProof: "Finalized event proof",
    event: "Event",
    finalizedSlot: "Finalized slot",
    eventIndex: "Event index",
    eventSignature: "Event signature",
    evidenceDigest: "Evidence digest",
  },
  wallet: {
    signNote: "One governance wallet signature is required. The service checks that the approved cancellation details were not changed before submission.",
    sign: "Sign as approval authority",
    recovery: "Recovery data (advanced)",
    recoveryPlaceholder: "Paste the unchanged wallet file after a browser reload.",
    recoveryHelp: "This temporary data stays in memory and is sent unchanged for verification.",
    sending: "Sending…",
    send: "Send cancellation",
  },
  evidence: {
    providerNotProof: "A provider response or wallet signature is not proof of a completed cancellation. The server must recover the exact transaction at finalized commitment and match the typed event.",
    reconcile: "Reconcile finalized cancellation evidence",
  },
  notices: {
    prepared: "The cancellation is ready for review. It cannot move funds; check the refund request and approval details before signing.",
    loaded: "The saved cancellation was loaded. For safety, the service does not return signed wallet data; paste the unchanged wallet file to continue.",
    signed: "The governance wallet signature is present in local memory. Submit these unchanged bytes; a signature is not finalized cancellation evidence.",
    noSignature: "The wallet did not return a signed transaction artifact.",
    unknownSubmission: "The provider outcome is unknown. Do not sign or submit different bytes; finalized chain evidence is required.",
    submitted: "The provider acknowledged the unchanged cancellation transaction. This is transport evidence only; the refund claim remains active until finalized cancellation event evidence is reconciled.",
    reconciled: "Finalized cancellation event evidence matched the durable artifact, and the original refund claim was converged atomically to cancelled.",
  },
  status: {
    lastError: "Last durable error:",
    finalizedIncomplete: "The durable status is finalized but its typed event proof is unavailable. Treat this as an operational inconsistency and do not infer completion.",
    labels: {
      prepared_unsubmitted: "Prepared — not submitted",
      signed: "Signed",
      submitting: "Submitting",
      submitted: "Submitted",
      processed: "Processed",
      confirmed: "Confirmed",
      finalized: "Finalized",
      rejected: "Rejected",
      unknown: "Unknown",
    },
  },
  errors: {
    auth: "Sign in with an account allowed to cancel refund requests. The credential is held in memory only.",
    finalizedRequest: "A finalized refund request reference is required; the service loads the account and approval details.",
    cancellationReference: "A cancellation reference is required.",
    prepare: "Refund cancellation preparation failed.",
    recovery: "Refund cancellation recovery failed.",
    sign: "Refund cancellation wallet signing failed.",
    submit: "Refund cancellation submission failed.",
    reconcile: "Refund cancellation reconciliation failed.",
    prepareBeforeSign: "Prepare or recover a cancellation before signing.",
    signableState: "This cancellation is not in a wallet-signable state. Reconcile its durable state before taking another action.",
    signatureRequired: "The governance wallet signature must be present before submission.",
    httpError: (message, status) => `${message} (HTTP ${displayHttpStatus(status)})`,
  },
};

const GERMAN: RefundCancellationMessages = {
  proposal: {
    ariaLabel: "Wallet-Arbeitsbereich für die Stornierung von Rückerstattungsanträgen",
    eyebrow: "Rückerstattungsantrag / kontrollierte Stornierung",
    title: "Rückerstattungsantrag stornieren",
    badge: "Keine Geldbewegung",
    description: "Die Stornierung stoppt einen bestätigten Rückerstattungsantrag. Der Dienst lädt die Kontodaten und bindet diese Entscheidung an die genehmigte Governance-Wallet.",
    finalizedRequest: "Referenz der bestätigten Rückerstattungsanfrage",
    finalizedPlaceholder: "Bestätigte Referenz der Rückerstattungsanfrage einfügen",
    requestReference: "Antragsreferenz",
    requestHelp: "Eine eindeutige Referenz verhindert doppelte Stornierungsanträge bei Wiederholungen.",
    preparing: "Wird vorbereitet…",
    prepare: "Stornierung vorbereiten",
    cancellationReference: "Referenz der Stornierung",
    cancellationPlaceholder: "Gespeicherte Stornierungsreferenz einfügen",
    load: "Gespeicherte Stornierung laden",
    serviceUnavailable: "Der Dienst ist nicht verfügbar. Es wird kein clientseitiger Ersatz oder lokaler Stornierungsstatus verwendet.",
  },
  review: {
    ariaLabel: "Prüfung des unveränderlichen Stornierungsartefakts",
    eyebrow: "Prüfung des unveränderlichen Artefakts",
    title: "Autorisierung der Governance-Wallet",
    noCancellation: "Keine Stornierung geladen",
    noCancellationCopy: "Bereiten Sie eine Stornierung vor oder laden Sie sie, um die Genehmigungsdetails vor der Signatur zu prüfen.",
    onChainRecord: "Rückerstattungsdatensatz auf der Chain",
    approvalAuthority: "Genehmigungsstelle",
    merchant: "Händler",
    asset: "Asset",
    adjustmentRecord: "Korrekturdatensatz",
    releaseReferenceNumber: "Referenznummer der Freigabe",
    ledgerRegistry: "Ledger-Register",
    instructionEvidence: "Nachweis der Anweisung",
    requestEvidence: "Nachweis des Antrags",
    submissionReference: "Übermittlungsreferenz",
    noneRecorded: "Keine erfasst",
    noFundsMovedTitle: "Die Stornierung bewegt kein Geld.",
    noFundsMovedCopy: "Die On-Chain-Anweisung markiert den Rückerstattungsantrag erst nach Governance-Autorisierung als storniert. Diese Oberfläche leitet die Finalität nicht aus Wallet-Signatur oder Providerbestätigung ab.",
    finalizedTitle: "Der endgültige Nachweis des Stornierungsereignisses stimmt mit dem dauerhaften Artefakt überein.",
    finalizedCopy: "Der Antrag ist dauerhaft storniert. Dies ist ausschließlich ein Nachweis des Bewegungsstatus und keine Rechts- oder Steuerberatung.",
    finalizedProof: "Nachweis des endgültigen Ereignisses",
    event: "Ereignis",
    finalizedSlot: "Endgültiger Slot",
    eventIndex: "Ereignisindex",
    eventSignature: "Ereignissignatur",
    evidenceDigest: "Nachweis-Digest",
  },
  wallet: {
    signNote: "Eine Signatur der Governance-Wallet ist erforderlich. Der Dienst prüft vor der Übermittlung, dass die genehmigten Stornierungsdaten unverändert sind.",
    sign: "Als Genehmigungsstelle signieren",
    recovery: "Wiederherstellungsdaten (erweitert)",
    recoveryPlaceholder: "Unveränderte Wallet-Datei nach einem Browser-Neuladen einfügen.",
    recoveryHelp: "Diese temporären Daten bleiben im Speicher und werden zur Prüfung unverändert übertragen.",
    sending: "Wird gesendet…",
    send: "Stornierung senden",
  },
  evidence: {
    providerNotProof: "Eine Providerantwort oder Wallet-Signatur beweist keine abgeschlossene Stornierung. Der Dienst muss die exakte Transaktion bei endgültiger Bestätigung finden und das typisierte Ereignis abgleichen.",
    reconcile: "Endgültigen Stornierungsnachweis abgleichen",
  },
  notices: {
    prepared: "Die Stornierung ist zur Prüfung bereit. Sie bewegt kein Geld; prüfen Sie Rückerstattungsantrag und Genehmigungsdetails vor der Signatur.",
    loaded: "Die gespeicherte Stornierung wurde geladen. Aus Sicherheitsgründen gibt der Dienst keine signierten Wallet-Daten zurück; fügen Sie die unveränderte Wallet-Datei ein.",
    signed: "Die Signatur der Governance-Wallet befindet sich im temporären Speicher. Übermitteln Sie diese unveränderten Bytes; eine Signatur ist kein endgültiger Stornierungsnachweis.",
    noSignature: "Die Wallet hat kein signiertes Transaktionsartefakt zurückgegeben.",
    unknownSubmission: "Das Providerergebnis ist unbekannt. Signieren oder übermitteln Sie keine anderen Bytes; ein endgültiger Chain-Nachweis ist erforderlich.",
    submitted: "Der Provider hat die unveränderte Stornierungstransaktion bestätigt. Dies ist nur ein Transportnachweis; der Rückerstattungsantrag bleibt aktiv, bis der endgültige Ereignisnachweis abgeglichen ist.",
    reconciled: "Der endgültige Nachweis des Stornierungsereignisses stimmt mit dem dauerhaften Artefakt überein; der ursprüngliche Rückerstattungsantrag wurde atomar auf storniert gesetzt.",
  },
  status: {
    lastError: "Letzter dauerhafter Fehler:",
    finalizedIncomplete: "Der dauerhafte Status ist endgültig, aber der typisierte Ereignisnachweis fehlt. Behandeln Sie dies als Betriebsinkonsistenz und leiten Sie keinen Abschluss ab.",
    labels: { prepared_unsubmitted: "Vorbereitet — nicht übermittelt", signed: "Signiert", submitting: "Wird übermittelt", submitted: "Übermittelt", processed: "Verarbeitet", confirmed: "Bestätigt", finalized: "Endgültig", rejected: "Abgelehnt", unknown: "Unbekannt" },
  },
  errors: {
    auth: "Melden Sie sich mit einem Konto an, das Rückerstattungsanträge stornieren darf. Das Anmeldemittel bleibt nur im Speicher.",
    finalizedRequest: "Eine Referenz der bestätigten Rückerstattungsanfrage ist erforderlich; der Dienst lädt Konto- und Genehmigungsdaten.",
    cancellationReference: "Eine Referenz der Stornierung ist erforderlich.",
    prepare: "Die Vorbereitung der Rückerstattungsstornierung ist fehlgeschlagen.",
    recovery: "Die Wiederherstellung der Rückerstattungsstornierung ist fehlgeschlagen.",
    sign: "Die Signatur der Rückerstattungsstornierung ist fehlgeschlagen.",
    submit: "Die Übermittlung der Rückerstattungsstornierung ist fehlgeschlagen.",
    reconcile: "Der Abgleich der Rückerstattungsstornierung ist fehlgeschlagen.",
    prepareBeforeSign: "Bereiten Sie eine Stornierung vor oder laden Sie sie, bevor Sie signieren.",
    signableState: "Diese Stornierung kann nicht signiert werden. Gleichen Sie den dauerhaften Status ab, bevor Sie fortfahren.",
    signatureRequired: "Vor der Übermittlung muss die Signatur der Governance-Wallet vorhanden sein.",
    httpError: (message, status) => `${message} (HTTP-Status ${displayHttpStatus(status)})`,
  },
};

const ARABIC: RefundCancellationMessages = {
  proposal: {
    ariaLabel: "مساحة محفظة إلغاء طلبات الاسترداد",
    eyebrow: "طلب استرداد / إلغاء مضبوط",
    title: "إلغاء طلب استرداد",
    badge: "لم تتحرك أموال",
    description: "يوقف الإلغاء طلب الاسترداد المؤكد من التقدم. تحمّل الخدمة تفاصيل الحساب وتربط القرار بمحفظة الحوكمة المعتمدة.",
    finalizedRequest: "مرجع طلب الاسترداد المؤكد",
    finalizedPlaceholder: "ألصق مرجع طلب الاسترداد المؤكد",
    requestReference: "مرجع الطلب",
    requestHelp: "يمنع المرجع الفريد إنشاء طلبات إلغاء مكررة عند إعادة المحاولة.",
    preparing: "جارٍ الإعداد…",
    prepare: "إعداد الإلغاء",
    cancellationReference: "مرجع الإلغاء",
    cancellationPlaceholder: "ألصق مرجع إلغاء محفوظاً",
    load: "تحميل الإلغاء المحفوظ",
    serviceUnavailable: "الخدمة غير متاحة. لا يتم استخدام بديل محلي أو حالة إلغاء محلية.",
  },
  review: {
    ariaLabel: "مراجعة أثر الإلغاء غير القابل للتغيير",
    eyebrow: "مراجعة الأثر غير القابل للتغيير",
    title: "تفويض محفظة الحوكمة",
    noCancellation: "لم يتم تحميل إلغاء",
    noCancellationCopy: "أعد إلغاء أو استرجعه لمراجعة تفاصيل الموافقة قبل التوقيع.",
    onChainRecord: "سجل الاسترداد على الشبكة",
    approvalAuthority: "جهة الموافقة",
    merchant: "التاجر",
    asset: "الأصل",
    adjustmentRecord: "سجل التصحيح",
    releaseReferenceNumber: "رقم مرجع الإصدار",
    ledgerRegistry: "سجل الدفتر",
    instructionEvidence: "دليل التعليمات",
    requestEvidence: "دليل الطلب",
    submissionReference: "مرجع الإرسال",
    noneRecorded: "لا يوجد سجل",
    noFundsMovedTitle: "لا يحرك الإلغاء أموالاً.",
    noFundsMovedCopy: "تضع تعليمات الشبكة طلب الاسترداد في حالة الإلغاء بعد تفويض الحوكمة فقط. لا تستنتج هذه الواجهة النهائية من توقيع المحفظة أو إقرار المزوّد.",
    finalizedTitle: "تطابق دليل حدث الإلغاء النهائي مع الأثر الدائم.",
    finalizedCopy: "أُلغي الطلب الآن بشكل دائم. هذا دليل على حالة الحركة فقط، وليس نصيحة قانونية أو ضريبية.",
    finalizedProof: "دليل الحدث النهائي",
    event: "الحدث",
    finalizedSlot: "الموضع النهائي",
    eventIndex: "فهرس الحدث",
    eventSignature: "توقيع الحدث",
    evidenceDigest: "بصمة الدليل",
  },
  wallet: {
    signNote: "مطلوب توقيع واحد من محفظة الحوكمة. تتحقق الخدمة من عدم تغيير تفاصيل الإلغاء المعتمدة قبل الإرسال.",
    sign: "التوقيع بصفة جهة الموافقة",
    recovery: "بيانات الاسترداد (متقدم)",
    recoveryPlaceholder: "ألصق ملف المحفظة غير المتغير بعد إعادة تحميل المتصفح.",
    recoveryHelp: "تبقى هذه البيانات المؤقتة في الذاكرة وتُرسل دون تغيير للتحقق.",
    sending: "جارٍ الإرسال…",
    send: "إرسال الإلغاء",
  },
  evidence: {
    providerNotProof: "لا تثبت استجابة المزوّد أو توقيع المحفظة اكتمال الإلغاء. يجب على الخدمة استعادة المعاملة نفسها عند التأكيد النهائي ومطابقة الحدث المعرّف.",
    reconcile: "مطابقة دليل الإلغاء النهائي",
  },
  notices: {
    prepared: "الإلغاء جاهز للمراجعة. لا يحرك أموالاً؛ راجع طلب الاسترداد وتفاصيل الموافقة قبل التوقيع.",
    loaded: "تم تحميل الإلغاء المحفوظ. حفاظاً على الأمان، لا تعيد الخدمة بيانات محفظة موقعة؛ ألصق ملف المحفظة غير المتغير للمتابعة.",
    signed: "توقيع محفظة الحوكمة موجود في الذاكرة المؤقتة. أرسل هذه البايتات دون تغيير؛ التوقيع ليس دليلاً نهائياً على الإلغاء.",
    noSignature: "لم تُعد المحفظة أثراً لمعاملة موقعة.",
    unknownSubmission: "نتيجة المزوّد غير معروفة. لا توقّع أو ترسل بايتات مختلفة؛ يلزم دليل نهائي من الشبكة.",
    submitted: "أقر المزوّد بمعاملة الإلغاء دون تغيير. هذا دليل نقل فقط؛ يظل طلب الاسترداد نشطاً حتى مطابقة دليل الحدث النهائي.",
    reconciled: "تطابق دليل حدث الإلغاء النهائي مع الأثر الدائم، ووُحّد طلب الاسترداد الأصلي بشكل ذري إلى حالة ملغى.",
  },
  status: {
    lastError: "آخر خطأ دائم:",
    finalizedIncomplete: "الحالة الدائمة نهائية، لكن دليل الحدث المعرّف غير متاح. اعتبر ذلك تناقضاً تشغيلياً ولا تستنتج الاكتمال.",
    labels: { prepared_unsubmitted: "مُعد — لم يُرسل", signed: "موقّع", submitting: "جارٍ الإرسال", submitted: "أُرسل", processed: "تمت المعالجة", confirmed: "مؤكد", finalized: "نهائي", rejected: "مرفوض", unknown: "غير معروف" },
  },
  errors: {
    auth: "سجّل الدخول بحساب مسموح له بإلغاء طلبات الاسترداد. تبقى بيانات الاعتماد في الذاكرة فقط.",
    finalizedRequest: "مرجع طلب الاسترداد المؤكد مطلوب؛ تحمّل الخدمة تفاصيل الحساب والموافقة.",
    cancellationReference: "مرجع الإلغاء مطلوب.",
    prepare: "تعذر إعداد إلغاء الاسترداد.",
    recovery: "تعذر استعادة إلغاء الاسترداد.",
    sign: "تعذر توقيع إلغاء الاسترداد.",
    submit: "تعذر إرسال إلغاء الاسترداد.",
    reconcile: "تعذر مطابقة إلغاء الاسترداد.",
    prepareBeforeSign: "أعد إلغاء أو استرجعه قبل التوقيع.",
    signableState: "هذا الإلغاء ليس في حالة قابلة لتوقيع المحفظة. طابق الحالة الدائمة قبل المتابعة.",
    signatureRequired: "يجب وجود توقيع محفظة الحوكمة قبل الإرسال.",
    httpError: (message, status) => `${message} (حالة HTTP: ${displayHttpStatus(status)})`,
  },
};

const JAPANESE: RefundCancellationMessages = {
  proposal: {
    ariaLabel: "返金申請の取消ウォレットワークスペース",
    eyebrow: "返金申請 / 管理された取消",
    title: "返金申請を取り消す",
    badge: "資金移動なし",
    description: "取消により、確認済みの返金申請の処理を停止します。サービスが口座情報を読み込み、この判断を承認済みガバナンスウォレットに結び付けます。",
    finalizedRequest: "確定済み返金申請参照",
    finalizedPlaceholder: "確認済みの返金申請参照を貼り付け",
    requestReference: "申請参照",
    requestHelp: "一意の参照により再試行時の重複取消申請を防ぎます。",
    preparing: "準備中…",
    prepare: "取消を準備",
    cancellationReference: "取消参照",
    cancellationPlaceholder: "保存済みの取消参照を貼り付け",
    load: "保存済みの取消を読み込む",
    serviceUnavailable: "サービスを利用できません。クライアント側の代替処理やローカル取消状態は使用しません。",
  },
  review: {
    ariaLabel: "不変の取消アーティファクト確認",
    eyebrow: "不変アーティファクトの確認",
    title: "ガバナンスウォレットの承認",
    noCancellation: "取消が読み込まれていません",
    noCancellationCopy: "署名前に取消を準備または復元し、承認内容を確認してください。",
    onChainRecord: "オンチェーン返金記録",
    approvalAuthority: "承認権限",
    merchant: "加盟店",
    asset: "アセット",
    adjustmentRecord: "調整記録",
    releaseReferenceNumber: "リリース参照番号",
    ledgerRegistry: "台帳レジストリ",
    instructionEvidence: "命令証拠",
    requestEvidence: "申請証拠",
    submissionReference: "送信参照",
    noneRecorded: "記録なし",
    noFundsMovedTitle: "取消では資金は移動しません。",
    noFundsMovedCopy: "オンチェーン命令はガバナンス承認後に返金申請を取り消し済みとして記録するだけです。この画面はウォレット署名やプロバイダー確認から確定性を推測しません。",
    finalizedTitle: "確定した取消イベント証拠が永続アーティファクトと一致しました。",
    finalizedCopy: "申請は永続的に取り消されました。これは移動状態の証拠であり、法的または税務上の助言ではありません。",
    finalizedProof: "確定イベント証拠",
    event: "イベント",
    finalizedSlot: "確定スロット",
    eventIndex: "イベントインデックス",
    eventSignature: "イベント署名",
    evidenceDigest: "証拠ダイジェスト",
  },
  wallet: {
    signNote: "ガバナンスウォレットの署名が1つ必要です。送信前に、承認済みの取消内容が変更されていないことをサービスが確認します。",
    sign: "承認権限として署名",
    recovery: "復元データ（詳細）",
    recoveryPlaceholder: "ブラウザ再読み込み後に変更していないウォレットファイルを貼り付け",
    recoveryHelp: "この一時データはメモリ内に保持され、変更せずに検証へ送信されます。",
    sending: "送信中…",
    send: "取消を送信",
  },
  evidence: {
    providerNotProof: "プロバイダー応答やウォレット署名は取消完了の証明ではありません。サービスが確定状態で同一の取引を復元し、型付きイベントを照合する必要があります。",
    reconcile: "確定した取消証拠を照合",
  },
  notices: {
    prepared: "取消の確認準備ができました。資金は移動しません。署名前に返金申請と承認内容を確認してください。",
    loaded: "保存済みの取消を読み込みました。安全のため署名済みウォレットデータは返しません。続行するには変更していないウォレットファイルを貼り付けてください。",
    signed: "ガバナンスウォレットの署名が一時メモリにあります。この変更していないバイト列を送信してください。署名は取消完了の確定証拠ではありません。",
    noSignature: "ウォレットから署名済みトランザクションアーティファクトが返されませんでした。",
    unknownSubmission: "プロバイダー結果が不明です。別のバイト列に署名または送信しないでください。確定したチェーン証拠が必要です。",
    submitted: "プロバイダーが変更していない取消トランザクションを受理しました。これは転送の証拠にすぎず、確定イベント証拠を照合するまで返金申請は有効です。",
    reconciled: "確定した取消イベント証拠が永続アーティファクトと一致し、元の返金申請を取消済みに原子的に収束しました。",
  },
  status: {
    lastError: "最後の永続エラー:",
    finalizedIncomplete: "永続状態は確定していますが、型付きイベント証拠を利用できません。運用上の不整合として扱い、完了と推測しないでください。",
    labels: { prepared_unsubmitted: "準備済み — 未送信", signed: "署名済み", submitting: "送信中", submitted: "送信済み", processed: "処理済み", confirmed: "確認済み", finalized: "確定済み", rejected: "拒否", unknown: "不明" },
  },
  errors: {
    auth: "返金申請の取消権限を持つアカウントでサインインしてください。資格情報はメモリ内だけに保持されます。",
    finalizedRequest: "確定済み返金申請参照が必要です。口座と承認の詳細はサービスが読み込みます。",
    cancellationReference: "取消参照が必要です。",
    prepare: "返金取消の準備に失敗しました。",
    recovery: "返金取消の復元に失敗しました。",
    sign: "返金取消のウォレット署名に失敗しました。",
    submit: "返金取消の送信に失敗しました。",
    reconcile: "返金取消の照合に失敗しました。",
    prepareBeforeSign: "署名前に取消を準備または復元してください。",
    signableState: "この取消はウォレット署名可能な状態ではありません。続行する前に永続状態を照合してください。",
    signatureRequired: "送信前にガバナンスウォレットの署名が必要です。",
    httpError: (message, status) => `${message}（HTTPステータス: ${displayHttpStatus(status)}）`,
  },
};

const CATALOG: Record<SupportedLocale, RefundCancellationMessages> = {
  "en-US": ENGLISH,
  "de-DE": GERMAN,
  "ar-EG": ARABIC,
  "ja-JP": JAPANESE,
};

export function refundCancellationMessagesFor(locale: string): RefundCancellationMessages {
  return CATALOG[locale as SupportedLocale] ?? ENGLISH;
}
