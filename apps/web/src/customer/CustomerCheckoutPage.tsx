import { useEffect, useMemo, useState } from "react";
import { parseCompiledSettlementArtifact, parseSettlementStatusResponse, signAndEncodeSettlement, type CompiledSettlementArtifact } from "../checkout";
import WalletPicker from "../WalletPicker";
import { fetchWithTimeout, operatorResponseError, REQUEST_TIMEOUT_MS } from "../api";
import { formatAtomic, parseQuoteResult } from "../admin/TaxQuotesPage";
import type { PublicKey } from "@solana/web3.js";

type QuoteResult = ReturnType<typeof parseQuoteResult>;

type PublicCheckout = {
  sessionId: string;
  calculationId: string;
  routeId: string;
  expiresAtUnix: number;
  quote: QuoteResult;
  route: { id: string; routeKey: string; version: number; mint: string; mintDecimals: number; tokenProgram: string };
  settlementIntentId: string | null;
};

function objectValue(value: unknown, message: string): Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error(message);
  return value as Record<string, unknown>;
}

function requiredString(value: unknown, label: string): string {
  if (typeof value !== "string" || value.trim() === "") throw new Error(`The checkout service returned an invalid ${label}.`);
  return value;
}

function uuidV7(value: unknown, label: string): string {
  const normalized = requiredString(value, label).toLowerCase();
  if (!/^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/u.test(normalized)) throw new Error(`The checkout service returned an invalid ${label}.`);
  return normalized;
}

function parsePublicCheckout(value: unknown): PublicCheckout {
  const source = objectValue(value, "The checkout service returned an invalid session.");
  if (source.schema !== "public-checkout-session/v1") throw new Error("The checkout service returned an unsupported session schema.");
  const quote = parseQuoteResult(source.quote);
  const route = objectValue(source.route, "The checkout service returned an invalid settlement route.");
  const sessionId = uuidV7(source.session_id, "session ID");
  const calculationId = uuidV7(source.calculation_id, "calculation ID");
  const routeId = uuidV7(source.route_id, "route ID");
  if (quote.calculationId !== calculationId || quote.expiresAtUnix <= Math.floor(Date.now() / 1000)) throw new Error("The quote is expired or is not bound to this checkout session.");
  const expiresAtUnix = source.expires_at_unix;
  if (typeof expiresAtUnix !== "number" || !Number.isSafeInteger(expiresAtUnix) || expiresAtUnix <= 0 || expiresAtUnix > quote.expiresAtUnix) throw new Error("The checkout service returned an invalid session expiry.");
  const version = route.version;
  const mintDecimals = route.mint_decimals;
  if (typeof version !== "number" || !Number.isSafeInteger(version) || version <= 0 || typeof mintDecimals !== "number" || !Number.isSafeInteger(mintDecimals) || mintDecimals < 0 || mintDecimals > 9) throw new Error("The checkout service returned an invalid route version or precision.");
  const routeDisplay = {
    id: uuidV7(route.id, "route ID"),
    routeKey: requiredString(route.route_key, "route key"),
    version,
    mint: requiredString(route.mint, "asset mint"),
    mintDecimals,
    tokenProgram: requiredString(route.token_program, "token program"),
  };
  if (routeDisplay.id !== routeId) throw new Error("The checkout service returned a route outside this checkout session.");
  return {
    sessionId,
    calculationId,
    routeId,
    expiresAtUnix,
    quote,
    route: routeDisplay,
    settlementIntentId: source.settlement_intent_id === null ? null : uuidV7(source.settlement_intent_id, "settlement intent ID"),
  };
}

async function publicJson(path: string, token: string, init: RequestInit = {}): Promise<unknown> {
  const response = await fetchWithTimeout(path, { ...init, headers: { accept: "application/json", ...(init.headers ?? {}), "x-checkout-token": token } }, REQUEST_TIMEOUT_MS.default);
  const payload = await response.json().catch(() => null);
  if (!response.ok) throw operatorResponseError(response, payload, `The checkout service returned HTTP ${response.status}.`);
  return payload;
}

