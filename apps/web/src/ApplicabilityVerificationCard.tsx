import { useCallback, useEffect, useState } from "react";
import { fetchWithTimeout, operatorResponseError, REQUEST_TIMEOUT_MS } from "./api";
import { ApplicabilityVerification, buildApplicabilityVerificationUrl, parseApplicabilityVerification } from "./applicabilityVerification";
import { applicabilityVerificationMessagesFor } from "./applicabilityVerificationMessages";
import { operatorErrorMessage } from "./operatorError";
import { SupportedLocale } from "./formatters";

type Props = { apiBase: string; accessToken: string; serviceOnline: boolean; intentId: string; streamKey: string; locale: SupportedLocale | string; refreshKey?: string };

export default function ApplicabilityVerificationCard({ apiBase, accessToken, serviceOnline, intentId, streamKey, locale, refreshKey = "" }: Props) {
  const messages = applicabilityVerificationMessagesFor(locale);
  const [verification, setVerification] = useState<ApplicabilityVerification | null>(null);
  const [state, setState] = useState<"idle" | "loading" | "ready" | "error">("idle");
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async (signal?: AbortSignal) => {
    if (!intentId || !serviceOnline || !accessToken.trim() || !streamKey.trim()) {
      setVerification(null);
      setState("idle");
      return;
    }
    setState("loading");
    setError(null);
    try {
      const response = await fetchWithTimeout(buildApplicabilityVerificationUrl(apiBase, intentId, streamKey), { headers: { authorization: `Bearer ${accessToken.trim()}` }, signal }, REQUEST_TIMEOUT_MS.default);
      const payload: unknown = await response.json().catch(() => null);
      if (!response.ok) throw operatorResponseError(response, payload, messages.unavailable);
      setVerification(parseApplicabilityVerification(payload));
      setState("ready");
    } catch (cause) {
      if (cause instanceof DOMException && cause.name === "AbortError") return;
      setState("error");
      setError(operatorErrorMessage(cause, messages.unavailable));
    }
  }, [accessToken, apiBase, intentId, messages.unavailable, serviceOnline, streamKey]);

  useEffect(() => {
    const controller = new AbortController();
    void load(controller.signal);
    return () => controller.abort();
  }, [load, refreshKey]);

  if (!intentId || !streamKey.trim()) return null;
  const status = verification?.status ?? (state === "loading" ? "pending" : null);
  return <section className={`route-evidence applicability-evidence state-${status ?? "unknown"}`} aria-labelledby="applicability-verification-title" aria-live="polite">
    <div className="panel-heading"><div><p className="eyebrow">{messages.eyebrow}</p><h3 id="applicability-verification-title">{messages.title}</h3></div>{status && <span className="status-pill">{messages.status[status]}</span>}</div>
    <p>{messages.description}</p>
    {verification?.status === "verified" ? <p className="inline-success">{messages.verifiedCopy}</p> : <p className="field-help">{messages.notFinality}</p>}
    {verification?.attempts !== null && verification?.attempts !== undefined && <small>Attempts: {verification.attempts}{verification.last_error_code ? ` · ${verification.last_error_code}` : ""}</small>}
    {verification?.signature && <small>Signature <code>{verification.signature}</code></small>}
    {error && <p className="inline-error" role="alert">{error}</p>}
    <button type="button" className="secondary-button" onClick={() => void load()} disabled={state === "loading"}>{messages.retry}</button>
  </section>;
}
