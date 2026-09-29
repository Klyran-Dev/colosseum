import { getWallets } from "@wallet-standard/app";
import type { Wallet, WalletAccount } from "@wallet-standard/base";
import { StandardConnect } from "@wallet-standard/features";
import { SolanaSignTransaction } from "@solana/wallet-standard-features";
import { PublicKey, Transaction } from "@solana/web3.js";

export const SOLANA_WALLET_CHAINS = [
  "solana:mainnet",
  "solana:devnet",
  "solana:testnet",
  "solana:localnet",
] as const;

export type LegacyInjectedWallet = {
  isConnected?: boolean;
  publicKey?: PublicKey | null;
  connect: () => Promise<{ publicKey: PublicKey }>;
  signTransaction: (transaction: Transaction) => Promise<Transaction>;
};

export type WalletRegistry = {
  get: () => readonly Wallet[];
};

export type WalletDescriptor = {
  id: string;
  kind: "standard" | "legacy";
  name: string;
  icon: string | null;
  chains: readonly string[];
  standardWallet?: Wallet;
  legacyWallet?: LegacyInjectedWallet;
};

export type ConnectedWallet = {
  descriptor: WalletDescriptor;
  readonly publicKey: PublicKey;
  connect: () => Promise<{ publicKey: PublicKey }>;
};

export class WalletAccountSelectionRequired extends Error {
  readonly walletId: string;
  readonly accounts: readonly string[];

  constructor(walletId: string, accounts: readonly string[]) {
    super("The selected wallet returned multiple Solana accounts; explicit account selection is required.");
    this.name = "WalletAccountSelectionRequired";
    this.walletId = walletId;
    this.accounts = accounts;
  }
}

declare global {
  interface Window {
    solana?: LegacyInjectedWallet;
  }
}

let activeWallet: ConnectedWallet | null = null;
let preferredWalletId: string | null = null;

function legacyWallet(): LegacyInjectedWallet | null {
  return typeof window !== "undefined" && window.solana ? window.solana : null;
}

function hasFunction(value: unknown, name: string): value is Record<string, (...args: never[]) => unknown> {
  return Boolean(value && typeof value === "object" && name in value && typeof (value as Record<string, unknown>)[name] === "function");
}

function supportsSolana(wallet: Wallet): boolean {
  return wallet.chains.some((chain) => SOLANA_WALLET_CHAINS.includes(chain as typeof SOLANA_WALLET_CHAINS[number]))
    && hasFunction(wallet.features[StandardConnect], "connect")
    && hasFunction(wallet.features[SolanaSignTransaction], "signTransaction");
}

function standardDescriptors(registry: WalletRegistry): WalletDescriptor[] {
  return registry.get().flatMap((wallet, index) => supportsSolana(wallet) ? [{
    id: `standard:${index}:${wallet.name}`,
    kind: "standard" as const,
    name: wallet.name,
    icon: wallet.icon,
    chains: wallet.chains.filter((chain) => SOLANA_WALLET_CHAINS.includes(chain as typeof SOLANA_WALLET_CHAINS[number])),
    standardWallet: wallet,
  }] : []);
}

export function discoverWallets(registry: WalletRegistry = getWallets()): WalletDescriptor[] {
  const standard = standardDescriptors(registry);
  if (standard.length > 0) return standard;
  const legacy = legacyWallet();
  return legacy ? [{
    id: "legacy:injected",
    kind: "legacy",
    name: "Injected Solana wallet",
    icon: null,
    chains: ["solana:legacy-injected"],
    legacyWallet: legacy,
  }] : [];
}

export function observeWalletDiscovery(listener: () => void): () => void {
  const wallets = getWallets();
  const offRegister = wallets.on("register", listener);
  const offUnregister = wallets.on("unregister", listener);
  return () => {
    offRegister();
    offUnregister();
  };
}

export function setPreferredWalletId(walletId: string | null): void {
  preferredWalletId = walletId;
  if (activeWallet && walletId !== activeWallet.descriptor.id) activeWallet = null;
}

