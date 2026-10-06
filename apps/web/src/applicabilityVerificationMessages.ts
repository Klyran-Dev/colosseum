import { SupportedLocale } from "./formatters";

export type ApplicabilityVerificationMessages = {
  eyebrow: string;
  title: string;
  description: string;
  status: Record<"not_applicable" | "pending" | "retrying" | "verified" | "dead_lettered", string>;
  retry: string;
  unavailable: string;
  notFinality: string;
  verifiedCopy: string;
};

const ENGLISH: ApplicabilityVerificationMessages = {
  eyebrow: "Tax treatment evidence",
  title: "Chain evidence status",
  description: "This status reports finalized account evidence for the server-owned tax treatment. It does not approve, file, or move funds.",
  status: { not_applicable: "No applicability event", pending: "Waiting for verification", retrying: "Verification retrying", verified: "Verified on finalized ledger", dead_lettered: "Review required" },
  retry: "Refresh evidence status",
  unavailable: "Evidence status is temporarily unavailable. The payment remains unchanged.",
  notFinality: "Only the verified state contains finalized account evidence. Every other state is non-final.",
  verifiedCopy: "The binding account matches the settlement event, receipt, merchant, asset, context, and attestation digest at finalized commitment.",
};

const GERMAN: ApplicabilityVerificationMessages = { ...ENGLISH, eyebrow: "Nachweis der Steuerbehandlung", title: "Status des Ledger-Nachweises", description: "Dieser Status zeigt den finalisierten Kontonachweis für die serverseitige Steuerbehandlung. Er genehmigt, meldet oder bewegt keine Mittel.", status: { not_applicable: "Kein Applicability-Ereignis", pending: "Prüfung ausstehend", retrying: "Prüfung wird wiederholt", verified: "Auf finalisiertem Ledger geprüft", dead_lettered: "Prüfung erforderlich" }, retry: "Nachweisstatus aktualisieren", unavailable: "Der Nachweisstatus ist vorübergehend nicht verfügbar. Die Zahlung bleibt unverändert.", notFinality: "Nur der geprüfte Status enthält einen finalisierten Kontonachweis. Alle anderen Status sind nicht final.", verifiedCopy: "Das Binding-Konto stimmt bei finaler Bestätigung mit Ereignis, Beleg, Händler, Asset, Kontext und Attestierungs-Digest überein." };
const ARABIC: ApplicabilityVerificationMessages = { ...ENGLISH, eyebrow: "دليل المعالجة الضريبية", title: "حالة دليل السجل", description: "تعرض هذه الحالة دليل الحساب النهائي للمعالجة الضريبية المملوكة للخادم. لا تمنح موافقة أو تقدم إقراراً أو تحرك أموالاً.", status: { not_applicable: "لا يوجد حدث انطباق", pending: "بانتظار التحقق", retrying: "إعادة محاولة التحقق", verified: "تم التحقق في السجل النهائي", dead_lettered: "المراجعة مطلوبة" }, retry: "تحديث حالة الدليل", unavailable: "حالة الدليل غير متاحة مؤقتاً. لم تتغير الدفعة.", notFinality: "الحالة المتحققة فقط تحتوي على دليل حساب نهائي. الحالات الأخرى غير نهائية.", verifiedCopy: "يتطابق حساب الربط مع الحدث والإيصال والتاجر والأصل والسياق وبصمة التصديق عند الالتزام النهائي." };
const JAPANESE: ApplicabilityVerificationMessages = { ...ENGLISH, eyebrow: "税務処理の証拠", title: "台帳証拠の状態", description: "この状態は、サーバー管理の税務処理に対する確定済みアカウント証拠を示します。承認、申告、資金移動は行いません。", status: { not_applicable: "適用イベントなし", pending: "検証待ち", retrying: "検証を再試行中", verified: "確定台帳で検証済み", dead_lettered: "レビューが必要" }, retry: "証拠状態を更新", unavailable: "証拠状態を一時的に取得できません。支払いは変更されていません。", notFinality: "確定済みアカウント証拠を含むのは検証済み状態だけです。他の状態は確定ではありません。", verifiedCopy: "最終確定時に、バインディングアカウントが決済イベント、レシート、加盟店、アセット、コンテキスト、証明ダイジェストと一致します。" };

export function applicabilityVerificationMessagesFor(locale: SupportedLocale | string): ApplicabilityVerificationMessages {
  if (locale.toLowerCase().startsWith("de")) return GERMAN;
  if (locale.toLowerCase().startsWith("ar")) return ARABIC;
  if (locale.toLowerCase().startsWith("ja")) return JAPANESE;
  return ENGLISH;
}
