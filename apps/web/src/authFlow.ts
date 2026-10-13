export type AuthIntent = "signin" | "signup";

const FALLBACK_NEXT_PATH = "/operations/tax-simulator";
export const AUTH_RETURN_PATH_STORAGE_KEY = "solana-tax.auth.return-path.v1";
const INVITATION_ID_PATTERN = /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/iu;

export function isAuthPath(pathname: string): boolean {
  return pathname === "/auth/signin" || pathname === "/auth/signup";
}

export function authIntentFromPath(pathname: string): AuthIntent {
  return pathname === "/auth/signup" ? "signup" : "signin";
}

export function safeAuthNextPath(value: string | null | undefined): string {
  if (!value || !value.startsWith("/") || value.startsWith("//")) return FALLBACK_NEXT_PATH;
  if (/^[\u0000-\u001f\u007f]/u.test(value) || value.includes("#")) return FALLBACK_NEXT_PATH;
  if (value === "/auth/signin" || value === "/auth/signup") return FALLBACK_NEXT_PATH;
  return value;
}

export function safeInvitationId(value: string | null | undefined): string | null {
  if (!value || !INVITATION_ID_PATTERN.test(value)) return null;
  return value.toLowerCase();
}

export function invitationIdFromSearch(search: string): string | null {
  try {
    return safeInvitationId(new URLSearchParams(search).get("invite"));
  } catch {
    return null;
  }
}

export function storeAuthReturnPath(storage: Storage, value: string): void {
  try {
    storage.setItem(AUTH_RETURN_PATH_STORAGE_KEY, safeAuthNextPath(value));
  } catch {
    throw new Error("The browser could not stage the sign-in return path safely.");
  }
}

export function consumeAuthReturnPath(storage: Storage): string {
  let value: string | null;
  try {
    value = storage.getItem(AUTH_RETURN_PATH_STORAGE_KEY);
    storage.removeItem(AUTH_RETURN_PATH_STORAGE_KEY);
  } catch {
    throw new Error("The browser could not read the sign-in return path safely.");
  }
  return safeAuthNextPath(value);
}

export function authPathForIntent(intent: AuthIntent, nextPath = FALLBACK_NEXT_PATH, invitationId: string | null = null): string {
  const params = new URLSearchParams({ next: safeAuthNextPath(nextPath) });
  const safeInvite = intent === "signup" ? safeInvitationId(invitationId) : null;
  if (safeInvite) params.set("invite", safeInvite);
  return `/auth/${intent === "signup" ? "signup" : "signin"}?${params.toString()}`;
}

