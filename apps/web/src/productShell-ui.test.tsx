// @vitest-environment happy-dom

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import ProductShell from "./ProductShell";
import { AuthorizationSnapshot } from "./authorization";
import { parseRuntimeContext } from "./runtimeContext";
import { parseBrowserAuthConfig } from "./authConfig";

afterEach(() => cleanup());

describe("operator-first product shell", () => {
  const testExpiryUnix = Math.floor(Date.now() / 1000) + 60;

  it("connects the keyboard skip link to the stable main workspace landmark", () => {
    render(<ProductShell
      workspace="quote"
      onNavigate={() => undefined}
      serviceState="online"
      accessToken=""
      locale="en-US"
      timeZone="UTC"
      onLocaleChange={() => undefined}
      onTimeZoneChange={() => undefined}
    >
      <div>Workspace content</div>
    </ProductShell>);

    const skipLink = screen.getByRole("link", { name: "Skip to workspace content" });
    expect(skipLink.getAttribute("href")).toBe("#main-content");
    expect(document.getElementById("main-content")?.tagName).toBe("MAIN");
  });

  it("uses task language for the primary navigation", () => {
    render(<ProductShell
      workspace="quote"
      onNavigate={() => undefined}
      serviceState="online"
      accessToken=""
      locale="en-US"
      timeZone="UTC"
      onLocaleChange={() => undefined}
      onTimeZoneChange={() => undefined}
    >
      <div>Workspace content</div>
    </ProductShell>);

    expect(screen.getByRole("button", { name: "Tax quotes" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Tax rules" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Treasury" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Exceptions & recovery" })).toBeTruthy();
    expect(screen.getByText("Automated withholding and evidence—not tax filing or legal advice.")).toBeTruthy();
    expect(screen.queryByText("Control plane")).toBeNull();
    expect(screen.queryByText("Move funds")).toBeNull();
  });

  it("uses the selected locale for the shell navigation and workspace heading", () => {
    render(<ProductShell
      workspace="quote"
      onNavigate={() => undefined}
      serviceState="online"
      accessToken=""
      locale="de-DE"
      timeZone="UTC"
      onLocaleChange={() => undefined}
      onTimeZoneChange={() => undefined}
    >
      <div>Workspace content</div>
    </ProductShell>);

    expect(screen.getByRole("button", { name: "Steuerangebote" })).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Steuerangebote" })).toBeTruthy();
    expect(screen.getByRole("group", { name: "Tägliche Arbeit" })).toBeTruthy();
    expect(screen.getByRole("status").textContent).toBe("Arbeitsbereich: Steuerangebote");
  });

  it("localizes navigation access status and keeps raw server reasons out of the affordance", () => {
    render(<ProductShell
      workspace="quote"
      onNavigate={() => undefined}
      serviceState="online"
      accessToken="verified-session"
      authorizationSnapshot={{
        state: "server-allowed",
        expiresAtUnix: testExpiryUnix,
        subject: "operator-1",
        tenantId: "01900000-0000-4000-8000-000000000001",
        roles: [],
        scopes: [],
        attributes: {},
        capabilities: {
          "tax.preview.run": { kind: "allow", reasonCode: "scope_and_role_satisfied", reason: "The service confirmed this action for the current account." },
          "refund.claim.create": { kind: "deny", reasonCode: "role_missing", reason: "This action needs an approved responsibility." },
        },
      }}
      locale="de-DE"
      timeZone="UTC"
      onLocaleChange={() => undefined}
      onTimeZoneChange={() => undefined}
    >
      <div>Workspace content</div>
    </ProductShell>);

    const availableTitle = screen.getByRole("button", { name: "Steuersimulator" }).getAttribute("title") ?? "";
    const unavailableTitle = screen.getByRole("button", { name: "Erstattungsanfragen" }).getAttribute("title") ?? "";
    expect(availableTitle).toContain("Verfügbar");
    expect(unavailableTitle).toContain("Nicht verfügbar");
    expect(unavailableTitle).toContain("Für diesen Arbeitsbereich ist eine genehmigte Verantwortung erforderlich.");
    expect(unavailableTitle).not.toContain("This action needs an approved responsibility.");
    expect(availableTitle).not.toContain("Available");
    expect(unavailableTitle).not.toContain("Not available");
  });

  it("uses stable semantic glyphs instead of first-letter avatars", () => {
    render(<ProductShell
      workspace="quote"
      onNavigate={() => undefined}
      serviceState="online"
      accessToken=""
      locale="en-US"
      timeZone="UTC"
      onLocaleChange={() => undefined}
      onTimeZoneChange={() => undefined}
    >
      <div>Workspace content</div>
    </ProductShell>);

    const quoteButton = screen.getByRole("button", { name: "Tax quotes" });
    const paymentButton = screen.getByRole("button", { name: "Payment review" });
    expect(quoteButton.querySelector("svg")).toBeTruthy();
    expect(paymentButton.querySelector("svg")).toBeTruthy();
    expect(quoteButton.querySelector("svg")?.getAttribute("aria-hidden")).toBe("true");
    expect(quoteButton.querySelector(".nav-item-icon")?.textContent).toBe("");
  });

  it("keeps technical context out of the primary page title", () => {
    const snapshot: AuthorizationSnapshot = {
      state: "server-allowed",
      expiresAtUnix: testExpiryUnix,
      subject: "operator-1",
      tenantId: "01900000-0000-7000-8000-000000000001",
      roles: [],
      scopes: [],
      attributes: {},
      capabilities: {},
    };
    render(<ProductShell
      workspace="access-control"
      onNavigate={() => undefined}
      serviceState="online"
      accessToken="token"
      authorizationSnapshot={snapshot}
      locale="en-US"
      timeZone="UTC"
      onLocaleChange={() => undefined}
      onTimeZoneChange={() => undefined}
    >
      <div>Workspace content</div>
    </ProductShell>);

    expect(screen.getByRole("heading", { name: "Access & approvals" })).toBeTruthy();
    expect(screen.getAllByText("Access checked").length).toBeGreaterThan(0);
    expect(screen.queryByText("RBAC · PBAC · ABAC · tenant scoped")).toBeNull();
  });

  it("shows only the verified account reference and never fixed tenant or operator labels", () => {
    const snapshot: AuthorizationSnapshot = {
      state: "server-allowed",
      expiresAtUnix: testExpiryUnix,
      subject: "operator-verified",
      tenantId: "01900000-0000-4000-8000-000000000001",
      roles: [],
      scopes: [],
      attributes: {},
      capabilities: {},
    };

    render(<ProductShell
      workspace="quote"
      onNavigate={() => undefined}
      serviceState="online"
      accessToken="verified-session"
      authorizationSnapshot={snapshot}
      runtimeContext={parseRuntimeContext({ VITE_DEPLOYMENT_ENVIRONMENT: "staging", VITE_SOLANA_NETWORK: "devnet" })}
      locale="en-US"
      timeZone="UTC"
      onLocaleChange={() => undefined}
      onTimeZoneChange={() => undefined}
    >
      <div>Workspace content</div>
    </ProductShell>);

    expect(screen.getAllByText("Account 01900000").length).toBe(1);
    expect(screen.getByText("Staging")).toBeTruthy();
    expect(screen.getByText("Solana Devnet")).toBeTruthy();
    expect(screen.queryByText("Acme Treasury")).toBeNull();
    expect(screen.queryByText("RA")).toBeNull();
  });

  it("makes missing deployment context visible instead of showing a plausible badge", () => {
    render(<ProductShell
      workspace="quote"
      onNavigate={() => undefined}
      serviceState="online"
      accessToken=""
      runtimeContext={parseRuntimeContext({})}
      locale="en-US"
      timeZone="UTC"
      onLocaleChange={() => undefined}
      onTimeZoneChange={() => undefined}
    >
      <div>Workspace content</div>
    </ProductShell>);

    expect(screen.getAllByText("Not configured").length).toBeGreaterThanOrEqual(2);
    const systemWarning = screen.getByText(/release build/i);
    expect(systemWarning.closest(".system-status-warning")?.getAttribute("role")).toBeNull();
    expect(screen.queryByText("Staging")).toBeNull();
    expect(screen.queryByText("Test network")).toBeNull();
  });

  it("keeps system diagnostics behind one calm, accessible disclosure", () => {
    render(<ProductShell
      workspace="quote"
      onNavigate={() => undefined}
      serviceState="offline"
      accessToken=""
      runtimeContext={parseRuntimeContext({})}
      locale="en-US"
      timeZone="UTC"
      onLocaleChange={() => undefined}
      onTimeZoneChange={() => undefined}
    >
      <div>Workspace content</div>
    </ProductShell>);

    const summary = screen.getByRole("button", { name: /System status/i });
    expect(summary.closest("details")?.hasAttribute("open")).toBe(false);

    fireEvent.click(summary);
    expect(screen.getByText("Deployment environment")).toBeTruthy();
    expect(screen.getByText("Solana network")).toBeTruthy();
    expect(screen.getByText("API services")).toBeTruthy();
    expect(screen.getByText("Sign in to see account actions.")).toBeTruthy();
  });

  it("keeps top-bar popovers mutually exclusive", () => {
    render(<ProductShell
      workspace="quote"
      onNavigate={() => undefined}
      serviceState="online"
      accessToken=""
      authConfig={parseBrowserAuthConfig({ VITE_DEPLOYMENT_ENVIRONMENT: "local", VITE_LOCAL_CREDENTIAL_ENTRY: "true" })}
      locale="en-US"
      timeZone="UTC"
      onLocaleChange={() => undefined}
      onTimeZoneChange={() => undefined}
    >
      <div>Workspace content</div>
    </ProductShell>);

    const displaySettings = screen.getByRole("button", { name: "Open display settings" });
    const localSignIn = screen.getByRole("button", { name: "Open local sign-in" });
    fireEvent.click(displaySettings);
    expect(displaySettings.closest("details")?.hasAttribute("open")).toBe(true);
    fireEvent.click(localSignIn);
    expect(localSignIn.closest("details")?.hasAttribute("open")).toBe(true);
    expect(displaySettings.closest("details")?.hasAttribute("open")).toBe(false);
  });

  it("keeps protection assurances behind one compact page-context disclosure", () => {
    render(<ProductShell
      workspace="quote"
      onNavigate={() => undefined}
      serviceState="online"
      accessToken=""
      locale="en-US"
      timeZone="UTC"
      onLocaleChange={() => undefined}
      onTimeZoneChange={() => undefined}
    >
      <div>Workspace content</div>
    </ProductShell>);

    expect(document.querySelector(".security-strip")).toBeNull();
    const summary = screen.getByRole("button", { name: "Open protection details" });
    expect(summary.closest("details")?.hasAttribute("open")).toBe(false);

    fireEvent.click(summary);
    expect(screen.getByText("Account access")).toBeTruthy();
    expect(screen.getByText("Every decision has a record.")).toBeTruthy();
    expect(screen.getByText("Sensitive actions need approval.")).toBeTruthy();
    expect(screen.getByText("Wallets stay with their owners.")).toBeTruthy();
  });

  it("provides a labeled narrow-screen navigation disclosure that closes after selection", () => {
    const onNavigate = () => undefined;
    render(<ProductShell
      workspace="quote"
      onNavigate={onNavigate}
      serviceState="online"
      accessToken=""
      locale="en-US"
      timeZone="UTC"
      onLocaleChange={() => undefined}
      onTimeZoneChange={() => undefined}
    >
      <div>Workspace content</div>
    </ProductShell>);

    const toggle = screen.getByRole("button", { name: "Open navigation" });
    expect(toggle.getAttribute("aria-expanded")).toBe("false");
    expect(toggle.getAttribute("aria-controls")).toBe("primary-navigation");

    fireEvent.click(toggle);
    expect(screen.getByRole("button", { name: "Close navigation" }).getAttribute("aria-expanded")).toBe("true");

    fireEvent.click(screen.getByRole("button", { name: "Tax simulator" }));
    expect(screen.getByRole("button", { name: "Open navigation" }).getAttribute("aria-expanded")).toBe("false");
  });

  it("keeps daily work open and lower-frequency navigation groups progressively disclosed", () => {
    render(<ProductShell
      workspace="document-checkout"
      onNavigate={() => undefined}
      serviceState="online"
      accessToken=""
      locale="en-US"
      timeZone="UTC"
      onLocaleChange={() => undefined}
      onTimeZoneChange={() => undefined}
    >
      <div>Workspace content</div>
    </ProductShell>);

    expect(screen.getByRole("group", { name: "Daily work" }).closest("details")?.open).toBe(true);
    expect(screen.getByRole("group", { name: "Money movement" }).closest("details")?.open).toBe(false);
    expect(screen.getByRole("group", { name: "Rules & access" }).closest("details")?.open).toBe(false);
    expect(screen.getByRole("button", { name: "Document payment" }).getAttribute("aria-current")).toBe("page");
  });

  it("keeps one navigation group open after an operator changes work areas", () => {
    render(<ProductShell
      workspace="quote"
      onNavigate={() => undefined}
      serviceState="online"
      accessToken=""
      locale="en-US"
      timeZone="UTC"
      onLocaleChange={() => undefined}
      onTimeZoneChange={() => undefined}
    >
      <div>Workspace content</div>
    </ProductShell>);

    const dailyWork = screen.getByRole("group", { name: "Daily work" }).closest("details");
    const rulesAndAccess = screen.getByRole("group", { name: "Rules & access" }).closest("details");
    expect(dailyWork?.open).toBe(true);
    expect(rulesAndAccess?.open).toBe(false);

    fireEvent.click(screen.getByText("Rules & access", { exact: true }));

    expect(rulesAndAccess?.open).toBe(true);
    expect(dailyWork?.open).toBe(false);
  });

  it("announces the current workspace without exposing sensitive context", () => {
    const { rerender } = render(<ProductShell
      workspace="quote"
      onNavigate={() => undefined}
      serviceState="online"
      accessToken="verified-session"
      authorizationSnapshot={{
        state: "server-allowed",
        expiresAtUnix: testExpiryUnix,
        subject: "operator-verified",
        tenantId: "01900000-0000-4000-8000-000000000001",
        roles: ["tax_rule_reviewer"],
        scopes: ["tax:quote"],
        attributes: {},
        capabilities: {},
      }}
      locale="en-US"
      timeZone="UTC"
      onLocaleChange={() => undefined}
      onTimeZoneChange={() => undefined}
    >
      <div>Workspace content</div>
    </ProductShell>);

    expect(screen.getByRole("status").textContent).toBe("Tax quotes workspace");
    expect(screen.getByRole("status").textContent).not.toMatch(/operator|01900000|tax:quote|signed|amount|digest/i);

    rerender(<ProductShell
      workspace="governance"
      onNavigate={() => undefined}
      serviceState="online"
      accessToken="verified-session"
      authorizationSnapshot={{
        state: "server-allowed",
        expiresAtUnix: testExpiryUnix,
        subject: "operator-verified",
        tenantId: "01900000-0000-4000-8000-000000000001",
        roles: ["tax_rule_reviewer"],
        scopes: ["tax:quote"],
        attributes: {},
        capabilities: {},
      }}
      locale="en-US"
      timeZone="UTC"
      onLocaleChange={() => undefined}
      onTimeZoneChange={() => undefined}
    >
      <div>Workspace content</div>
    </ProductShell>);

    expect(screen.getByRole("status").textContent).toBe("Tax rules workspace");
  });

  it("shows server-owned access status beside high-consequence workspaces", () => {
    const snapshot: AuthorizationSnapshot = {
      state: "server-allowed",
      expiresAtUnix: testExpiryUnix,
      subject: "operator-1",
      tenantId: "01900000-0000-4000-8000-000000000001",
      roles: [],
      scopes: [],
      attributes: {},
      capabilities: {
        "tax.preview.run": { kind: "allow", reasonCode: "scope_and_role_satisfied", reason: "The service confirmed this action for the current account." },
        "refund.claim.create": { kind: "deny", reasonCode: "role_missing", reason: "This action needs an approved responsibility." },
      },
    };
    render(<ProductShell
      workspace="quote"
      onNavigate={() => undefined}
      serviceState="online"
      accessToken="verified-session"
      authorizationSnapshot={snapshot}
      locale="en-US"
      timeZone="UTC"
      onLocaleChange={() => undefined}
      onTimeZoneChange={() => undefined}
    >
      <div>Workspace content</div>
    </ProductShell>);

    expect(screen.getByRole("button", { name: "Tax simulator" }).getAttribute("title")).toMatch(/Available/);
    expect(screen.getByRole("button", { name: "Refund requests" }).getAttribute("title")).toMatch(/Not available/);
  });

  it("keeps navigation calm while preserving accessible access context", () => {
    const snapshot: AuthorizationSnapshot = {
      state: "server-allowed",
      expiresAtUnix: testExpiryUnix,
      subject: "operator-1",
      tenantId: "01900000-0000-4000-8000-000000000001",
      roles: [],
      scopes: [],
      attributes: {},
      capabilities: {
        "refund.claim.create": {
          kind: "deny",
          reasonCode: "role_missing",
          reason: "This action needs an approved responsibility.",
        },
      },
    };
    render(<ProductShell
      workspace="quote"
      onNavigate={() => undefined}
      serviceState="online"
      accessToken="verified-session"
      authorizationSnapshot={snapshot}
      locale="en-US"
      timeZone="UTC"
      onLocaleChange={() => undefined}
      onTimeZoneChange={() => undefined}
    >
      <div>Workspace content</div>
    </ProductShell>);

    const item = screen.getByRole("button", { name: "Refund requests" });
    const contextId = item.getAttribute("aria-describedby");
    expect(contextId).toBeTruthy();
    expect(item.querySelector(".nav-policy-mark")).toBeNull();
    expect(item.querySelector(".nav-risk")).toBeNull();
    const context = screen.getByText(/Access: Not available\. This workspace needs an approved responsibility\./);
    expect(context.id).toBe(contextId);
    expect(context.className.split(/\s+/)).toContain("sr-only");
    expect(context.textContent).toMatch(/Sensitive financial action/);
    expect(item.getAttribute("title")).toMatch(/Sensitive financial action/);
  });

  it("lets experienced operators compact the desktop navigation without changing its accessible names", () => {
    render(<ProductShell
      workspace="quote"
      onNavigate={() => undefined}
      serviceState="online"
      accessToken=""
      locale="en-US"
      timeZone="UTC"
      onLocaleChange={() => undefined}
      onTimeZoneChange={() => undefined}
    >
      <div>Workspace content</div>
    </ProductShell>);

    const collapse = screen.getByRole("button", { name: "Collapse navigation" });
    expect(document.querySelector(".product-shell")?.classList.contains("navigation-compact")).toBe(false);
    fireEvent.click(collapse);

    expect(document.querySelector(".product-shell")?.classList.contains("navigation-compact")).toBe(true);
    expect(screen.getByRole("button", { name: "Expand navigation" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Tax quotes" })).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Tax quotes" })).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Expand navigation" }));
    expect(document.querySelector(".product-shell")?.classList.contains("navigation-compact")).toBe(false);
  });

  it("stays in the compact rail while the operator keeps working", () => {
    render(<ProductShell
      workspace="quote"
      onNavigate={() => undefined}
      serviceState="online"
      accessToken=""
      locale="en-US"
      timeZone="UTC"
      onLocaleChange={() => undefined}
      onTimeZoneChange={() => undefined}
    >
      <div>Workspace content</div>
    </ProductShell>);

    fireEvent.click(screen.getByRole("button", { name: "Collapse navigation" }));

    expect(document.querySelector(".product-shell.navigation-compact")).toBeTruthy();
    expect(document.querySelector(".product-shell.navigation-compact .nav-item-glyph")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Tax quotes" })).toBeTruthy();
  });

  it("turns the account control into a safe session menu", () => {
    const onSignOut = vi.fn();
    const snapshot: AuthorizationSnapshot = {
      state: "server-allowed",
      expiresAtUnix: testExpiryUnix,
      subject: "operator-verified",
      tenantId: "01900000-0000-4000-8000-000000000001",
      roles: ["tax_rule_reviewer"],
      scopes: ["audit:read"],
      attributes: {},
      capabilities: {},
    };

    render(<ProductShell
      workspace="quote"
      onNavigate={() => undefined}
      serviceState="online"
      accessToken="verified-session"
      authorizationSnapshot={snapshot}
      onSignOut={onSignOut}
      locale="en-US"
      timeZone="UTC"
      onLocaleChange={() => undefined}
      onTimeZoneChange={() => undefined}
    >
      <div>Workspace content</div>
    </ProductShell>);

    const accountMenu = screen.getByRole("button", { name: "Open account menu" });
    expect(accountMenu).toBeTruthy();
    expect(accountMenu.closest("details")?.hasAttribute("open")).toBe(false);

    fireEvent.click(accountMenu);

    expect(accountMenu.closest("details")?.hasAttribute("open")).toBe(true);
    expect(screen.getAllByText("Account 01900000").length).toBe(1);
    expect(screen.getByText("operator-verified")).toBeTruthy();
    expect(screen.getByRole("link", { name: "Review access & approvals" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Sign out" })).toBeTruthy();
    expect(accountMenu.closest("details")?.textContent).not.toContain("verified-session");

    fireEvent.click(screen.getByRole("button", { name: "Sign out" }));
    expect(onSignOut).toHaveBeenCalledOnce();
  });

  it("exposes a bounded theme preference without changing the shell authority boundary", () => {
    const onThemeChange = vi.fn();
    const snapshot: AuthorizationSnapshot = {
      state: "server-allowed",
      expiresAtUnix: testExpiryUnix,
      subject: "operator-verified",
      tenantId: "01900000-0000-4000-8000-000000000001",
      roles: [],
      scopes: [],
      attributes: {},
      capabilities: {},
    };

    render(<ProductShell
      workspace="quote"
      onNavigate={() => undefined}
      serviceState="online"
      accessToken="verified-session"
      authorizationSnapshot={snapshot}
      themePreference="light"
      onThemeChange={onThemeChange}
      locale="en-US"
      timeZone="UTC"
      onLocaleChange={() => undefined}
      onTimeZoneChange={() => undefined}
    >
      <div>Workspace content</div>
    </ProductShell>);

    const shell = document.querySelector(".product-shell");
    expect(shell?.getAttribute("data-theme")).toBe("light");
    fireEvent.click(screen.getByRole("button", { name: "Open display settings" }));
    const theme = screen.getByRole("combobox", { name: "Theme" });
    expect(theme).toBeTruthy();
    fireEvent.change(theme, { target: { value: "dark" } });
    expect(onThemeChange).toHaveBeenCalledWith("dark");
    expect(screen.getByText("Theme changes appearance only.")).toBeTruthy();
  });
});
