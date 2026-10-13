import { describe, expect, it } from "vitest";
import { authErrorMessage, authResponseError } from "./authErrors";

function response(status = 409, requestId = ""): Response {
  return new Response(null, {
    status,
    headers: requestId ? { "x-request-id": requestId } : undefined,
  });
}

describe("email OTP authentication error contract", () => {
  it("maps the bootstrap outcome to an explicit signup recovery action", () => {
    const error = authResponseError(
      response(),
      { error: "ACCOUNT_SETUP_REQUIRED", message: "untrusted server copy" },
      "The sign-in code could not be verified.",
    );

    expect(error.code).toBe("ACCOUNT_SETUP_REQUIRED");
    expect(error.recoveryAction).toBe("signup");
    expect(authErrorMessage(error, "fallback")).toBe("Your email was verified. Finish account setup to continue.");
  });

  it("maps signup against an existing account to sign-in recovery", () => {
    const error = authResponseError(
      response(),
      { error: "ACCOUNT_EXISTS", message: "untrusted server copy" },
      "The sign-in code could not be verified.",
    );

    expect(error.recoveryAction).toBe("signin");
    expect(authErrorMessage(error, "fallback")).toBe("An account already exists. Continue to sign in.");
  });

  it("keeps invalid or unknown outcomes generic and does not trust server text", () => {
    const error = authResponseError(
      response(401),
      { error: "AUTHENTICATION_FAILED", message: "database diagnostic" },
      "The sign-in code could not be verified.",
    );

    expect(error.recoveryAction).toBeNull();
    expect(authErrorMessage(error, "fallback")).toBe("The code is invalid or expired. Request a new code.");

    const unknown = authResponseError(response(500), { error: "INTERNAL", message: "secret" }, "fallback");
    expect(authErrorMessage(unknown, "fallback")).toBe("fallback");
  });
});