function accountPublicKey(account: WalletAccount): PublicKey {
  let address: PublicKey;
  let bytes: PublicKey;
  try {
    address = new PublicKey(account.address);
    bytes = new PublicKey(account.publicKey);
  } catch {
    throw new Error("The selected wallet returned an invalid Solana account.");
  }
  if (!address.equals(bytes)) throw new Error("The selected wallet returned inconsistent account identity.");
  return address;
}

function supportedAccounts(accounts: readonly WalletAccount[], walletId: string): readonly { account: WalletAccount; address: string }[] {
  const supported = accounts
    .filter((account) => account.features.includes(SolanaSignTransaction))
    .map((account) => ({ account, address: accountPublicKey(account).toBase58() }));
  if (supported.length === 0) throw new Error("The selected wallet did not authorize a Solana transaction-signing account.");
  if (new Set(supported.map(({ address }) => address)).size !== supported.length) {
    throw new Error(`The selected wallet returned duplicate Solana accounts for ${walletId}.`);
  }
  return supported;
}

function selectAccount(accounts: readonly WalletAccount[], walletId: string, expectedAddress?: string): WalletAccount {
  const supported = supportedAccounts(accounts, walletId);
  if (expectedAddress) {
    let requested: PublicKey;
    try {
      requested = new PublicKey(expectedAddress);
    } catch {
      throw new Error("The requested Solana account is invalid.");
    }
    const selected = supported.find(({ address }) => new PublicKey(address).equals(requested));
    if (!selected) throw new Error("The selected wallet did not authorize the requested Solana account.");
    return selected.account;
  }
  if (supported.length !== 1) throw new WalletAccountSelectionRequired(walletId, supported.map(({ address }) => address));
  return supported[0].account;
}

function chainForRpc(chains: readonly string[], rpcUrl: string): string {
  const normalized = rpcUrl.toLowerCase();
  const expected = normalized.includes("localhost") || normalized.includes("127.0.0.1")
    ? "solana:localnet"
    : normalized.includes("devnet")
      ? "solana:devnet"
      : normalized.includes("testnet")
        ? "solana:testnet"
        : "solana:mainnet";
  if (!chains.includes(expected)) throw new Error(`The selected wallet does not support the configured Solana network (${expected}).`);
  return expected;
}

export async function connectWalletDescriptor(descriptor: WalletDescriptor, expectedAddress?: string): Promise<ConnectedWallet> {
  if (!descriptor) throw new Error("Select a compatible Solana wallet before connecting.");
  if (descriptor.kind === "legacy") {
    const wallet = descriptor.legacyWallet;
    if (!wallet) throw new Error("The injected Solana wallet is unavailable.");
    const response = await wallet.connect();
    if (!(response.publicKey instanceof PublicKey)) throw new Error("The injected wallet returned an invalid public key.");
    const connected: ConnectedWallet = {
      descriptor,
      publicKey: response.publicKey,
      connect: async () => {
        const next = await wallet.connect();
        if (!(next.publicKey instanceof PublicKey)) throw new Error("The injected wallet returned an invalid public key.");
        return next;
      },
    };
    activeWallet = connected;
    return connected;
  }

  const wallet = descriptor.standardWallet;
  if (!wallet) throw new Error("The selected Wallet Standard provider is unavailable.");
  const connectFeature = wallet.features[StandardConnect] as { connect?: (input?: { silent?: boolean }) => Promise<{ accounts: readonly WalletAccount[] }> } | undefined;
  if (!connectFeature?.connect) throw new Error("The selected wallet does not support standard connection.");
  let account = selectAccount((await connectFeature.connect({ silent: false })).accounts, descriptor.id, expectedAddress);
  const connected: ConnectedWallet = {
    descriptor,
    get publicKey() {
      return accountPublicKey(account);
    },
    connect: async () => {
      const result = await connectFeature.connect?.({ silent: false });
      if (!result) throw new Error("The selected wallet did not return a connection result.");
      account = selectAccount(result.accounts, descriptor.id, expectedAddress ?? account.address);
      return { publicKey: accountPublicKey(account) };
    },
  };
  activeWallet = connected;
  return connected;
}

