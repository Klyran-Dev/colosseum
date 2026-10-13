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

    expect(screen.getByRole("main", { name: "Sign in to Solana Tax Protocol" }).getAttribute("data-layout")).toBe("auth-branded");
    expect(screen.getByRole("complementary", { name: "Solana Tax Protocol security context" }).getAttribute("data-panel")).toBe("brand");
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

    expect(screen.getByRole("heading", { name: "Sign in to Solana Tax Protocol" })).toBeTruthy();
    expect(screen.getByRole("complementary", { name: "Solana Tax Protocol security context" })).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Evidence at every step" })).toBeTruthy();
    expect(screen.getByLabelText("Work email")).toBeTruthy();
    expect(screen.getByText("Your account and workspace access are verified before protected actions are available.")).toBeTruthy();
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
    expect(screen.getByText("New accounts start with standard operator access. Elevated responsibilities are assigned by an administrator.")).toBeTruthy();
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

    expect(screen.getByRole("heading", { name: "Check your email" })).toBeTruthy();
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
});
