import { SupportedLocale } from "./formatters";

export interface VaultReconciliationMessages {
  control: { ariaLabel: string; eyebrow: string; title: string; badge: string; description: string; loading: string; load: string; unavailable: string };
  targets: { eyebrow: string; title: string; count: (count: number) => string; notLoaded: string; notLoadedCopy: string; empty: string; emptyCopy: string; account: string; observationSlot: string; expected: string; observed: string };
  detail: { eyebrow: string; title: string; taxVault: string; mintProgram: string; openingBalance: string; atSlot: string; openingReference: string; receiptTax: string; releases: string; staleAlert: string; mismatchAlert: string; errorAlert: string; unknown: string; history: string; recentRuns: string; loading: string; notLoaded: string };
  status: { noObservation: string; stale: string; matched: string; mismatch: string; error: string; staleShort: string };
  notice: { loaded: (count: number) => string; empty: string };
  common: { unavailable: string; lookupError: string; historyError: string; httpError: (message: string, status: number) => string };
}

function displayHttpStatus(status: number): string {
  return Number.isInteger(status) && status >= 100 && status <= 599 ? String(status) : "unknown status";
}

const ENGLISH: VaultReconciliationMessages = {
  control: { ariaLabel: "Vault reconciliation workspace", eyebrow: "Balance checks / view only", title: "Check tax balances", badge: "View only", description: "Compare recorded tax balances with the latest verified chain observation. This workspace cannot change balances, release funds, or clear an alert.", loading: "Loading evidence…", load: "Load reconciliation targets", unavailable: "The API is unavailable. No cached or client-derived reconciliation state is shown." },
  targets: { eyebrow: "Verified balance sources", title: "Active tax accounts", count: (count) => `${count} account${count === 1 ? "" : "s"}`, notLoaded: "Balance evidence not loaded", notLoadedCopy: "Sign in to inspect active tax accounts and their latest finalized observations.", empty: "No balance sources configured", emptyCopy: "Opening balances require a separate approval process. An empty list is not evidence that funds reconcile.", account: "Account", observationSlot: "observation slot", expected: "Expected", observed: "observed" },
  detail: { eyebrow: "Immutable target identity", title: "Evidence detail", taxVault: "Tax vault", mintProgram: "Mint / token program", openingBalance: "Opening balance", atSlot: "at slot", openingReference: "Opening reference", receiptTax: "Finalized receipt tax", releases: "Finalized governed releases", staleAlert: "The latest finalized observation is stale and cannot prove the current vault balance. Stop release operations until a fresh observation is recorded.", mismatchAlert: "The latest finalized observation does not match the evidence-derived expectation. Stop release operations and follow the vault incident runbook.", errorAlert: "The latest observation could not be proven. Error code:", unknown: "unknown", history: "Append-only history", recentRuns: "Recent runs", loading: "Loading…", notLoaded: "Run history has not been loaded for this target." },
  status: { noObservation: "No observation yet", stale: "Stale — stop and investigate", matched: "Matched", mismatch: "Mismatch — stop and investigate", error: "Observation error — not matched", staleShort: "stale" },
  notice: { loaded: (count) => `Loaded ${count} verified balance source${count === 1 ? "" : "s"}.`, empty: "No active balance sources are configured for this account. This is not a matched state." },
  common: { unavailable: "The API is unavailable.", lookupError: "Vault reconciliation lookup failed; retry or contact an operator.", historyError: "Reconciliation history lookup failed; the target state is not being inferred.", httpError: (message, status) => `${message} (${displayHttpStatus(status)}).` },
};

