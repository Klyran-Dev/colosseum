// @vitest-environment happy-dom

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import ProductShell from "./ProductShell";
import { AuthorizationSnapshot } from "./authorization";
import { parseRuntimeContext } from "./runtimeContext";

afterEach(() => cleanup());

describe("operator-first product shell", () => {
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
    expect(screen.queryByText("Control plane")).toBeNull();
    expect(screen.queryByText("Move funds")).toBeNull();
  });

  it("keeps technical context out of the primary page title", () => {
    const snapshot: AuthorizationSnapshot = {
      state: "server-allowed",
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

    expect(screen.getByText("Account 01900000")).toBeTruthy();
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
    expect(screen.getByRole("alert").textContent).toMatch(/release build/i);
    expect(screen.queryByText("Staging")).toBeNull();
    expect(screen.queryByText("Test network")).toBeNull();
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

  it("shows server-owned access status beside high-consequence workspaces", () => {
    const snapshot: AuthorizationSnapshot = {
      state: "server-allowed",
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
});
