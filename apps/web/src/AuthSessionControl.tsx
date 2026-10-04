import { BrowserAuthConfig } from "./authConfig";

export type AuthSessionStatus = "signed-out" | "signing-in" | "signed-in" | "error";

export interface AuthSessionControlProps {
  config: BrowserAuthConfig;
  accessToken: string;
  status: AuthSessionStatus;
  error?: string | null;
  onCredentialChange: (value: string) => void;
  onSignIn: () => void;
  onSignOut: () => void;
}

export default function AuthSessionControl({
  config,
  accessToken,
  status,
  error,
  onCredentialChange,
  onSignIn,
  onSignOut,
}: AuthSessionControlProps) {
  if (config.mode === "local-credential") {
    return <details className="session-control session-control-local" aria-label="Local sign-in">
      <summary role="button" aria-label="Open local sign-in">Local sign-in</summary>
      <div className="session-control-local-fields">
        <label>
          <span>Sign-in credential (held in memory)</span>
          <textarea value={accessToken} onChange={(event) => onCredentialChange(event.target.value)} placeholder="Paste a local test credential" autoComplete="off" spellCheck={false} rows={2} aria-describedby="local-sign-in-help" />
        </label>
        <small id="local-sign-in-help">Local development only. This field is unavailable in staging and production builds.</small>
        {error && <p className="inline-error" role="alert">{error}</p>}
      </div>
    </details>;
  }

  if (config.mode === "oidc") {
    if (status === "signed-in" && accessToken) {
      return <div className="session-control session-control-signed-in" aria-label="Signed-in session">
        <span className="session-state"><span className="status-dot" aria-hidden="true" />Signed in</span>
        <button type="button" className="session-action" onClick={onSignOut}>Sign out</button>
        {error && <p className="inline-error" role="alert">{error}</p>}
      </div>;
    }
    return <div className="session-control session-control-oidc" aria-label="Sign-in">
      <button type="button" className="session-action" onClick={onSignIn} disabled={status === "signing-in"}>{status === "signing-in" ? "Opening sign-in…" : "Sign in"}</button>
      {error && <p className="inline-error" role="alert">{error}</p>}
    </div>;
  }

  return <details className="session-control session-control-unconfigured" aria-label="Sign-in unavailable">
    <summary role="button" aria-label="Sign-in unavailable"><span className="session-state"><span className="status-dot" aria-hidden="true" />Sign-in unavailable</span></summary>
    <div className="session-control-unconfigured-details">
      <p className="inline-error" role="alert">{error ?? config.warnings.join(" ")}</p>
      <small>Ask your administrator to configure secure sign-in for this deployment.</small>
    </div>
  </details>;
}