export async function connectInjectedWallet(walletId?: string, accountAddress?: string): Promise<PublicKey> {
  const wallets = discoverWallets();
  if (wallets.length === 0) throw new Error("No compatible Solana wallet was detected.");
  const selectedWalletId = walletId ?? preferredWalletId;
  if (!selectedWalletId && wallets.length > 1) throw new Error("Multiple compatible Solana wallets were detected; select one before connecting.");
  const descriptor = wallets.find((candidate) => candidate.id === (selectedWalletId ?? wallets[0].id));
  if (!descriptor) throw new Error("The selected Solana wallet is no longer available.");
  return (await connectWalletDescriptor(descriptor, accountAddress)).publicKey;
}

export async function ensureConnectedWallet(): Promise<ConnectedWallet> {
  if (activeWallet?.descriptor.kind === "standard") return activeWallet;
  const wallets = discoverWallets();
  if (wallets.length === 0) throw new Error("No compatible Solana wallet was detected.");
  const descriptor = preferredWalletId ? wallets.find((wallet) => wallet.id === preferredWalletId) : undefined;
  if (wallets.length > 1 && !descriptor) throw new Error("Multiple compatible Solana wallets were detected; select one before signing.");
  return connectWalletDescriptor(descriptor ?? wallets[0]);
}

export function injectedWallet(): LegacyInjectedWallet | null {
  const legacy = legacyWallet();
  if (legacy) return legacy;
  return null;
}

export function resetWalletSession(): void {
  activeWallet = null;
  preferredWalletId = null;
}

export async function signWalletTransaction(wallet: ConnectedWallet, transaction: Transaction, rpcUrl: string): Promise<Transaction> {
  if (wallet.descriptor.kind === "legacy") {
    const legacy = wallet.descriptor.legacyWallet;
    if (!legacy) throw new Error("The injected Solana wallet is unavailable.");
    const signed = await legacy.signTransaction(transaction);
    if (!(signed instanceof Transaction)) throw new Error("The injected wallet returned an invalid signed transaction.");
    return signed;
  }
  if (!rpcUrl.trim()) throw new Error("A VITE_SOLANA_RPC_URL is required before wallet signing is enabled.");
  const standard = wallet.descriptor.standardWallet;
  const feature = standard?.features[SolanaSignTransaction] as { signTransaction?: (input: { account: WalletAccount; transaction: Uint8Array; chain: string }) => Promise<readonly { signedTransaction: Uint8Array }[]> } | undefined;
  if (!standard || !feature?.signTransaction) throw new Error("The selected wallet does not support Solana transaction signing.");
  const connectFeature = standard.features[StandardConnect] as { connect?: (input?: { silent?: boolean }) => Promise<{ accounts: readonly WalletAccount[] }> } | undefined;
  if (!connectFeature?.connect) throw new Error("The selected wallet does not support standard connection.");
  const accounts = (await connectFeature.connect({ silent: true })).accounts;
  const account = selectAccount(accounts, wallet.descriptor.id, wallet.publicKey.toBase58());
  const result = await feature.signTransaction({
    account,
    transaction: Uint8Array.from(transaction.serialize({ requireAllSignatures: false, verifySignatures: false })),
    chain: chainForRpc(wallet.descriptor.chains, rpcUrl),
  });
  if (!result || result.length !== 1 || !(result[0].signedTransaction instanceof Uint8Array)) {
    throw new Error("The standard wallet returned an invalid signed transaction.");
  }
  let signed: Transaction;
  try {
    signed = Transaction.from(result[0].signedTransaction);
  } catch {
    throw new Error("The standard wallet returned malformed signed transaction bytes.");
  }
  if (!signed.feePayer?.equals(wallet.publicKey)) throw new Error("The standard wallet changed the compiled fee payer.");
  const signature = signed.signatures.find((entry) => entry.publicKey.equals(wallet.publicKey));
  if (!signature?.signature) throw new Error("The standard wallet did not sign with the selected account.");
  try {
    signed.serialize({ requireAllSignatures: false, verifySignatures: true });
  } catch {
    throw new Error("The standard wallet returned an invalid account signature.");
  }
  return signed;
}
