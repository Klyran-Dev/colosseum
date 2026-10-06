// @vitest-environment happy-dom

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { Keypair, Transaction } from "@solana/web3.js";
import { getWallets } from "@wallet-standard/app";
import { afterEach, describe, expect, it, vi } from "vitest";
import { SolanaSignTransaction } from "@solana/wallet-standard-features";
import { StandardConnect } from "@wallet-standard/features";
import type { Wallet } from "@wallet-standard/base";
import {
  connectWalletDescriptor,
  discoverWallets,
  ensureConnectedWallet,
  resetWalletSession,
  signWalletTransaction,
  type WalletRegistry,
} from "./wallet";
import WalletPicker from "./WalletPicker";

const registry = (wallets: readonly Wallet[]): WalletRegistry => ({ get: () => wallets });

afterEach(() => {
  cleanup();
  resetWalletSession();
  delete window.solana;
  vi.restoreAllMocks();
});

describe("Wallet Standard signing boundary", () => {
  it("filters registered wallets to Solana signTransaction capabilities", () => {
    const compatible = createStandardWallet("Compatible", Keypair.generate());
    const incompatible = createStandardWallet("No signer", Keypair.generate(), false);
    const nonSolana = createStandardWallet("Other chain", Keypair.generate(), true, ["eip155:1"]);

    expect(discoverWallets(registry([compatible, incompatible, nonSolana]))).toEqual([
      expect.objectContaining({ kind: "standard", name: "Compatible" }),
    ]);
  });

  it("requires an explicit descriptor and account when connecting a standard wallet", async () => {
    const first = Keypair.generate();
    const second = Keypair.generate();
    const wallet = createStandardWallet("Multi-account", first, true, ["solana:localnet"], [first, second]);
    const [descriptor] = discoverWallets(registry([wallet]));

    await expect(connectWalletDescriptor(descriptor)).rejects.toThrow(/account selection/i);
    const connected = await connectWalletDescriptor(descriptor, second.publicKey.toBase58());
    expect(connected.publicKey.toBase58()).toBe(second.publicKey.toBase58());
    expect(connected.descriptor.name).toBe("Multi-account");
  });

  it("signs serialized bytes through the standard feature and returns a verified transaction", async () => {
    const payer = Keypair.generate();
    const wallet = createStandardWallet("Signer", payer);
    const [descriptor] = discoverWallets(registry([wallet]));
    const connected = await connectWalletDescriptor(descriptor);
    const transaction = new Transaction({
      feePayer: payer.publicKey,
      recentBlockhash: Keypair.generate().publicKey.toBase58(),
    });

    const signed = await signWalletTransaction(connected, transaction, "http://127.0.0.1:8899");

    expect(signed.feePayer?.equals(payer.publicKey)).toBe(true);
    expect(signed.serialize({ requireAllSignatures: true, verifySignatures: true })).toBeInstanceOf(Uint8Array);
  });

  it("rejects malformed standard-wallet output instead of falling back silently", async () => {
    const payer = Keypair.generate();
    const wallet = createStandardWallet("Malformed", payer);
    const sign = wallet.features[SolanaSignTransaction] as { signTransaction: ReturnType<typeof vi.fn> };
    sign.signTransaction.mockResolvedValue([{ signedTransaction: Uint8Array.of(1, 2, 3) }]);
    const [descriptor] = discoverWallets(registry([wallet]));
    const connected = await connectWalletDescriptor(descriptor);
    const transaction = new Transaction({
      feePayer: payer.publicKey,
      recentBlockhash: Keypair.generate().publicKey.toBase58(),
    });

    await expect(signWalletTransaction(connected, transaction, "http://127.0.0.1:8899")).rejects.toThrow(/signed transaction/i);
  });

  it("retains the legacy adapter only when no standard wallet is registered", async () => {
    const payer = Keypair.generate();
    window.solana = {
      connect: vi.fn(async () => ({ publicKey: payer.publicKey })),
      signTransaction: vi.fn(async (transaction: Transaction) => {
        transaction.partialSign(payer);
        return transaction;
      }),
    };

    const [descriptor] = discoverWallets(registry([]));
    expect(descriptor.kind).toBe("legacy");
    const connected = await connectWalletDescriptor(descriptor);
    expect(connected.publicKey.toBase58()).toBe(payer.publicKey.toBase58());
  });

  it("renders an explicit provider selector when multiple standard wallets register", () => {
    const unregister = getWallets().register(
      createStandardWallet("Alpha", Keypair.generate()),
      createStandardWallet("Beta", Keypair.generate()),
    );
    render(<WalletPicker />);

    const picker = screen.getByRole("combobox", { name: "Wallet provider" });
    expect(screen.getByRole("option", { name: "Select a wallet provider" })).toBeTruthy();
    fireEvent.change(picker, { target: { value: "standard:0:Alpha" } });
    expect((picker as HTMLSelectElement).value).toBe("standard:0:Alpha");
    unregister();
  });

  it("renders an account selector after a multi-account provider requires explicit choice", async () => {
    const first = Keypair.generate();
    const second = Keypair.generate();
    const unregister = getWallets().register(
      createStandardWallet("Multi-account", first, true, ["solana:localnet"], [first, second]),
    );
    render(<WalletPicker />);

    fireEvent.click(screen.getByRole("button", { name: "Connect wallet" }));

    expect(await screen.findByRole("combobox", { name: "Wallet account" })).toBeTruthy();
    const accountPicker = screen.getByRole("combobox", { name: "Wallet account" }) as HTMLSelectElement;
    expect(accountPicker.value).toBe("");
    fireEvent.change(accountPicker, { target: { value: second.publicKey.toBase58() } });
    fireEvent.click(screen.getByRole("button", { name: "Connect selected account" }));

    await vi.waitFor(async () => {
      expect((await ensureConnectedWallet()).publicKey.toBase58()).toBe(second.publicKey.toBase58());
    });
    unregister();
  });

  it("redacts wallet provider exception details from the operator surface", async () => {
    window.solana = {
      connect: vi.fn(async () => { throw new Error("provider-internal rejection detail"); }),
      signTransaction: vi.fn(async () => { throw new Error("provider-internal signing detail"); }),
    };
    render(<WalletPicker />);

    fireEvent.click(screen.getByRole("button", { name: "Connect wallet" }));

    const alert = await screen.findByRole("alert");
    expect(alert.textContent).toContain("Wallet connection was rejected.");
    expect(alert.textContent).not.toContain("provider-internal rejection detail");
  });
});

