// @vitest-environment happy-dom

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import ProductShell, { ProductShellProps } from "./ProductShell";
import { AuthorizationSnapshot } from "./authorization";
import { parseRuntimeContext } from "./runtimeContext";

afterEach(() => cleanup());

const baseProps: Omit<ProductShellProps, "children"> = {
  workspace: "quote",
  onNavigate: vi.fn(),
  serviceState: "online",
  accessToken: "",
  locale: "en-US",
  timeZone: "UTC",
  onLocaleChange: vi.fn(),
  onTimeZoneChange: vi.fn(),
};

function renderShell(overrides: Partial<ProductShellProps> = {}) {
  return render(
    <ProductShell {...baseProps} {...overrides}>
      <div>Workspace content</div>
    </ProductShell>,
  );
}

function allowedSnapshot(): AuthorizationSnapshot {
  return {
    state: "server-allowed",
    expiresAtUnix: Math.floor(Date.now() / 1000) + 60,
    subject: "operator-verified",
    email: "operator@example.com",
    tenantId: "01900000-0000-7000-8000-000000000001",
    roles: ["tenant_admin"],
    scopes: ["access.control"],
    attributes: { jurisdiction: "US-CA" },
    capabilities: {
      "tax.quote.create": { kind: "allow", reasonCode: "scope_and_role_satisfied", reason: "Confirmed." },
    },
  };
}

describe("reference-driven control-plane shell", () => {
  it("renders the expanded dashboard composition and stable landmarks", () => {
    renderShell();

    const shell = document.querySelector(".control-plane-shell");
    expect(shell?.getAttribute("data-layout")).toBe("control-plane");
    expect(shell?.classList.contains("navigation-compact")).toBe(false);
    expect(document.querySelector(".control-sidebar")).toBeTruthy();
    expect(document.querySelector(".control-topbar")).toBeTruthy();
    expect(document.querySelector(".control-context-nav")).toBeTruthy();
    expect(document.querySelector(".workspace-status-deck")).toBeTruthy();
    expect(document.querySelector(".dashboard-insights")).toBeTruthy();
    expect(document.getElementById("main-content")?.tagName).toBe("MAIN");
  });

  it("renders reference-style analytics surfaces without fabricating business data", () => {
    renderShell();

    expect(screen.getByRole("heading", { name: "Policy decision flow" })).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Access distribution" })).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Recent verified actions" })).toBeTruthy();
    expect(screen.getByText("No verified activity is available for this session.")).toBeTruthy();
    expect(screen.getByText("Counts are derived from the server authorization snapshot.")).toBeTruthy();
  });

  it("renders access distribution as an accessible semantic SVG ring", () => {
    renderShell();

    const distribution = screen.getByRole("img", { name: "1 confirmed, 0 review, 15 blocked" });
    expect(distribution.querySelector("svg")).toBeTruthy();
    expect(distribution.querySelector(".dashboard-donut-confirmed")).toBeTruthy();
    expect(distribution.querySelector(".dashboard-donut-review")).toBeTruthy();
    expect(distribution.querySelector(".dashboard-donut-blocked")).toBeTruthy();
    expect(distribution.querySelector("[style*='gradient']")).toBeNull();
  });

  it("keeps every workspace reachable with truthful access labels", () => {
    renderShell();

    expect(screen.getByRole("button", { name: "Tax quotes" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Tax simulator" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Tax rules" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Treasury" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Evidence & audit" })).toBeTruthy();
    expect(screen.getAllByLabelText("Access denied").length).toBeGreaterThan(0);
    expect(screen.getByLabelText("Access confirmed")).toBeTruthy();
    expect(screen.getByText("Automated withholding and evidence—not tax filing or legal advice.")).toBeTruthy();
  });

  it("connects the keyboard skip link to the workspace main landmark", () => {
    renderShell({ accessToken: "signed-token", authorizationSnapshot: allowedSnapshot() });

    expect(screen.getByRole("link", { name: "Skip to workspace content" }).getAttribute("href")).toBe("#main-content");
    expect(document.getElementById("main-content")?.getAttribute("tabindex")).toBe("-1");
  });

  it("uses localized labels for the navigation and workspace heading", () => {
    renderShell({ locale: "de-DE" });

    expect(screen.getByRole("button", { name: "Steuerangebote" })).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Steuerangebote" })).toBeTruthy();
    expect(screen.getAllByText("Tägliche Arbeit").length).toBeGreaterThan(0);
    expect(screen.getByRole("status").textContent).toContain("Steuerangebote");
  });

  it("shows the server-owned RBAC, ABAC, and PBAC posture after authorization", () => {
    renderShell({ workspace: "access-control", accessToken: "verified-session", authorizationSnapshot: allowedSnapshot(), runtimeContext: parseRuntimeContext({ VITE_DEPLOYMENT_ENVIRONMENT: "staging", VITE_SOLANA_NETWORK: "devnet" }) });

    expect(screen.getByRole("heading", { name: "Access & approvals" })).toBeTruthy();
    expect(screen.getByText("RBAC · ABAC · PBAC")).toBeTruthy();
    expect(screen.getAllByText("Access checked").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Staging").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Solana Devnet").length).toBeGreaterThan(0);
    expect(screen.queryByText("Move funds")).toBeNull();
  });

  it("fails closed for a protected workspace when signed out", () => {
    renderShell({ workspace: "quote", serviceState: "offline" });

    expect(screen.getByRole("heading", { name: "Sign in to continue" })).toBeTruthy();
    expect(screen.getByText("Tenant, role, attributes, and policy are checked by the service.")).toBeTruthy();
    expect(screen.getByText("Protected actions remain unavailable.")).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Calculate quote" })).toBeNull();
  });

  it("exposes display settings without allowing presentation controls to change authorization", () => {
    const onLocaleChange = vi.fn();
    const onTimeZoneChange = vi.fn();
    renderShell({ onLocaleChange, onTimeZoneChange });

    fireEvent.click(screen.getByText("Display settings"));
    fireEvent.change(screen.getByRole("combobox", { name: "Language" }), { target: { value: "ar-EG" } });
    const timeZone = screen.getByDisplayValue("UTC");
    fireEvent.change(timeZone, { target: { value: "Not/AZone" } });

    expect(onLocaleChange).toHaveBeenCalledWith("ar-EG");
    expect(onTimeZoneChange).toHaveBeenCalledWith("Not/AZone");
    expect(timeZone).toBeTruthy();
  });

  it("supports compact mode while retaining accessible navigation names", () => {
    renderShell();

    fireEvent.click(screen.getByRole("button", { name: "Collapse navigation" }));
    expect(document.querySelector(".control-plane-shell")?.classList.contains("navigation-compact")).toBe(true);
    expect(screen.getByRole("button", { name: "Tax quotes" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Expand navigation" })).toBeTruthy();
  });

  it("keeps the account menu scoped to the verified server snapshot", () => {
    renderShell({ accessToken: "verified-session", authorizationSnapshot: allowedSnapshot() });

    fireEvent.click(screen.getByRole("button", { name: "Open account menu" }));
    expect(screen.getAllByText("operator@example.com").length).toBeGreaterThan(0);
    expect(screen.getByText("tenant admin")).toBeTruthy();
    expect(screen.getByText("Review access & approvals")).toBeTruthy();
  });
});
