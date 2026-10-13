// @vitest-environment happy-dom

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import AuthSessionControl from "./AuthSessionControl";
import { parseBrowserAuthConfig } from "./authConfig";

afterEach(() => cleanup());

describe("shared browser sign-in control", () => {
  it("shows the synthetic credential field only for an explicit local build", () => {
    const onCredentialChange = vi.fn();
    render(<AuthSessionControl
      config={parseBrowserAuthConfig({ VITE_DEPLOYMENT_ENVIRONMENT: "local", VITE_LOCAL_CREDENTIAL_ENTRY: "true" })}
      accessToken=""
      status="signed-out"
      onCredentialChange={onCredentialChange}
      onSignIn={() => undefined}
    />);

    const field = screen.getByLabelText("Sign-in credential (held in memory)");
    expect(field).toBeTruthy();
    const disclosure = screen.getByRole("button", { name: "Open local sign-in" });
    expect(disclosure.closest("details")?.hasAttribute("open")).toBe(false);
    fireEvent.change(field, { target: { value: "synthetic" } });
    expect(onCredentialChange).toHaveBeenCalledWith("synthetic");
  });

  it("localizes session actions without exposing the credential value", () => {
    render(<AuthSessionControl
      config={parseBrowserAuthConfig({ VITE_DEPLOYMENT_ENVIRONMENT: "local", VITE_LOCAL_CREDENTIAL_ENTRY: "true" })}
      accessToken=""
      status="signed-out"
      locale="de-DE"
      onCredentialChange={() => undefined}
      onSignIn={() => undefined}
    />);

    expect(screen.getByRole("button", { name: "Lokale Anmeldung öffnen" })).toBeTruthy();
    expect(screen.getByLabelText("Anmeldeinformation (im Speicher gehalten)")).toBeTruthy();
    expect(screen.getByPlaceholderText("Lokale Test-Anmeldeinformation einfügen")).toBeTruthy();
  });

  it("uses a plain sign-in action for OIDC and never renders a bearer field", () => {
    const onSignIn = vi.fn();
    render(<AuthSessionControl
      config={parseBrowserAuthConfig({
        VITE_DEPLOYMENT_ENVIRONMENT: "staging",
        VITE_OIDC_ISSUER: "https://identity.example.test/",
        VITE_OIDC_CLIENT_ID: "console",
        VITE_OIDC_REDIRECT_URI: "https://console.example.test/callback",
        VITE_OIDC_SCOPE: "openid profile",
      })}
      accessToken=""
      status="signed-out"
      onCredentialChange={() => undefined}
      onSignIn={onSignIn}
    />);

    fireEvent.click(screen.getByRole("button", { name: "Sign in" }));
    expect(onSignIn).toHaveBeenCalledTimes(1);
    expect(screen.queryByLabelText(/credential/i)).toBeNull();
  });

  it("renders the two-step email OTP flow without exposing an access token", () => {
    const onRequestOtp = vi.fn();
    const onVerifyOtp = vi.fn();
    const onEmailChange = vi.fn();
    const onOtpChange = vi.fn();
    const config = parseBrowserAuthConfig({
      VITE_DEPLOYMENT_ENVIRONMENT: "local",
      VITE_LOCAL_UI_MODE: "email-otp",
      VITE_AUTH_SERVICE_BASE_URL: "http://localhost:18085",
    });
    render(<AuthSessionControl
      config={config}
      accessToken=""
      status="signed-out"
      email="operator@example.com"
      onEmailChange={onEmailChange}
      onRequestOtp={onRequestOtp}
      onOtpChange={onOtpChange}
      onVerifyOtp={onVerifyOtp}
      onCredentialChange={() => undefined}
      onSignIn={() => undefined}
    />);

    fireEvent.change(screen.getByLabelText("Email address"), { target: { value: "new@example.com" } });
    expect(onEmailChange).toHaveBeenCalledWith("new@example.com");
    fireEvent.click(screen.getByRole("button", { name: "Send code" }));
    expect(onRequestOtp).toHaveBeenCalledTimes(1);

    cleanup();
    render(<AuthSessionControl
      config={config}
      accessToken=""
      status="signed-out"
      email="operator@example.com"
      otpCode="123456"
      otpChallengeId="0192e6ec-4d8f-7d2e-a40e-2c095f871405"
      onOtpChange={onOtpChange}
      onVerifyOtp={onVerifyOtp}
      onCredentialChange={() => undefined}
      onSignIn={() => undefined}
    />);
    fireEvent.change(screen.getByLabelText("One-time code"), { target: { value: "654321" } });
    fireEvent.click(screen.getByRole("button", { name: "Verify code" }));
    expect(onOtpChange).toHaveBeenCalledWith("654321");
    expect(onVerifyOtp).toHaveBeenCalledTimes(1);
  });

  it("keeps email sign-in compact until the user opens it", () => {
    const onExpandedChange = vi.fn();
    const config = parseBrowserAuthConfig({
      VITE_DEPLOYMENT_ENVIRONMENT: "local",
      VITE_LOCAL_UI_MODE: "email-otp",
      VITE_AUTH_SERVICE_BASE_URL: "http://localhost:18085",
    });

    render(<AuthSessionControl
      config={config}
      accessToken=""
      status="signed-out"
      onExpandedChange={onExpandedChange}
      onCredentialChange={() => undefined}
      onSignIn={() => undefined}
    />);

    const disclosure = screen.getByRole("button", { name: "Open email sign-in" });
    expect(disclosure.closest("details")?.hasAttribute("open")).toBe(false);
    fireEvent.click(disclosure);
    expect(onExpandedChange).toHaveBeenCalledWith(true);
  });

  it("keeps a configured failure visible and protected sign-in unavailable", () => {
    render(<AuthSessionControl
      config={parseBrowserAuthConfig({ VITE_DEPLOYMENT_ENVIRONMENT: "production" })}
      accessToken=""
      status="error"
      error="The identity provider is not configured."
      onCredentialChange={() => undefined}
      onSignIn={() => undefined}
    />);

    expect(screen.getByRole("alert").textContent).toContain("The identity provider is not configured.");
    expect(screen.queryByRole("button", { name: "Sign in" })).toBeNull();
  });

  it("does not make an unconfigured sign-in error dominate the shell", () => {
    render(<AuthSessionControl
      config={parseBrowserAuthConfig({ VITE_DEPLOYMENT_ENVIRONMENT: "production" })}
      accessToken=""
      status="error"
      error="The identity provider is not configured."
      onCredentialChange={() => undefined}
      onSignIn={() => undefined}
    />);

    const summary = screen.getByRole("button", { name: "Sign-in setup needed" });
    expect(summary.closest("details")?.hasAttribute("open")).toBe(false);
    fireEvent.click(summary);
    expect(screen.getByRole("alert").textContent).toContain("The identity provider is not configured.");
  });

  it("leaves signed-in identity presentation to the account menu without exposing the access token", () => {
    render(<AuthSessionControl
      config={parseBrowserAuthConfig({
        VITE_DEPLOYMENT_ENVIRONMENT: "staging",
        VITE_OIDC_ISSUER: "https://identity.example.test/",
        VITE_OIDC_CLIENT_ID: "console",
        VITE_OIDC_REDIRECT_URI: "https://console.example.test/callback",
        VITE_OIDC_SCOPE: "openid",
      })}
      accessToken="secret-access-token"
      status="signed-in"
      onCredentialChange={() => undefined}
      onSignIn={() => undefined}
    />);

    expect(screen.queryByText("Signed in")).toBeNull();
    expect(screen.queryByText("secret-access-token")).toBeNull();
    expect(screen.queryByRole("button", { name: "Sign out" })).toBeNull();
  });
});
