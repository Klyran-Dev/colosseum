export type BrowserAuthMode = "oidc" | "email-otp" | "local-credential" | "unconfigured";

export interface OidcBrowserConfig {
  issuer: string;
  clientId: string;
  redirectUri: string;
  scope: string;
  audience?: string;
}

export interface BrowserAuthConfig {
  mode: BrowserAuthMode;
  oidc?: OidcBrowserConfig;
  emailOtp?: { baseUrl: string };
  warnings: readonly string[];
}

export function parseAuthServiceBaseUrl(raw: Readonly<Record<string, unknown>>): string {
  const environment = value(raw, "VITE_DEPLOYMENT_ENVIRONMENT");
  const candidate = value(raw, "VITE_AUTH_SERVICE_BASE_URL");
  if (!candidate || !isSafeUrl(candidate, environment, false, true)) return "";
  return candidate.replace(/\/$/u, "");
}

const MAX_CLIENT_ID_LENGTH = 256;
const MAX_SCOPE_LENGTH = 512;
const MAX_AUDIENCE_LENGTH = 256;
const OIDC_KEYS = [
  "VITE_OIDC_ISSUER",
  "VITE_OIDC_CLIENT_ID",
  "VITE_OIDC_REDIRECT_URI",
  "VITE_OIDC_SCOPE",
  "VITE_OIDC_AUDIENCE",
] as const;
const LOCAL_AUTH_SERVICE_HOSTS = new Set(["localhost", "127.0.0.1", "[::1]", "auth-service"]);

function value(raw: Readonly<Record<string, unknown>>, key: string): string {
  const candidate = raw[key];
  return typeof candidate === "string" ? candidate.trim() : "";
}

function isSafeUrl(candidate: string, environment: string, requireOrigin: boolean, allowLocalAuthService = false): boolean {
  try {
    const parsed = new URL(candidate);
    if (parsed.username || parsed.password || parsed.hash || (!requireOrigin && parsed.search)) return false;
    if (requireOrigin && !parsed.origin) return false;
    if (environment !== "local" && parsed.protocol !== "https:") return false;
    if (environment === "local" && parsed.protocol !== "https:") {
      return parsed.protocol === "http:" && (allowLocalAuthService ? LOCAL_AUTH_SERVICE_HOSTS : new Set(["localhost", "127.0.0.1", "[::1]"])).has(parsed.hostname);
    }
    return parsed.protocol === "https:" || parsed.protocol === "http:";
  } catch {
    return false;
  }
}