const GERMAN: VaultReconciliationMessages = {
  control: { ariaLabel: "Arbeitsbereich für Tresorabgleich", eyebrow: "Kontrollen / nur Ansicht", title: "Steuerguthaben prüfen", badge: "Nur Ansicht", description: "Vergleichen Sie erfasste Steuerguthaben mit der letzten verifizierten Chain-Beobachtung. Dieser Arbeitsbereich kann Guthaben nicht ändern, Gelder nicht freigeben und keinen Alarm löschen.", loading: "Nachweise werden geladen…", load: "Abgleichsziele laden", unavailable: "Die API ist nicht verfügbar. Es wird kein zwischengespeicherter oder clientseitig abgeleiteter Abgleichstatus angezeigt." },
  targets: { eyebrow: "Verifizierte Guthabenquellen", title: "Aktive Steuerkonten", count: (count) => `${count} Konto${count === 1 ? "" : "en"}`, notLoaded: "Guthabennachweis nicht geladen", notLoadedCopy: "Melden Sie sich an, um aktive Steuerkonten und ihre neuesten endgültigen Beobachtungen zu prüfen.", empty: "Keine Guthabenquellen konfiguriert", emptyCopy: "Eröffnungsguthaben erfordern einen separaten Genehmigungsprozess. Eine leere Liste ist kein Nachweis eines erfolgreichen Abgleichs.", account: "Konto", observationSlot: "Beobachtungsslot", expected: "Erwartet", observed: "beobachtet" },
  detail: { eyebrow: "Unveränderliche Zielidentität", title: "Nachweisdetails", taxVault: "Steuertresor", mintProgram: "Mint / Tokenprogramm", openingBalance: "Eröffnungsguthaben", atSlot: "bei Slot", openingReference: "Eröffnungsreferenz", receiptTax: "Endgültige Steuer aus Belegen", releases: "Endgültige genehmigte Freigaben", staleAlert: "Die letzte endgültige Beobachtung ist veraltet und kann das aktuelle Tresorguthaben nicht beweisen. Stoppen Sie Freigaben, bis eine aktuelle Beobachtung vorliegt.", mismatchAlert: "Die letzte endgültige Beobachtung stimmt nicht mit der aus Nachweisen abgeleiteten Erwartung überein. Stoppen Sie Freigaben und befolgen Sie das Tresor-Incident-Runbook.", errorAlert: "Die letzte Beobachtung konnte nicht bewiesen werden. Fehlercode:", unknown: "unbekannt", history: "Nur anhängbare Historie", recentRuns: "Letzte Läufe", loading: "Wird geladen…", notLoaded: "Die Laufhistorie wurde für dieses Ziel noch nicht geladen." },
  status: { noObservation: "Noch keine Beobachtung", stale: "Veraltet — stoppen und prüfen", matched: "Abgeglichen", mismatch: "Abweichung — stoppen und prüfen", error: "Beobachtungsfehler — nicht abgeglichen", staleShort: "veraltet" },
  notice: { loaded: (count) => `${count} verifizierte Guthabenquelle${count === 1 ? "" : "n"} geladen.`, empty: "Für dieses Konto sind keine aktiven Guthabenquellen konfiguriert. Dies ist kein abgeglichener Status." },
  common: { unavailable: "Die API ist nicht verfügbar.", lookupError: "Der Tresorabgleich ist fehlgeschlagen; wiederholen Sie den Versuch oder kontaktieren Sie einen Bediener.", historyError: "Die Abgleichhistorie konnte nicht geladen werden; der Zielstatus wird nicht abgeleitet.", httpError: (message, status) => `${message} (${displayHttpStatus(status)}).` },
};

