// @vitest-environment happy-dom

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import AuthPage from "./AuthPage";
import { parseBrowserAuthConfig } from "./authConfig";

afterEach(() => cleanup());

const config = parseBrowserAuthConfig({
  VITE_DEPLOYMENT_ENVIRONMENT: "local",
  VITE_LOCAL_UI_MODE: "email-otp",
  VITE_AUTH_SERVICE_BASE_URL: "http://localhost:18085",
});

describe("Metronic authentication page", () => {
  it("exposes the licensed Demo 3 branded layout contract", () => {
    render(<AuthPage
      intent="signin"
      config={config}
      status="signed-out"
      email=""
      otpCode=""
      otpChallengeId={null}
      onEmailChange={() => undefined}
      onOtpChange={() => undefined}
      onRequestOtp={() => undefined}
      onVerifyOtp={() => undefined}
      onResetOtp={() => undefined}
      onNavigate={() => undefined}
    />);

    expect(screen.getByRole("main", { name: "Sign in to your workspace" }).getAttribute("data-layout")).toBe("auth-branded");
    expect(screen.getByRole("complementary", { name: "Solana Tax Protocol sign-in help" }).getAttribute("data-panel")).toBe("brand");
    expect(screen.getByText("Tax operations", { selector: ".auth-context-brand small" })).toBeTruthy();
  });

  it("renders the sign-in journey as a focused two-step surface", () => {
    const onRequestOtp = vi.fn();
    render(<AuthPage
      intent="signin"
      config={config}
      status="signed-out"
      email="operator@example.com"
      otpCode=""
      otpChallengeId={null}
      onEmailChange={() => undefined}
      onOtpChange={() => undefined}
      onRequestOtp={onRequestOtp}
      onVerifyOtp={() => undefined}
      onResetOtp={() => undefined}
      onNavigate={() => undefined}
    />);

    expect(screen.getByRole("heading", { name: "Sign in to your workspace" })).toBeTruthy();
    expect(screen.getByRole("complementary", { name: "Solana Tax Protocol sign-in help" })).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Simple, secure access" })).toBeTruthy();
    expect(screen.getByLabelText("Work email")).toBeTruthy();
    expect(screen.getByText("Enter your work email and we’ll send you a one-time code.")).toBeTruthy();
    fireEvent.change(screen.getByLabelText("Work email"), { target: { value: "operator@example.com" } });
    fireEvent.click(screen.getByRole("button", { name: "Continue with email" }));
    expect(onRequestOtp).toHaveBeenCalledTimes(1);
    expect(screen.getByRole("link", { name: "Create an account" })).toBeTruthy();
  });

  it("renders sign-up copy without offering a role selector", () => {
    render(<AuthPage
      intent="signup"
      config={config}
      status="signed-out"
      email="new@example.com"
      otpCode=""
      otpChallengeId={null}
      onEmailChange={() => undefined}
      onOtpChange={() => undefined}
      onRequestOtp={() => undefined}
      onVerifyOtp={() => undefined}
      onResetOtp={() => undefined}
      onNavigate={() => undefined}
    />);

    expect(screen.getByRole("heading", { name: "Create your workspace account" })).toBeTruthy();
    expect(screen.getByText("Your organization controls access.")).toBeTruthy();
    expect(screen.queryByLabelText(/role/i)).toBeNull();
    expect(screen.getByRole("link", { name: "Back to sign in" })).toBeTruthy();
  });

  it("shows the server-confirmed invited responsibility without making it selectable", () => {
    render(<AuthPage
      intent="signup"
      config={config}
      status="signed-out"
      email="moderator@example.com"
      otpCode=""
      otpChallengeId={null}
      invitationId="0192e6ec-4d8f-7d2e-a40e-2c095f871405"
      invitationRole="moderator"
      onEmailChange={() => undefined}
      onOtpChange={() => undefined}
      onRequestOtp={() => undefined}
      onVerifyOtp={() => undefined}
      onResetOtp={() => undefined}
      onNavigate={() => undefined}
    />);

    expect(screen.getByText("Invitation received")).toBeTruthy();
    expect(screen.getByText("Moderator responsibility")).toBeTruthy();
    expect(screen.queryByRole("combobox")).toBeNull();
  });

  it("moves to code verification without exposing a bearer token", () => {
    const onRequestOtp = vi.fn();
    render(<AuthPage
      intent="signin"
      config={config}
      status="signed-out"
      email="operator@example.com"
      otpCode="123456"
      otpChallengeId="0192e6ec-4d8f-7d2e-a40e-2c095f871405"
      onEmailChange={() => undefined}
      onOtpChange={() => undefined}
      onRequestOtp={onRequestOtp}
      onVerifyOtp={() => undefined}
      onResetOtp={() => undefined}
      onNavigate={() => undefined}
    />);

    expect(screen.getByRole("heading", { name: "Enter your code" })).toBeTruthy();
    expect(screen.getByLabelText("One-time code")).toBeTruthy();
    expect(screen.queryByText(/access token|bearer/i)).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Resend code" }));
    expect(onRequestOtp).toHaveBeenCalledTimes(1);
  });

  it("honors the server resend cooldown and explains when recovery is available", () => {
    render(<AuthPage
      intent="signin"
      config={config}
      status="signed-out"
      email="operator@example.com"
      otpCode=""
      otpChallengeId="0192e6ec-4d8f-7d2e-a40e-2c095f871405"
      resendAvailableAtUnix={Math.floor(Date.now() / 1000) + 30}
      onEmailChange={() => undefined}
      onOtpChange={() => undefined}
      onRequestOtp={() => undefined}
      onVerifyOtp={() => undefined}
      onResetOtp={() => undefined}
      onNavigate={() => undefined}
    />);

    expect(screen.getByRole("button", { name: /Resend code in about 30 seconds/ })).toHaveProperty("disabled", true);
  });

  it("renders an explicit recovery action for a verified account-flow outcome", () => {
    const onRecoveryAction = vi.fn();
    render(<AuthPage
      intent="signin"
      config={config}
      status="error"
      error="Your email was verified. Finish account setup to continue."
      recoveryAction="signup"
      onRecoveryAction={onRecoveryAction}
      email="initial-admin@local.test"
      otpCode=""
      otpChallengeId={null}
      onEmailChange={() => undefined}
      onOtpChange={() => undefined}
      onRequestOtp={() => undefined}
      onVerifyOtp={() => undefined}
      onResetOtp={() => undefined}
      onNavigate={() => undefined}
    />);

    fireEvent.click(screen.getByRole("button", { name: "Finish account setup" }));
    expect(onRecoveryAction).toHaveBeenCalledTimes(1);
  });
});
