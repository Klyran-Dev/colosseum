import { operatorErrorMessage, OperatorFacingError } from "./operatorError";

const UUIDV7_PATTERN = /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

export type AuthErrorCode =
  | "ACCOUNT_SETUP_REQUIRED"
  | "ACCOUNT_EXISTS"
  | "AUTHENTICATION_FAILED"
  | "RATE_LIMITED"
  | "SELF_SIGNUP_DAILY_LIMIT_REACHED"
  | "AUTHENTICATION_UNAVAILABLE"
  | "UNKNOWN";

export type AuthRecoveryAction = "signup" | "signin" | null;

const AUTH_COPY: Record<AuthErrorCode, { message: string; recoveryAction: AuthRecoveryAction }> = {
  ACCOUNT_SETUP_REQUIRED: {
    message: "Your email was verified. Finish account setup to continue.",
    recoveryAction: "signup",
  },
  ACCOUNT_EXISTS: {
    message: "An account already exists. Continue to sign in.",
    recoveryAction: "signin",
  },
  AUTHENTICATION_FAILED: {
    message: "The code is invalid or expired. Request a new code.",
    recoveryAction: null,
  },
  RATE_LIMITED: {
    message: "Too many code requests. Try again later.",
    recoveryAction: null,
  },
  SELF_SIGNUP_DAILY_LIMIT_REACHED: {
    message: "New account creation is temporarily full for today. Try again tomorrow.",
    recoveryAction: null,
  },
  AUTHENTICATION_UNAVAILABLE: {
    message: "Authentication is temporarily unavailable.",
    recoveryAction: null,
  },
  UNKNOWN: {
    message: "The authentication request could not be completed.",
    recoveryAction: null,
  },
};

function payloadErrorCode(payload: unknown): AuthErrorCode {
  if (!payload || typeof payload !== "object") return "UNKNOWN";
  const value = (payload as Record<string, unknown>).error;
  if (typeof value !== "string") return "UNKNOWN";
  return Object.prototype.hasOwnProperty.call(AUTH_COPY, value) ? value as AuthErrorCode : "UNKNOWN";
}

export class AuthResponseError extends OperatorFacingError {
  readonly code: AuthErrorCode;
  readonly recoveryAction: AuthRecoveryAction;

  constructor(code: AuthErrorCode, supportReference: string | null = null) {
    const copy = AUTH_COPY[code];
    super(copy.message, supportReference);
    this.name = "AuthResponseError";
    this.code = code;
    this.recoveryAction = copy.recoveryAction;
  }
}

export function authResponseError(response: Response, payload: unknown, fallback: string): AuthResponseError {
  const code = payloadErrorCode(payload);
  if (code === "UNKNOWN") {
    const error = new AuthResponseError("UNKNOWN");
    error.message = fallback;
    return error;
  }
  const requestId = response.headers.get("x-request-id")?.trim() ?? "";
  return new AuthResponseError(code, UUIDV7_PATTERN.test(requestId) ? requestId : null);
}

export function authErrorMessage(cause: unknown, fallback: string): string {
  if (!(cause instanceof AuthResponseError)) return operatorErrorMessage(cause, fallback);
  return cause.supportReference ? `${cause.message} (support reference: ${cause.supportReference})` : cause.message;
}
