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
      onSignOut={() => undefined}
    />);

    const field = screen.getByLabelText("Sign-in credential (held in memory)");
    expect(field).toBeTruthy();
    const disclosure = screen.getByRole("button", { name: "Open local sign-in" });
    expect(disclosure.closest("details")?.hasAttribute("open")).toBe(false);
    fireEvent.change(field, { target: { value: "synthetic" } });
    expect(onCredentialChange).toHaveBeenCalledWith("synthetic");
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
      onSignOut={() => undefined}
    />);

    fireEvent.click(screen.getByRole("button", { name: "Sign in" }));
    expect(onSignIn).toHaveBeenCalledTimes(1);
    expect(screen.queryByLabelText(/credential/i)).toBeNull();
  });

  it("keeps a configured failure visible and protected sign-in unavailable", () => {
    render(<AuthSessionControl
      config={parseBrowserAuthConfig({ VITE_DEPLOYMENT_ENVIRONMENT: "production" })}
      accessToken=""
      status="error"
      error="The identity provider is not configured."
      onCredentialChange={() => undefined}
      onSignIn={() => undefined}
      onSignOut={() => undefined}
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
      onSignOut={() => undefined}
    />);

    const summary = screen.getByRole("button", { name: "Sign-in unavailable" });
    expect(summary.closest("details")?.hasAttribute("open")).toBe(false);
    fireEvent.click(summary);
    expect(screen.getByRole("alert").textContent).toContain("The identity provider is not configured.");
  });

  it("offers sign out without exposing the access token", () => {
    const onSignOut = vi.fn();
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
      onSignOut={onSignOut}
    />);

    expect(screen.getByText("Signed in")).toBeTruthy();
    expect(screen.queryByText("secret-access-token")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Sign out" }));
    expect(onSignOut).toHaveBeenCalledTimes(1);
  });
});