export function parseBrowserAuthConfig(raw: Readonly<Record<string, unknown>>): BrowserAuthConfig {
  const environment = value(raw, "VITE_DEPLOYMENT_ENVIRONMENT");
  const authMethod = value(raw, "VITE_AUTH_METHOD");
  const localUiMode = value(raw, "VITE_LOCAL_UI_MODE");
  const localCredentialEntry = localUiMode === "credential" || value(raw, "VITE_LOCAL_CREDENTIAL_ENTRY") === "true";
  const explicitEmailOtpEntry = authMethod === "email-otp";
  const explicitOidcEntry = authMethod === "oidc";
  const legacyEmailOtpEntry = localUiMode === "email-otp";
  const emailOtpEntry = explicitEmailOtpEntry || legacyEmailOtpEntry;
  const hasOidcInput = OIDC_KEYS.some((key) => value(raw, key) !== "");

  if (authMethod && authMethod !== "oidc" && authMethod !== "email-otp") {
    return { mode: "unconfigured", warnings: ["VITE_AUTH_METHOD must be oidc or email-otp."] };
  }

  if (explicitEmailOtpEntry && localCredentialEntry) {
    return { mode: "unconfigured", warnings: ["Email OTP sign-in cannot be combined with local credential entry."] };
  }
  if (explicitOidcEntry && (localCredentialEntry || legacyEmailOtpEntry)) {
    return { mode: "unconfigured", warnings: ["OIDC sign-in cannot be combined with local credential or email OTP entry."] };
  }

  if (localCredentialEntry) {
    if (environment !== "local") {
      return { mode: "unconfigured", warnings: ["Local credential entry is permitted only in a local build."] };
    }
    if (hasOidcInput) {
      return { mode: "unconfigured", warnings: ["Local credential entry cannot be combined with OIDC sign-in configuration."] };
    }
    return { mode: "local-credential", warnings: [] };
  }

  if (emailOtpEntry) {
    const baseUrl = value(raw, "VITE_AUTH_SERVICE_BASE_URL");
    if (!explicitEmailOtpEntry && environment !== "local") {
      return { mode: "unconfigured", warnings: ["Email OTP self-sign-in is permitted only in a local build until a delivery provider and provisioning policy are approved."] };
    }
    if (hasOidcInput) {
      return { mode: "unconfigured", warnings: ["Email OTP sign-in cannot be combined with OIDC sign-in configuration."] };
    }
    if (!baseUrl || !isSafeUrl(baseUrl, environment, false, environment === "local")) {
      return {
        mode: "unconfigured",
        warnings: [environment === "local"
          ? "The local authentication service URL must use loopback HTTP or the bounded Compose auth-service hostname, without query or fragment data."
          : "The email OTP service URL must use HTTPS outside local development."],
      };
    }
    return { mode: "email-otp", emailOtp: { baseUrl: baseUrl.replace(/\/$/u, "") }, warnings: [] };
  }

  if (explicitOidcEntry && !hasOidcInput) {
    return { mode: "unconfigured", warnings: ["The selected OIDC authentication method is missing its provider configuration."] };
  }

  if (!hasOidcInput) {
    return { mode: "unconfigured", warnings: ["No browser sign-in configuration was supplied by the release build."] };
  }

  const issuer = value(raw, "VITE_OIDC_ISSUER");
  const clientId = value(raw, "VITE_OIDC_CLIENT_ID");
  const redirectUri = value(raw, "VITE_OIDC_REDIRECT_URI");
  const scope = value(raw, "VITE_OIDC_SCOPE");
  const audience = value(raw, "VITE_OIDC_AUDIENCE");
  const warnings: string[] = [];

  if (!issuer) warnings.push("The OIDC issuer is required.");
  if (!clientId || clientId.length > MAX_CLIENT_ID_LENGTH) warnings.push("The OIDC client ID is missing or too long.");
  if (!redirectUri) warnings.push("The OIDC redirect URI is required.");
  if (!scope || scope.length > MAX_SCOPE_LENGTH || !scope.split(/\s+/u).includes("openid")) warnings.push("The OIDC scope must include openid.");
  if (audience.length > MAX_AUDIENCE_LENGTH) warnings.push("The OIDC audience is too long.");
  if (issuer && !isSafeUrl(issuer, environment, false)) {
    warnings.push(environment === "local" ? "The local OIDC issuer must use HTTPS or loopback HTTP." : "The OIDC issuer must use HTTPS outside local development.");
  }
  if (redirectUri && !isSafeUrl(redirectUri, environment, true)) {
    warnings.push(environment === "local" ? "The local OIDC redirect URI must use HTTPS or loopback HTTP." : "The OIDC redirect URI must use HTTPS outside local development.");
  }
  if (warnings.length > 0) return { mode: "unconfigured", warnings };

  return {
    mode: "oidc",
    oidc: { issuer, clientId, redirectUri, scope, ...(audience ? { audience } : {}) },
    warnings: [],
  };
}

export const BROWSER_AUTH_CONFIG = parseBrowserAuthConfig({
  VITE_DEPLOYMENT_ENVIRONMENT: import.meta.env.VITE_DEPLOYMENT_ENVIRONMENT ?? (import.meta.env.MODE === "test" ? "local" : undefined),
  VITE_OIDC_ISSUER: import.meta.env.VITE_OIDC_ISSUER,
  VITE_OIDC_CLIENT_ID: import.meta.env.VITE_OIDC_CLIENT_ID,
  VITE_OIDC_REDIRECT_URI: import.meta.env.VITE_OIDC_REDIRECT_URI,
  VITE_OIDC_SCOPE: import.meta.env.VITE_OIDC_SCOPE,
  VITE_OIDC_AUDIENCE: import.meta.env.VITE_OIDC_AUDIENCE,
  VITE_LOCAL_UI_MODE: import.meta.env.VITE_LOCAL_UI_MODE ?? (import.meta.env.MODE === "test" ? "credential" : "disabled"),
  VITE_LOCAL_CREDENTIAL_ENTRY: import.meta.env.VITE_LOCAL_CREDENTIAL_ENTRY,
  VITE_AUTH_SERVICE_BASE_URL: import.meta.env.VITE_AUTH_SERVICE_BASE_URL,
});

export const BROWSER_AUTH_SERVICE_BASE_URL = parseAuthServiceBaseUrl({
  VITE_DEPLOYMENT_ENVIRONMENT: import.meta.env.VITE_DEPLOYMENT_ENVIRONMENT ?? (import.meta.env.MODE === "test" ? "local" : undefined),
  VITE_AUTH_SERVICE_BASE_URL: import.meta.env.VITE_AUTH_SERVICE_BASE_URL,
});
