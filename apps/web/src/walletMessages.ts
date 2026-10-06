import { SupportedLocale } from "./formatters";

export interface WalletMessages {
  noWallet: string;
  providerPrefix: string;
  providerLabel: string;
  providerPlaceholder: string;
  accountLabel: string;
  accountAriaLabel: string;
  accountPlaceholder: string;
  multipleAccounts: string;
  connectionRejected: string;
  selectedAccount: string;
  connecting: string;
  connectSelected: string;
  reconnect: string;
  connect: string;
  connectAfterArtifact: string;
}

const ENGLISH: WalletMessages = {
  noWallet: "No compatible Solana wallet is currently available.",
  providerPrefix: "Wallet provider:",
  providerLabel: "Wallet provider",
  providerPlaceholder: "Select a wallet provider",
  accountLabel: "Solana account",
  accountAriaLabel: "Wallet account",
  accountPlaceholder: "Select an account",
  multipleAccounts: "This wallet has multiple Solana accounts. Select the account that should authorize the server-compiled transaction.",
  connectionRejected: "Wallet connection was rejected.",
  selectedAccount: "Selected account:",
  connecting: "Connecting…",
  connectSelected: "Connect selected account",
  reconnect: "Reconnect wallet",
  connect: "Connect wallet",
  connectAfterArtifact: "Connect after artifact review",
};

const GERMAN: WalletMessages = {
  noWallet: "Derzeit ist kein kompatibles Solana-Wallet verfügbar.",
  providerPrefix: "Wallet-Anbieter:",
  providerLabel: "Wallet-Anbieter",
  providerPlaceholder: "Wallet-Anbieter auswählen",
  accountLabel: "Solana-Konto",
  accountAriaLabel: "Wallet-Konto",
  accountPlaceholder: "Konto auswählen",
  multipleAccounts: "Dieses Wallet hat mehrere Solana-Konten. Wählen Sie das Konto aus, das die serverseitig kompilierte Transaktion autorisieren soll.",
  connectionRejected: "Die Wallet-Verbindung wurde abgelehnt.",
  selectedAccount: "Ausgewähltes Konto:",
  connecting: "Verbindung wird hergestellt…",
  connectSelected: "Ausgewähltes Konto verbinden",
  reconnect: "Wallet erneut verbinden",
  connect: "Wallet verbinden",
  connectAfterArtifact: "Nach Prüfung des Artefakts verbinden",
};

const ARABIC: WalletMessages = {
  noWallet: "لا تتوفر حاليًا محفظة Solana متوافقة.",
  providerPrefix: "موفّر المحفظة:",
  providerLabel: "موفّر المحفظة",
  providerPlaceholder: "اختر موفّر محفظة",
  accountLabel: "حساب Solana",
  accountAriaLabel: "حساب المحفظة",
  accountPlaceholder: "اختر حسابًا",
  multipleAccounts: "تحتوي هذه المحفظة على عدة حسابات Solana. اختر الحساب الذي سيصرّح بالمعاملة التي أعدّها الخادم.",
  connectionRejected: "تم رفض اتصال المحفظة.",
  selectedAccount: "الحساب المحدد:",
  connecting: "جارٍ الاتصال…",
  connectSelected: "اتصل بالحساب المحدد",
  reconnect: "إعادة اتصال المحفظة",
  connect: "اتصال بالمحفظة",
  connectAfterArtifact: "اتصل بعد مراجعة الأثر",
};

const JAPANESE: WalletMessages = {
  noWallet: "現在、対応する Solana ウォレットはありません。",
  providerPrefix: "ウォレットプロバイダー:",
  providerLabel: "ウォレットプロバイダー",
  providerPlaceholder: "ウォレットプロバイダーを選択",
  accountLabel: "Solana アカウント",
  accountAriaLabel: "ウォレットアカウント",
  accountPlaceholder: "アカウントを選択",
  multipleAccounts: "このウォレットには複数の Solana アカウントがあります。サーバーがコンパイルしたトランザクションを承認するアカウントを選択してください。",
  connectionRejected: "ウォレット接続が拒否されました。",
  selectedAccount: "選択したアカウント:",
  connecting: "接続中…",
  connectSelected: "選択したアカウントを接続",
  reconnect: "ウォレットを再接続",
  connect: "ウォレットを接続",
  connectAfterArtifact: "アーティファクト確認後に接続",
};

const CATALOGS: Record<SupportedLocale, WalletMessages> = {
  "en-US": ENGLISH,
  "de-DE": GERMAN,
  "ar-EG": ARABIC,
  "ja-JP": JAPANESE,
};

export function walletMessagesFor(locale: string): WalletMessages {
  return CATALOGS[locale as SupportedLocale] ?? ENGLISH;
}