function createStandardWallet(
  name: string,
  signer: Keypair,
  includeSigner = true,
  chains: readonly string[] = ["solana:localnet"],
  accounts: readonly Keypair[] = [signer],
): Wallet {
  const identifiers = chains as Wallet["chains"];
  const accountRecords = accounts.map((account) => ({
    address: account.publicKey.toBase58(),
    publicKey: account.publicKey.toBytes(),
    chains: identifiers,
    features: [SolanaSignTransaction],
  }));
  const features: Record<string, unknown> = {
    [StandardConnect]: {
      version: "1.0.0",
      connect: vi.fn(async () => ({ accounts: accountRecords })),
    },
  };
  if (includeSigner) {
    features[SolanaSignTransaction] = {
      version: "1.0.0",
      supportedTransactionVersions: ["legacy"],
      signTransaction: vi.fn(async ({ transaction }: { transaction: Uint8Array }) => {
        const parsed = Transaction.from(transaction);
        parsed.partialSign(signer);
        return [{ signedTransaction: Uint8Array.from(parsed.serialize({ requireAllSignatures: false, verifySignatures: false })) }];
      }),
    };
  }
  return {
    version: "1.0.0",
    name,
    icon: "data:image/png;base64,AA==",
    chains: identifiers,
    features,
    accounts: [],
  };
}