function tokenFromHash(): string {
  const fragment = window.location.hash.replace(/^#/u, "");
  const params = new URLSearchParams(fragment);
  return params.get("token")?.trim() ?? "";
}

function newNonce(): string {
  const bytes = new Uint32Array(2);
  crypto.getRandomValues(bytes);
  return ((BigInt(bytes[0]) << 32n) | BigInt(bytes[1])).toString();
}

export default function CustomerCheckoutPage({ sessionId }: { sessionId: string }) {
  const [token] = useState(tokenFromHash);
  const [checkout, setCheckout] = useState<PublicCheckout | null>(null);
  const [walletAddress, setWalletAddress] = useState<PublicKey | null>(null);
  const [nonce, setNonce] = useState(newNonce);
  const [artifact, setArtifact] = useState<CompiledSettlementArtifact | null>(null);
  const [status, setStatus] = useState<ReturnType<typeof parseSettlementStatusResponse> | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!token) {
      setError("This checkout link is incomplete. Ask the merchant to create a new link.");
      return;
    }
    let cancelled = false;
    publicJson(`/api/v1/public/checkout/sessions/${encodeURIComponent(sessionId)}`, token)
      .then((payload) => { if (!cancelled) setCheckout(parsePublicCheckout(payload)); })
      .catch((cause) => { if (!cancelled) setError(cause instanceof Error ? cause.message : "The checkout session could not be loaded."); });
    return () => { cancelled = true; };
  }, [sessionId, token]);

  const expired = Boolean(checkout && checkout.expiresAtUnix <= Math.floor(Date.now() / 1000));
  const displayStatus = status?.status ?? (checkout?.settlementIntentId ? "prepared_unsubmitted" : "not_started");
  const total = useMemo(() => checkout ? formatAtomic(checkout.quote.result.grossAtomic, checkout.quote.result.atomicDecimals, checkout.quote.result.currency) : "—", [checkout]);

  async function compile() {
    if (!checkout || !walletAddress) { setError("Connect a wallet before preparing payment."); return; }
    if (expired) { setError("This quote has expired. Ask the merchant for a new checkout link."); return; }
    setBusy(true); setError(null);
    try {
      const payload = await publicJson(`/api/v1/public/checkout/sessions/${encodeURIComponent(checkout.sessionId)}/compile`, token, { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify({ payer: walletAddress.toBase58(), nonce }) });
      const next = parseCompiledSettlementArtifact(payload);
      if (!next.intent_id) throw new Error("The checkout service did not return a durable settlement intent.");
      setArtifact(next); setStatus(null);
    } catch (cause) { setError(cause instanceof Error ? cause.message : "Payment preparation failed."); }
    finally { setBusy(false); }
  }

  async function signAndSubmit() {
    if (!checkout || !artifact || !artifact.intent_id || !walletAddress) { setError("Prepare the payment and connect a wallet before signing."); return; }
    setBusy(true); setError(null);
    try {
      const signed = await signAndEncodeSettlement(artifact, walletAddress, import.meta.env.VITE_SOLANA_RPC_URL ?? "");
      const payload = await publicJson(`/api/v1/public/checkout/sessions/${encodeURIComponent(checkout.sessionId)}/submit`, token, { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify({ intent_id: artifact.intent_id, signed_transaction_base64: signed, instruction: artifact }) });
      setStatus(parseSettlementStatusResponse(payload));
    } catch (cause) { setError(cause instanceof Error ? cause.message : "Wallet signing or submission failed."); }
    finally { setBusy(false); }
  }

  async function refreshStatus() {
    if (!checkout || !artifact?.intent_id) return;
    setBusy(true); setError(null);
    try {
      const payload = await publicJson(`/api/v1/public/checkout/sessions/${encodeURIComponent(checkout.sessionId)}/intent/${encodeURIComponent(artifact.intent_id)}`, token);
      setStatus(parseSettlementStatusResponse(payload));
    } catch (cause) { setError(cause instanceof Error ? cause.message : "Settlement status could not be refreshed."); }
    finally { setBusy(false); }
  }

  return <main className="min-h-screen bg-admin-bg px-5 py-10 text-admin-ink lg:px-8"><div className="mx-auto max-w-3xl"><header className="mb-7"><p className="text-xs font-black uppercase tracking-[0.16em] text-admin-primary">Secure customer checkout</p><h1 className="mt-2 text-3xl font-black tracking-[-0.04em] sm:text-4xl">Review and authorize payment</h1><p className="mt-3 max-w-2xl text-sm leading-6 text-admin-muted">This page shows a server-owned tax quote. Your wallet remains the only signer; the checkout service never receives your private key.</p></header>{error && <div className="mb-6 rounded-2xl border border-admin-red/30 bg-admin-red-soft p-4 text-sm leading-6 text-admin-red" role="alert">{error}</div>}{!checkout && !error && <div className="rounded-2xl border border-admin-line bg-admin-surface p-8 text-sm text-admin-muted" role="status">Loading verified checkout…</div>}{checkout && <div className="space-y-6"><section className="rounded-2xl border border-admin-line bg-admin-surface p-6 shadow-soft"><div className="flex flex-wrap items-start justify-between gap-4"><div><p className="text-xs font-black uppercase tracking-[0.14em] text-admin-primary">{checkout.route.routeKey} · v{checkout.route.version}</p><h2 className="mt-1 text-2xl font-black text-admin-ink">Payment summary</h2></div><span className={`rounded-full px-3 py-1 text-xs font-bold ${expired ? "bg-admin-red-soft text-admin-red" : "bg-admin-green-soft text-admin-green"}`}>{expired ? "Quote expired" : "Quote verified"}</span></div><div className="mt-6 grid gap-3 sm:grid-cols-3"><div className="rounded-xl bg-admin-surface-muted p-4"><span className="block text-xs font-bold uppercase tracking-[0.1em] text-admin-muted">Gross</span><strong className="mt-2 block text-xl font-black">{total}</strong></div><div className="rounded-xl bg-admin-surface-muted p-4"><span className="block text-xs font-bold uppercase tracking-[0.1em] text-admin-muted">Tax</span><strong className="mt-2 block text-xl font-black">{formatAtomic(checkout.quote.result.totalTaxAtomic, checkout.quote.result.atomicDecimals, checkout.quote.result.currency)}</strong></div><div className="rounded-xl bg-admin-surface-muted p-4"><span className="block text-xs font-bold uppercase tracking-[0.1em] text-admin-muted">Net</span><strong className="mt-2 block text-xl font-black">{formatAtomic(checkout.quote.result.totalNetAtomic, checkout.quote.result.atomicDecimals, checkout.quote.result.currency)}</strong></div></div><dl className="mt-6 grid gap-3 text-sm sm:grid-cols-2"><div><dt className="text-admin-muted">Asset mint</dt><dd className="mt-1 break-all font-mono text-xs">{checkout.route.mint}</dd></div><div><dt className="text-admin-muted">Quote expires</dt><dd className="mt-1">{new Date(checkout.expiresAtUnix * 1000).toLocaleString()}</dd></div><div><dt className="text-admin-muted">Rule</dt><dd className="mt-1">{checkout.quote.result.ruleSetId} v{checkout.quote.result.ruleVersion}</dd></div><div><dt className="text-admin-muted">Reference</dt><dd className="mt-1 break-all font-mono text-xs">{checkout.calculationId}</dd></div></dl></section><section className="rounded-2xl border border-admin-line bg-admin-surface p-6 shadow-soft"><h2 className="text-xl font-black">Connect your wallet</h2><p className="mt-2 text-sm leading-6 text-admin-muted">Confirm the receiving network and exact amount in your wallet before approving. The transaction is still pending until the Solana provider returns a status.</p><div className="mt-5"><WalletPicker disabled={busy || expired} connectedAddress={walletAddress?.toBase58()} onConnected={setWalletAddress} /></div><div className="mt-5 grid gap-4 sm:grid-cols-[1fr_auto]"><label className="block"><span className="mb-2 block text-sm font-bold">Customer nonce</span><input className="w-full rounded-xl border border-admin-line bg-admin-bg px-4 py-3 font-mono text-sm" value={nonce} onChange={(event) => setNonce(event.target.value)} disabled={busy || Boolean(artifact)} inputMode="numeric" aria-describedby="customer-nonce-help" /><small id="customer-nonce-help" className="mt-1 block text-xs text-admin-muted">A unique canonical decimal replay-protection value for this checkout.</small></label><button type="button" onClick={() => { setNonce(newNonce()); setArtifact(null); setStatus(null); }} disabled={busy || Boolean(artifact)} className="self-end rounded-xl border border-admin-line px-4 py-3 text-sm font-bold disabled:opacity-50">New nonce</button></div>{artifact && <div className="mt-5 rounded-xl border border-admin-green/30 bg-admin-green-soft p-4 text-sm"><strong className="block">Transaction prepared</strong><p className="mt-1 text-admin-muted">Intent {artifact.intent_id} is bound to this checkout session. Review the wallet prompt before signing.</p></div>}<div className="mt-5 flex flex-wrap gap-3"><button type="button" onClick={() => void compile()} disabled={busy || expired || !walletAddress || Boolean(artifact)} className="rounded-xl bg-admin-primary px-5 py-3 text-sm font-bold text-white disabled:cursor-not-allowed disabled:opacity-50">{busy && !artifact ? "Preparing…" : "Prepare payment"}</button><button type="button" onClick={() => void signAndSubmit()} disabled={busy || expired || !artifact || !walletAddress || Boolean(status)} className="rounded-xl bg-admin-green px-5 py-3 text-sm font-bold text-white disabled:cursor-not-allowed disabled:opacity-50">{busy && artifact && !status ? "Waiting for wallet…" : "Sign and submit"}</button>{status && <button type="button" onClick={() => void refreshStatus()} disabled={busy} className="rounded-xl border border-admin-line px-5 py-3 text-sm font-bold disabled:opacity-50">Refresh status</button>}</div></section>{status && <section className="rounded-2xl border border-admin-line bg-admin-surface p-6 shadow-soft" aria-live="polite"><p className="text-xs font-black uppercase tracking-[0.14em] text-admin-primary">Chain status</p><h2 className="mt-1 text-xl font-black">{status.status}</h2><p className="mt-2 text-sm leading-6 text-admin-muted">A submission status is not the same as finalized chain evidence. Refresh this panel until the reconciler reports the authoritative lifecycle state.</p>{status.signature && <code className="mt-4 block break-all rounded-xl bg-admin-surface-muted p-3 text-xs">Signature {status.signature}</code>}</section>}</div>}</div></main>;
}

export { parsePublicCheckout };
