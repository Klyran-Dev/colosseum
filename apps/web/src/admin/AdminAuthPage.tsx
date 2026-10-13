import { FormEvent, useEffect, useMemo, useState } from "react";
import { Link, useNavigate } from "@tanstack/react-router";
import { beginOidcAuthorization, completeOidcCallback } from "../oidcSession";
import { fetchWithTimeout, REQUEST_TIMEOUT_MS } from "../api";
import { BROWSER_AUTH_CONFIG, BROWSER_AUTH_SERVICE_BASE_URL } from "../authConfig";
import { invitationIdFromSearch, safeAuthNextPath } from "../authFlow";
import { authErrorMessage, authResponseError, AuthRecoveryAction, AuthResponseError } from "../authErrors";
import { useAdminSession } from "./session";

type AuthIntent = "signin" | "signup";
const ADMIN_TARGETS = ["/admin", "/admin/tax/quotes", "/admin/tax/rules", "/admin/settlements", "/admin/treasury", "/admin/access", "/admin/evidence", "/admin/settings"] as const;
type AdminTarget = typeof ADMIN_TARGETS[number];

function targetPath(value: string): AdminTarget {
  const pathname = value.split("?", 1)[0];
  return (ADMIN_TARGETS as readonly string[]).includes(pathname) ? pathname as AdminTarget : "/admin";
}

function AppLogo() {
  return (
    <div className="flex items-center gap-3">
      <span className="grid size-10 place-items-center rounded-xl bg-admin-primary text-lg font-black text-white shadow-soft">S</span>
      <span className="leading-tight">
        <strong className="block text-sm font-extrabold tracking-tight text-admin-ink">Tax operations</strong>
        <small className="block text-xs text-admin-muted">Workspace access</small>
      </span>
    </div>
  );
}

function errorMessage(value: unknown): string {
  if (value && typeof value === "object" && "message" in value && typeof value.message === "string") return value.message;
  return value instanceof Error ? value.message : "The authentication service could not complete this request.";
}

async function requestAuthJson(path: string, fallback: string, init: RequestInit = {}): Promise<unknown> {
  const headers = new Headers(init.headers);
  headers.set("accept", "application/json");
  const response = await fetchWithTimeout(path, { ...init, headers }, REQUEST_TIMEOUT_MS.default);
  const payload: unknown = await response.json().catch(() => null);
  if (!response.ok) throw authResponseError(response, payload, fallback);
  return payload;
}

