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
  onSignIn: () => void;
  onSignOut: () => void;
  locale?: SupportedLocale;
}

export default function AuthSessionControl({
  config,
  accessToken,
  status,
  error,
  onCredentialChange,
  onSignIn,
  onSignOut,
  locale = "en-US",
}: AuthSessionControlProps) {
  const session = shellMessagesFor(locale).session;

  if (config.mode === "local-credential") {
    return <details className="session-control session-control-local" aria-label={session.localSignIn}>
      <summary role="button" aria-label={session.openLocalSignIn}>{session.localSignIn}</summary>
      <div className="session-control-local-fields">
        <label>
          <span>{session.signInCredential}</span>
          <textarea value={accessToken} onChange={(event) => onCredentialChange(event.target.value)} placeholder={session.localCredentialPlaceholder} autoComplete="off" spellCheck={false} rows={2} aria-describedby="local-sign-in-help" />
        </label>
        <small id="local-sign-in-help">{session.localDevelopmentHelp}</small>
        {error && <p className="inline-error" role="alert">{error}</p>}
      </div>
    </details>;
  }

  if (config.mode === "oidc") {
    if (status === "signed-in" && accessToken) {
      return <div className="session-control session-control-signed-in" aria-label={session.signedInSession}>
        <span className="session-state"><span className="status-dot" aria-hidden="true" />{session.signedIn}</span>
        <button type="button" className="session-action" onClick={onSignOut}>{session.signOut}</button>
        {error && <p className="inline-error" role="alert">{error}</p>}
      </div>;
    }
    return <div className="session-control session-control-oidc" aria-label={session.signIn}>
      <button type="button" className="session-action" onClick={onSignIn} disabled={status === "signing-in"}>{status === "signing-in" ? session.openingSignIn : session.signIn}</button>
      {error && <p className="inline-error" role="alert">{error}</p>}
    </div>;
  }

  return <details className="session-control session-control-unconfigured" aria-label={session.signInSetupNeeded}>
    <summary role="button" aria-label={session.signInSetupNeededSummary}><span className="session-state"><span className="status-dot" aria-hidden="true" />{session.signInSetupNeeded}</span></summary>
    <div className="session-control-unconfigured-details">
      <p className="inline-error" role="alert">{error ?? config.warnings.join(" ")}</p>
      <small>{session.secureSignInHelp}</small>
    </div>
  </details>;
}
