import { useEffect, useRef } from "react";
import { BrowserAuthConfig } from "./authConfig";
import { SupportedLocale } from "./formatters";
import { shellMessagesFor } from "./shellMessages";

export type AuthSessionStatus = "signed-out" | "signing-in" | "signed-in" | "error";

export interface AuthSessionControlProps {
  config: BrowserAuthConfig;
  accessToken: string;
  status: AuthSessionStatus;
  error?: string | null;
  onCredentialChange: (value: string) => void;
  email?: string;
  otpCode?: string;
  otpChallengeId?: string | null;
  onEmailChange?: (value: string) => void;
  onOtpChange?: (value: string) => void;
  onRequestOtp?: () => void;
  onVerifyOtp?: () => void;
  onResetOtp?: () => void;
  expanded?: boolean;
  onExpandedChange?: (expanded: boolean) => void;
  onSignIn: () => void;
  onOpenAuthPage?: (intent: "signin" | "signup") => void;
  locale?: SupportedLocale;
}

export default function AuthSessionControl({
  config,
  accessToken,
  status,
  error,
  onCredentialChange,
  email = "",
  otpCode = "",
  otpChallengeId = null,
  onEmailChange = () => undefined,
  onOtpChange = () => undefined,
  onRequestOtp = () => undefined,
  onVerifyOtp = () => undefined,
  onResetOtp = () => undefined,
  expanded = false,
  onExpandedChange = () => undefined,
  onSignIn,
  onOpenAuthPage,
  locale = "en-US",
}: AuthSessionControlProps) {
  const session = shellMessagesFor(locale).session;
  const localCredentialRef = useRef<HTMLTextAreaElement>(null);
  const emailRef = useRef<HTMLInputElement>(null);
  const otpRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (!expanded) return;
    if (config.mode === "local-credential") localCredentialRef.current?.focus();
    if (config.mode === "email-otp") (otpChallengeId ? otpRef : emailRef).current?.focus();
  }, [config.mode, expanded, otpChallengeId]);

  if (config.mode === "local-credential") {
    return <details className="session-control session-control-local" aria-label={session.localSignIn} open={expanded} onToggle={(event) => onExpandedChange(event.currentTarget.open)}>
      <summary role="button" aria-label={session.openLocalSignIn}>{session.localSignIn}</summary>
      <div className="session-control-local-fields">
        <label>
          <span>{session.signInCredential}</span>
        <textarea ref={localCredentialRef} value={accessToken} onChange={(event) => onCredentialChange(event.target.value)} placeholder={session.localCredentialPlaceholder} autoComplete="off" spellCheck={false} rows={2} aria-describedby="local-sign-in-help" />
        </label>
        <small id="local-sign-in-help">{session.localDevelopmentHelp}</small>
        {error && <p className="inline-error" role="alert">{error}</p>}
      </div>
    </details>;
  }

  if (config.mode === "oidc") {
    if (status === "signed-in" && accessToken) {
      return error ? <div className="session-control session-control-signed-in" aria-label={session.signedInSession}>
        <p className="inline-error" role="alert">{error}</p>
      </div> : null;
    }
    return <div className="session-control session-control-oidc" aria-label={session.signIn}>
      <button type="button" className="session-action" onClick={onSignIn} disabled={status === "signing-in"}>{status === "signing-in" ? session.openingSignIn : session.signIn}</button>
      {error && <p className="inline-error" role="alert">{error}</p>}
    </div>;
  }

  if (config.mode === "email-otp") {
    if (status === "signed-in" && accessToken) {
      return error ? <div className="session-control session-control-signed-in" aria-label={session.signedInSession}>
        <p className="inline-error" role="alert">{error}</p>
      </div> : null;
    }
    if (onOpenAuthPage) {
      return <div className="session-control session-control-email-otp" aria-label={session.emailOtpSignIn}>
        <button type="button" className="session-action" onClick={() => onOpenAuthPage("signin")}>{session.emailOtpSignIn}</button>
      </div>;
    }
    return <details className="session-control session-control-email-otp" aria-label={session.emailOtpSignIn} open={expanded} onToggle={(event) => onExpandedChange(event.currentTarget.open)}>
      <summary role="button" aria-label={session.openEmailOtpSignIn}>{session.emailOtpSignIn}</summary>
      <div className="session-control-email-otp-fields">
        {!otpChallengeId ? <>
          <label>
            <span>{session.emailLabel}</span>
            <input ref={emailRef} type="email" value={email} onChange={(event) => onEmailChange(event.target.value)} placeholder={session.emailPlaceholder} autoComplete="email" inputMode="email" />
          </label>
          <small>{session.emailOtpHelp}</small>
          <button type="button" className="session-action" onClick={onRequestOtp} disabled={status === "signing-in" || !email.trim()}>{status === "signing-in" ? session.sendingCode : session.sendCode}</button>
        </> : <>
          <p className="session-help">{session.emailOtpSent}</p>
          <label>
            <span>{session.codeLabel}</span>
            <input ref={otpRef} type="text" value={otpCode} onChange={(event) => onOtpChange(event.target.value)} placeholder={session.codePlaceholder} autoComplete="one-time-code" inputMode="numeric" maxLength={6} />
          </label>
          <div className="session-control-email-otp-actions">
            <button type="button" className="session-action" onClick={onVerifyOtp} disabled={status === "signing-in" || otpCode.trim().length !== 6}>{status === "signing-in" ? session.verifyingCode : session.verifyCode}</button>
            <button type="button" className="session-action session-action-secondary" onClick={onResetOtp}>{session.changeEmail}</button>
          </div>
        </>}
        {error && <p className="inline-error" role="alert">{error}</p>}
      </div>
    </details>;
  }

  return <details className="session-control session-control-unconfigured" aria-label={session.signInSetupNeeded} open={expanded} onToggle={(event) => onExpandedChange(event.currentTarget.open)}>
    <summary role="button" aria-label={session.signInSetupNeededSummary}><span className="session-state"><span className="status-dot" aria-hidden="true" />{session.signInSetupNeeded}</span></summary>
    <div className="session-control-unconfigured-details">
      <p className="inline-error" role="alert">{error ?? config.warnings.join(" ")}</p>
      <small>{session.secureSignInHelp}</small>
    </div>
  </details>;
}