const ARABIC: VaultReconciliationMessages = {
  ...ENGLISH,
  control: { ...ENGLISH.control, ariaLabel: "مساحة مطابقة خزينة الضرائب", eyebrow: "فحوص الرصيد / عرض فقط", title: "فحص أرصدة الضرائب", badge: "عرض فقط", description: "قارن أرصدة الضرائب المسجلة مع أحدث ملاحظة سلسلة تم التحقق منها. لا يمكن لهذه المساحة تغيير الأرصدة أو تحرير الأموال أو مسح تنبيه.", loading: "جارٍ تحميل الدليل…", load: "تحميل أهداف المطابقة", unavailable: "واجهة API غير متاحة. لا يتم عرض حالة مطابقة مخزنة أو مشتقة من العميل." },
  targets: { ...ENGLISH.targets, eyebrow: "مصادر الرصيد التي تم التحقق منها", title: "حسابات الضرائب النشطة", count: (count) => `${count} حساب`, notLoaded: "لم يتم تحميل دليل الرصيد", notLoadedCopy: "سجّل الدخول لفحص حسابات الضرائب النشطة وأحدث ملاحظاتها النهائية.", empty: "لم يتم تكوين مصادر رصيد", emptyCopy: "تتطلب الأرصدة الافتتاحية عملية موافقة منفصلة. القائمة الفارغة ليست دليلاً على المطابقة.", account: "الحساب", observationSlot: "موضع الملاحظة", expected: "المتوقع", observed: "المرصود" },
  detail: { ...ENGLISH.detail, eyebrow: "هوية الهدف غير القابلة للتغيير", title: "تفاصيل الدليل", taxVault: "خزينة الضرائب", mintProgram: "الـ Mint / برنامج الرموز", openingBalance: "الرصيد الافتتاحي", atSlot: "في الموضع", openingReference: "مرجع الافتتاح", receiptTax: "ضريبة الإيصالات النهائية", releases: "التحريرات النهائية المعتمدة", staleAlert: "الملاحظة النهائية الأخيرة قديمة ولا تثبت رصيد الخزينة الحالي. أوقف عمليات التحرير حتى يتم تسجيل ملاحظة جديدة.", mismatchAlert: "لا تتطابق الملاحظة النهائية الأخيرة مع التوقع المشتق من الدليل. أوقف عمليات التحرير واتبع دليل حوادث الخزينة.", errorAlert: "تعذر إثبات الملاحظة الأخيرة. رمز الخطأ:", unknown: "غير معروف", history: "سجل ملحق فقط", recentRuns: "العمليات الأخيرة", loading: "جارٍ التحميل…", notLoaded: "لم يتم تحميل سجل العمليات لهذا الهدف." },
  status: { ...ENGLISH.status, noObservation: "لا توجد ملاحظة بعد", stale: "قديم — أوقف وتحقق", matched: "متطابق", mismatch: "عدم تطابق — أوقف وتحقق", error: "خطأ في الملاحظة — غير متطابق", staleShort: "قديم" },
  notice: { loaded: (count) => `تم تحميل ${count} من مصادر الرصيد التي تم التحقق منها.`, empty: "لا توجد مصادر رصيد نشطة مكوّنة لهذا الحساب. هذه ليست حالة تطابق." },
  common: { ...ENGLISH.common, unavailable: "واجهة API غير متاحة.", lookupError: "تعذرت مطابقة الخزينة؛ أعد المحاولة أو اتصل بالمشغل.", historyError: "تعذر تحميل سجل المطابقة؛ لم يتم استنتاج حالة الهدف.", httpError: (message, status) => `${message} (${displayHttpStatus(status)}).` },
};

const JAPANESE: VaultReconciliationMessages = { ...ENGLISH,
  control: { ...ENGLISH.control, ariaLabel: "保管庫照合ワークスペース", eyebrow: "残高確認 / 閲覧のみ", title: "税残高を確認", badge: "閲覧のみ", description: "記録された税残高と最新の検証済みチェーン観測を比較します。このワークスペースは残高変更、資金解放、アラート解除を行いません。", load: "照合対象を読み込む" },
  targets: { ...ENGLISH.targets, eyebrow: "検証済み残高ソース", title: "アクティブな税口座", notLoaded: "残高証拠が読み込まれていません" },
  detail: { ...ENGLISH.detail, eyebrow: "不変の対象識別子", title: "証拠の詳細", staleAlert: "最新の確定観測は古く、現在の保管庫残高を証明できません。新しい観測が記録されるまで解放操作を停止してください。" },
};

const CATALOG: Record<SupportedLocale, VaultReconciliationMessages> = { "en-US": ENGLISH, "de-DE": GERMAN, "ar-EG": ARABIC, "ja-JP": JAPANESE };

export function vaultReconciliationMessagesFor(locale: string): VaultReconciliationMessages {
  return CATALOG[locale as SupportedLocale] ?? ENGLISH;
}
