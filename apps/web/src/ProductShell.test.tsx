// @vitest-environment happy-dom

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import ProductShell from "./ProductShell";
import { parseBrowserAuthConfig } from "./authConfig";

afterEach(() => cleanup());

describe("product shell access state", () => {
  it("opens with an expanded reference-style rail while preserving labeled access", () => {
    render(<ProductShell
      workspace="quote"
      onNavigate={vi.fn()}
      serviceState="online"
      accessToken=""
      locale="en-US"
      timeZone="UTC"
      onLocaleChange={vi.fn()}
      onTimeZoneChange={vi.fn()}
    >
      <div>Workspace content</div>
    </ProductShell>);

    expect(document.querySelector(".product-shell")?.className).not.toContain("navigation-compact");
    expect(document.querySelector(".metronic-navbar")).toBeTruthy();
    expect(document.querySelector(".metronic-content-frame")).toBeTruthy();
    expect(screen.getByRole("link", { name: "Solana Tax Protocol home" })).toBeTruthy();
    expect(document.querySelector(".metronic-header .product-brand")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Collapse navigation" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Tax quotes" })).toBeTruthy();
  });

  it("uses verified shell context for the workspace status deck", () => {
    render(<ProductShell
      workspace="quote"
      onNavigate={vi.fn()}
      serviceState="online"
      accessToken=""
      locale="en-US"
      timeZone="UTC"
      onLocaleChange={vi.fn()}
      onTimeZoneChange={vi.fn()}
    >
      <div>Workspace content</div>
    </ProductShell>);

    expect(document.querySelectorAll(".workspace-status-card")).toHaveLength(4);
    expect(document.querySelector(".workspace-status-card-primary")).toBeTruthy();
    expect(document.querySelector(".workspace-status-card[data-status-kind=deployment]")).toBeTruthy();
    expect(document.querySelector(".workspace-status-card[data-status-kind=network]")).toBeTruthy();
    expect(document.querySelector(".workspace-status-card[data-status-kind=api]")).toBeTruthy();
    expect(screen.getAllByText("Your access").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Deployment environment").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Solana network").length).toBeGreaterThan(0);
    expect(screen.getAllByText("API services").length).toBeGreaterThan(0);
  });

  it("does not repeat the access summary in the public workspace heading", () => {
    render(<ProductShell
      workspace="preview"
      onNavigate={vi.fn()}
      serviceState="online"
      accessToken=""
      locale="en-US"
      timeZone="UTC"
      onLocaleChange={vi.fn()}
      onTimeZoneChange={vi.fn()}
    >
      <div>Scenario content</div>
    </ProductShell>);

    expect(document.querySelector(".page-context-card")).toBeNull();
    expect(document.querySelector(".workspace-status-card-primary")?.textContent).toContain("Available without sign-in");
  });

  it("does not mount protected workspace controls before server access is confirmed", () => {
    render(<ProductShell
      workspace="checkout"
      onNavigate={vi.fn()}
      serviceState="online"
      accessToken="signed-session"
      authConfig={parseBrowserAuthConfig({
        VITE_DEPLOYMENT_ENVIRONMENT: "local",
        VITE_LOCAL_UI_MODE: "email-otp",
        VITE_AUTH_SERVICE_BASE_URL: "http://localhost:18085",
      })}
      locale="en-US"
      timeZone="UTC"
      onLocaleChange={vi.fn()}
      onTimeZoneChange={vi.fn()}
    >
      <button type="button">Protected checkout control</button>
    </ProductShell>);

    expect(screen.getByRole("heading", { name: "Checking access" })).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Protected checkout control" })).toBeNull();
  });

  it("keeps the protected workspace gate explanatory with one global sign-in action", () => {
    const config = parseBrowserAuthConfig({
      VITE_DEPLOYMENT_ENVIRONMENT: "local",
      VITE_LOCAL_UI_MODE: "email-otp",
      VITE_AUTH_SERVICE_BASE_URL: "http://localhost:18085",
    });

    render(<ProductShell
      workspace="checkout"
      onNavigate={vi.fn()}
      serviceState="online"
      accessToken=""
      authConfig={config}
      locale="en-US"
      timeZone="UTC"
      onLocaleChange={vi.fn()}
      onTimeZoneChange={vi.fn()}
    >
      <div>Protected checkout form</div>
    </ProductShell>);

    expect(screen.getByRole("heading", { name: "Sign in to continue" })).toBeTruthy();
    expect(screen.queryByText("Protected checkout form")).toBeNull();
    expect(screen.getAllByText("Your access").length).toBeGreaterThan(0);

    expect(screen.getByText("Use the sign-in control in the header to continue.")).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Open sign-in" })).toBeNull();
    expect(screen.getByRole("button", { name: "Open email sign-in" })).toBeTruthy();
    expect(screen.getByRole("list", { name: "Protected workspace steps" })).toBeTruthy();
    expect(document.querySelector(".workspace-auth-gate")?.getAttribute("data-layout")).toBe("compact-callout");
  });

  it("does not duplicate the signed-out email sign-in action with an account menu", () => {
    const config = parseBrowserAuthConfig({
      VITE_DEPLOYMENT_ENVIRONMENT: "local",
      VITE_LOCAL_UI_MODE: "email-otp",
      VITE_AUTH_SERVICE_BASE_URL: "http://localhost:18085",
    });

    render(<ProductShell
      workspace="quote"
      onNavigate={vi.fn()}
      serviceState="online"
      accessToken=""
      authConfig={config}
      locale="en-US"
      timeZone="UTC"
      onLocaleChange={vi.fn()}
      onTimeZoneChange={vi.fn()}
    >
      <div>Workspace content</div>
    </ProductShell>);

    expect(screen.getByRole("button", { name: "Open email sign-in" })).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Open account menu" })).toBeNull();
  });

  it("starts the configured OIDC flow from the global sign-in control", () => {
    const onSignIn = vi.fn();
    const config = parseBrowserAuthConfig({
      VITE_DEPLOYMENT_ENVIRONMENT: "staging",
      VITE_OIDC_ISSUER: "https://identity.example.test/",
      VITE_OIDC_CLIENT_ID: "console",
      VITE_OIDC_REDIRECT_URI: "https://console.example.test/callback",
      VITE_OIDC_SCOPE: "openid profile",
    });

    render(<ProductShell
      workspace="checkout"
      onNavigate={vi.fn()}
      serviceState="online"
      accessToken=""
      authConfig={config}
      onSignIn={onSignIn}
      locale="en-US"
      timeZone="UTC"
      onLocaleChange={vi.fn()}
      onTimeZoneChange={vi.fn()}
    >
      <div>Protected checkout form</div>
    </ProductShell>);

    fireEvent.click(screen.getByRole("button", { name: "Sign in" }));
    expect(onSignIn).toHaveBeenCalledTimes(1);
  });

  it("does not add a second OIDC action inside the protected workspace gate", () => {
    const onSignIn = vi.fn();
    const config = parseBrowserAuthConfig({
      VITE_DEPLOYMENT_ENVIRONMENT: "staging",
      VITE_OIDC_ISSUER: "https://identity.example.test/",
      VITE_OIDC_CLIENT_ID: "console",
      VITE_OIDC_REDIRECT_URI: "https://console.example.test/callback",
      VITE_OIDC_SCOPE: "openid profile",
    });

    render(<ProductShell
      workspace="checkout"
      onNavigate={vi.fn()}
      serviceState="online"
      accessToken=""
      authConfig={config}
      onSignIn={onSignIn}
      locale="en-US"
      timeZone="UTC"
      onLocaleChange={vi.fn()}
      onTimeZoneChange={vi.fn()}
    >
      <div>Protected checkout form</div>
    </ProductShell>);

    expect(screen.queryByRole("button", { name: "Open sign-in" })).toBeNull();
    expect(screen.getByRole("button", { name: "Sign in" })).toBeTruthy();
    expect(onSignIn).not.toHaveBeenCalled();
  });

  it("does not block the public tax simulator behind sign-in", () => {
    render(<ProductShell
      workspace="preview"
      onNavigate={vi.fn()}
      serviceState="online"
      accessToken=""
      locale="en-US"
      timeZone="UTC"
      onLocaleChange={vi.fn()}
      onTimeZoneChange={vi.fn()}
    >
      <div>Scenario form</div>
    </ProductShell>);

    expect(screen.queryByRole("heading", { name: "Sign in to continue" })).toBeNull();
    expect(screen.getByText("Scenario form")).toBeTruthy();
    expect(screen.getAllByText("Available without sign-in").length).toBeGreaterThan(0);
    expect(screen.queryByText("Protected actions may need review or approval.", { exact: true })).toBeNull();
  });

  it("does not repeat stable verified access above the authenticated task", () => {
    render(<ProductShell
      workspace="quote"
      onNavigate={vi.fn()}
      serviceState="online"
      accessToken="verified-session"
      authorizationSnapshot={{
        state: "server-allowed",
        expiresAtUnix: Math.floor(Date.now() / 1000) + 60,
        subject: "operator",
        tenantId: "01900000-0000-4000-8000-000000000001",
        roles: [],
        scopes: [],
        attributes: {},
      }}
      locale="en-US"
      timeZone="UTC"
      onLocaleChange={vi.fn()}
      onTimeZoneChange={vi.fn()}
    >
      <div>Authenticated task content</div>
    </ProductShell>);

    expect(screen.getAllByText("Your access").length).toBeGreaterThan(0);
    expect(screen.getByText("Authenticated task content")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Open account menu" })).toBeTruthy();
  });
});