export default function AdminAuthPage({ intent }: { intent: AuthIntent }) {
  const { setSession } = useAdminSession();
  const navigate = useNavigate();
  const [email, setEmail] = useState("");
  const [code, setCode] = useState("");
  const [challengeId, setChallengeId] = useState("");
  const [challengeExpiresAtUnix, setChallengeExpiresAtUnix] = useState<number | null>(null);
  const [resendAvailableAtUnix, setResendAvailableAtUnix] = useState(0);
  const [nowUnix, setNowUnix] = useState(() => Math.floor(Date.now() / 1000));
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [recoveryAction, setRecoveryAction] = useState<AuthRecoveryAction>(null);
  const authBase = BROWSER_AUTH_SERVICE_BASE_URL || BROWSER_AUTH_CONFIG.emailOtp?.baseUrl || "";
  const emailOtpEnabled = BROWSER_AUTH_CONFIG.mode === "email-otp" && Boolean(authBase);
  const oidcEnabled = BROWSER_AUTH_CONFIG.mode === "oidc" && Boolean(BROWSER_AUTH_CONFIG.oidc);
  const nextPath = useMemo(() => targetPath(safeAuthNextPath(new URLSearchParams(window.location.search).get("next"))), []);
  const invitationId = intent === "signup" ? invitationIdFromSearch(window.location.search) : null;

  useEffect(() => {
    const callback = new URLSearchParams(window.location.search);
    if (!oidcEnabled || (!callback.has("code") && !callback.has("error")) || !BROWSER_AUTH_CONFIG.oidc) return;
    let active = true;
    setBusy(true);
    completeOidcCallback(BROWSER_AUTH_CONFIG.oidc, new URL(window.location.href), window.sessionStorage)
      .then((token) => {
        if (!active) return;
        setSession({ accessToken: token.accessToken, email: "Verified operator", expiresAtUnix: token.expiresAtUnix });
        return navigate({ to: nextPath });
      })
      .catch((cause: unknown) => { if (active) setError(errorMessage(cause)); })
      .finally(() => { if (active) setBusy(false); });
    return () => { active = false; };
  }, [navigate, nextPath, oidcEnabled, setSession]);

  useEffect(() => {
    if (!challengeId) return;
    const timer = window.setInterval(() => setNowUnix(Math.floor(Date.now() / 1000)), 1_000);
    return () => window.clearInterval(timer);
  }, [challengeId]);

  function responseNumber(payload: unknown, key: string): number | null {
    if (!payload || typeof payload !== "object") return null;
    const candidate = (payload as Record<string, unknown>)[key];
    return typeof candidate === "number" && Number.isSafeInteger(candidate) && candidate >= 0 ? candidate : null;
  }

  async function requestChallenge(): Promise<void> {
    if (!emailOtpEnabled) throw new Error(BROWSER_AUTH_CONFIG.warnings[0] ?? "Sign-in is not configured for this deployment.");
    const endpoint = (operation: string) => `${authBase}/v1/auth/email/${operation}`;
    const payload = await requestAuthJson(endpoint("request"), "The sign-in code could not be sent.", { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify({ email, intent, ...(invitationId ? { invitation_id: invitationId } : {}) }) });
    const value = payload && typeof payload === "object" ? payload as Record<string, unknown> : {};
    const id = typeof value.challenge_id === "string" && value.challenge_id ? value.challenge_id : "";
    const expiresAtUnix = responseNumber(payload, "expires_at_unix");
    if (!id || expiresAtUnix === null || expiresAtUnix <= Math.floor(Date.now() / 1000)) throw new Error("The authentication service returned an invalid challenge.");
    const retryAfterSeconds = responseNumber(payload, "retry_after_seconds") ?? 0;
    setChallengeId(id);
    setChallengeExpiresAtUnix(expiresAtUnix);
    setResendAvailableAtUnix(Math.floor(Date.now() / 1000) + retryAfterSeconds);
    setCode("");
    setRecoveryAction(null);
  }

  async function resendChallenge(): Promise<void> {
    if (busy || !email.trim() || nowUnix < resendAvailableAtUnix) return;
    setBusy(true);
    setError(null);
    setRecoveryAction(null);
    try {
      await requestChallenge();
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setBusy(false);
    }
  }

  function resetChallenge(): void {
    setChallengeId("");
    setChallengeExpiresAtUnix(null);
    setResendAvailableAtUnix(0);
    setCode("");
    setError(null);
    setRecoveryAction(null);
  }

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setBusy(true);
    setError(null);
    setRecoveryAction(null);
    try {
      if (oidcEnabled && BROWSER_AUTH_CONFIG.oidc) {
        const authorizationUrl = await beginOidcAuthorization(BROWSER_AUTH_CONFIG.oidc, window.sessionStorage);
        window.location.assign(authorizationUrl);
        return;
      }
      if (!emailOtpEnabled) throw new Error(BROWSER_AUTH_CONFIG.warnings[0] ?? "Sign-in is not configured for this deployment.");
      if (!challengeId) {
        await requestChallenge();
      } else {
        const endpoint = (operation: string) => `${authBase}/v1/auth/email/${operation}`;
        const payload = await requestAuthJson(endpoint("verify"), "The sign-in code could not be verified.", { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify({ email, code, challenge_id: challengeId }) });
        const value = payload && typeof payload === "object" ? payload as Record<string, unknown> : {};
        if (typeof value.access_token !== "string" || !value.access_token || typeof value.expires_in !== "number" || !Number.isSafeInteger(value.expires_in) || value.expires_in <= 0) throw new Error("The authentication service returned an invalid session.");
        setSession({ accessToken: value.access_token, email, expiresAtUnix: Math.floor(Date.now() / 1000) + value.expires_in });
        await navigate({ to: nextPath });
      }
    } catch (cause) {
      const authError = cause instanceof AuthResponseError ? cause : null;
      setRecoveryAction(authError?.recoveryAction ?? null);
      if (authError?.recoveryAction) {
        setChallengeId("");
        setChallengeExpiresAtUnix(null);
        setResendAvailableAtUnix(0);
        setCode("");
      }
      setError(authErrorMessage(cause, "The sign-in code could not be completed."));
    } finally {
      setBusy(false);
    }
  }

  const configured = emailOtpEnabled || oidcEnabled;
  return (
    <main className="min-h-screen bg-admin-bg px-3 py-3 sm:px-5 sm:py-5 lg:px-10 lg:py-8">
      <div className="admin-auth-shell mx-auto grid max-w-5xl overflow-hidden rounded-2xl border border-admin-line bg-admin-surface shadow-panel lg:grid-cols-[1.08fr_.92fr] lg:rounded-3xl">
        <section className="flex flex-col justify-between p-5 sm:p-8 lg:p-10">
          <div>
            <Link to="/admin" aria-label="Back to admin overview"><AppLogo /></Link>
            <div className="mt-9 max-w-lg sm:mt-12 lg:mt-16">
              <p className="text-xs font-black uppercase tracking-[0.16em] text-admin-primary">{intent === "signin" ? "Welcome back" : "Get started"}</p>
              <h1 className="admin-auth-title mt-4 font-extrabold text-admin-ink">{intent === "signin" ? "Sign in to your workspace" : "Create your workspace account"}</h1>
              <p className="mt-4 text-[0.9375rem] leading-6 text-admin-muted">{intent === "signin" ? "Enter your work email and we’ll send you a one-time code." : "Create your account with your work email. An administrator can invite teammates and grant additional access later."}</p>
            </div>
            <form className="mt-6 max-w-md space-y-3.5" onSubmit={submit}>
              {!configured && <div className="rounded-xl border border-admin-amber/30 bg-admin-amber-soft p-3 text-[0.8125rem] leading-5 text-admin-amber-ink" role="alert"><strong className="block">Sign-in is not configured</strong><span>{BROWSER_AUTH_CONFIG.warnings.join(" ") || "A supported identity provider must be configured for this deployment."}</span></div>}
              {emailOtpEnabled && <>
                <label className="block" htmlFor="admin-auth-email"><span className="mb-1.5 block text-[0.8125rem] font-bold text-admin-ink">Work email</span><input id="admin-auth-email" className="w-full rounded-xl border border-admin-line bg-admin-surface-muted px-4 py-2.5 text-sm text-admin-ink outline-none transition placeholder:text-admin-muted focus:border-admin-primary focus:bg-admin-surface focus:ring-4 focus:ring-admin-primary/10" type="email" value={email} onChange={(event) => setEmail(event.target.value)} autoComplete="email" placeholder="you@company.com" required /></label>
                {challengeId && <label className="block" htmlFor="admin-auth-code"><span className="mb-1.5 block text-[0.8125rem] font-bold text-admin-ink">One-time code</span><input id="admin-auth-code" className="w-full rounded-xl border border-admin-line bg-admin-surface-muted px-4 py-2.5 font-mono text-sm tracking-[0.3em] text-admin-ink outline-none transition placeholder:text-admin-muted focus:border-admin-primary focus:bg-admin-surface focus:ring-4 focus:ring-admin-primary/10" inputMode="numeric" autoComplete="one-time-code" value={code} onChange={(event) => setCode(event.target.value.replace(/\D/gu, "").slice(0, 6))} placeholder="000000" required /></label>}
              </>}
              {error && <div className="rounded-xl border border-admin-red/30 bg-admin-red-soft p-3.5 text-[0.8125rem] leading-5 text-admin-red" role="alert"><p>{error}</p>{recoveryAction === "signup" && <a href={`/auth/signup?next=${encodeURIComponent(nextPath)}`} className="mt-3 inline-flex rounded-lg bg-admin-primary px-3 py-2 text-xs font-bold text-white">Finish account setup</a>}{recoveryAction === "signin" && <a href={`/auth/signin?next=${encodeURIComponent(nextPath)}`} className="mt-3 inline-flex rounded-lg bg-admin-primary px-3 py-2 text-xs font-bold text-white">Continue to sign in</a>}</div>}
              {challengeId && <div className="rounded-xl border border-admin-line bg-admin-surface-muted p-3 text-xs leading-5 text-admin-muted" role="status" aria-live="polite">
                <p>{challengeExpiresAtUnix !== null && challengeExpiresAtUnix <= nowUnix ? "This code has expired. Request a new code to continue." : "Use the newest code sent to this email address."}</p>
                <div className="mt-3 flex flex-wrap items-center gap-3">
                  <button type="button" onClick={() => void resendChallenge()} disabled={busy || !email.trim() || nowUnix < resendAvailableAtUnix} className="rounded-lg border border-admin-line bg-admin-surface px-3 py-1.5 text-xs font-bold text-admin-ink disabled:cursor-not-allowed disabled:opacity-50">{busy ? "Sending new code…" : nowUnix < resendAvailableAtUnix ? `Resend code in ${resendAvailableAtUnix - nowUnix}s` : "Resend code"}</button>
                  <button type="button" onClick={resetChallenge} disabled={busy} className="text-xs font-bold text-admin-primary disabled:cursor-not-allowed disabled:opacity-50">Use a different email</button>
                </div>
              </div>}
              <button type="submit" disabled={busy || !configured} className="w-full rounded-xl bg-admin-primary px-5 py-2.5 text-[0.8125rem] font-bold text-white shadow-soft transition hover:bg-admin-primary-strong disabled:cursor-not-allowed disabled:opacity-60">{busy ? "Preparing your sign-in…" : oidcEnabled ? "Continue with identity provider" : challengeId ? "Verify code" : intent === "signin" ? "Continue with email" : "Create account"}</button>
              <p className="text-xs leading-5 text-admin-muted">An administrator controls which workspace areas are available after you sign in.</p>
            </form>
          </div>
          <div className="mt-8 text-sm text-admin-muted">{intent === "signin" ? <span>Don’t have an account yet? <Link to="/auth/signup" className="font-bold text-admin-primary">Create an account</Link></span> : <span>Already have an account? <Link to="/auth/signin" className="font-bold text-admin-primary">Back to sign in</Link></span>}</div>
        </section>
        <aside className="hidden bg-admin-primary-soft p-8 lg:flex lg:flex-col lg:justify-center">
          <p className="text-xs font-black uppercase tracking-[0.16em] text-admin-primary">Your secure workspace</p>
          <h2 className="mt-4 max-w-sm text-3xl font-extrabold tracking-[-0.04em] text-admin-ink">A simple, secure sign-in.</h2>
          <p className="mt-4 max-w-md text-[0.9375rem] leading-6 text-admin-muted">We’ll confirm your email before opening your workspace.</p>
          <div className="mt-10 space-y-5">{[["01", "Check your email", "We’ll send a one-time code to your inbox."], ["02", "Enter your code", "Use the newest code to continue."], ["03", "You’re in", "Your workspace opens with the access your administrator provides."]].map(([number, title, detail]) => <div key={number} className="flex gap-4 border-t border-admin-primary/20 pt-5"><span className="font-mono text-sm font-bold text-admin-primary">{number}</span><span><strong className="block text-sm text-admin-ink">{title}</strong><small className="mt-1 block text-sm leading-5 text-admin-muted">{detail}</small></span></div>)}</div>
        </aside>
      </div>
    </main>
  );
}
