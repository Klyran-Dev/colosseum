import { useEffect, useState } from "react";
import { PublicKey } from "@solana/web3.js";
import {
  connectInjectedWallet,
  discoverWallets,
  observeWalletDiscovery,
  setPreferredWalletId,
  WalletAccountSelectionRequired,
  type WalletDescriptor,
} from "./wallet";
import { DEFAULT_LOCALE } from "./formatters";
import { walletMessagesFor } from "./walletMessages";

type WalletPickerProps = {
  disabled?: boolean;
  connectEnabled?: boolean;
  connectedAddress?: string | null;
  onConnected?: (publicKey: PublicKey) => void;
  locale?: string;
};

export default function WalletPicker({ disabled = false, connectEnabled = true, connectedAddress = null, onConnected, locale = DEFAULT_LOCALE }: WalletPickerProps) {
  const messages = walletMessagesFor(locale);
  const [wallets, setWallets] = useState<WalletDescriptor[]>(() => discoverWallets());
  const [selectedWalletId, setSelectedWalletId] = useState<string>(() => {
    const initial = discoverWallets();
    return initial.length === 1 ? initial[0].id : "";
  });
  const [accountOptions, setAccountOptions] = useState<readonly string[]>([]);
  const [selectedAccount, setSelectedAccount] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const refresh = () => {
      const next = discoverWallets();
      setWallets(next);
      setSelectedWalletId((current) => current && next.some((wallet) => wallet.id === current)
        ? current
        : next.length === 1 ? next[0].id : "");
      setAccountOptions([]);
      setSelectedAccount("");
    };
    refresh();
    return observeWalletDiscovery(refresh);
  }, []);

  if (wallets.length === 0) {
    return <p className="wallet-picker-status" role="status">{messages.noWallet}</p>;
  }

  async function connect() {
    setError(null);
    setBusy(true);
    try {
      const walletId = selectedWalletId || (wallets.length === 1 ? wallets[0].id : undefined);
      const publicKey = await connectInjectedWallet(walletId, selectedAccount || undefined);
      if (walletId) setPreferredWalletId(walletId);
      setAccountOptions([]);
      setSelectedAccount("");
      onConnected?.(publicKey);
    } catch (cause) {
      if (cause instanceof WalletAccountSelectionRequired) {
        setSelectedWalletId(cause.walletId);
        setPreferredWalletId(cause.walletId);
        setAccountOptions(cause.accounts);
        setSelectedAccount("");
        setError(messages.multipleAccounts);
      } else {
        setError(messages.connectionRejected);
      }
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="wallet-picker">
      {wallets.length === 1
        ? <p className="wallet-picker-status" role="status">{messages.providerPrefix} <strong>{wallets[0].name}</strong></p>
        : <label>
          <span>{messages.providerLabel}</span>
          <select
            aria-label={messages.providerLabel}
            value={selectedWalletId}
            disabled={disabled || busy}
            onChange={(event) => {
              const value = event.target.value;
              setSelectedWalletId(value);
              setPreferredWalletId(value || null);
              setAccountOptions([]);
              setSelectedAccount("");
              setError(null);
            }}
          >
            <option value="">{messages.providerPlaceholder}</option>
            {wallets.map((wallet) => <option value={wallet.id} key={wallet.id}>{wallet.name}</option>)}
          </select>
        </label>}
      {accountOptions.length > 0 && <label>
        <span>{messages.accountLabel}</span>
        <select
          aria-label={messages.accountAriaLabel}
          value={selectedAccount}
          disabled={disabled || busy}
          onChange={(event) => setSelectedAccount(event.target.value)}
        >
          <option value="">{messages.accountPlaceholder}</option>
          {accountOptions.map((account) => <option value={account} key={account}>{account}</option>)}
        </select>
      </label>}
      {error && <p className="wallet-picker-error" role="alert">{error}</p>}
      {connectedAddress && <p className="wallet-picker-account" role="status">{messages.selectedAccount} <code>{connectedAddress}</code></p>}
      <button
        type="button"
        onClick={connect}
        disabled={disabled || busy || !connectEnabled || (wallets.length > 1 && !selectedWalletId) || (accountOptions.length > 0 && !selectedAccount)}
      >
        {busy ? messages.connecting : accountOptions.length > 0 ? messages.connectSelected : connectEnabled ? connectedAddress ? messages.reconnect : messages.connect : messages.connectAfterArtifact}
      </button>
    </div>
  );
}
