// @vitest-environment happy-dom

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { run } from "axe-core";
import { afterEach, describe, expect, it, vi } from "vitest";
import App from "./App";
import CheckoutPanel from "./CheckoutPanel";
import GovernancePanel from "./GovernancePanel";
import ReplayOpsPanel from "./ReplayOpsPanel";
import VaultReconciliationPanel from "./VaultReconciliationPanel";
import VaultReleasePanel from "./VaultReleasePanel";
import SettlementAdjustmentPanel from "./SettlementAdjustmentPanel";
import RefundClaimPanel from "./RefundClaimPanel";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe("primary product accessibility contract", () => {
  it("has no automated WCAG 2 A/AA violations on the quote workspace", async () => {
    vi.stubGlobal("fetch", vi.fn(async () => new Response(null, { status: 200 })));
    render(<App />);

    const result = await run(document.body, {
      runOnly: { type: "tag", values: ["wcag2a", "wcag2aa"] },
    });

    expect(result.violations).toEqual([]);
  });

  it("has no automated WCAG 2 A/AA violations across lazy operational workspaces", async () => {
    vi.stubGlobal("fetch", vi.fn(async () => new Response(null, { status: 200 })));

    const workspaces = [
      <GovernancePanel key="governance" apiBase="/api" serviceOnline />,
      <ReplayOpsPanel key="replay" apiBase="/api" serviceOnline />,
      <VaultReconciliationPanel key="vault-reconciliation" apiBase="/api" serviceOnline />,
      <VaultReleasePanel key="vault" apiBase="/api" serviceOnline accessToken="" />,
      <SettlementAdjustmentPanel key="adjustments" apiBase="/api" serviceOnline accessToken="" />,
      <RefundClaimPanel key="refund-claims" apiBase="/api" serviceOnline accessToken="" locale="en-US" />,
      <CheckoutPanel
        key="checkout"
        apiBase="/api"
        serviceOnline
        accessToken=""
        calculation={null}
        formatAtomic={(value, currency) => `${currency} ${value}`}
      />,
    ];

    for (const workspace of workspaces) {
      render(workspace);
      const result = await run(document.body, {
        runOnly: { type: "tag", values: ["wcag2a", "wcag2aa"] },
      });
      expect(result.violations).toEqual([]);
      cleanup();
    }
  });

  it("provides a skip target and moves focus after workspace navigation", () => {
    vi.stubGlobal("fetch", vi.fn(async () => new Response(null, { status: 200 })));
    render(<App />);

    const skipLink = screen.getByRole("link", { name: "Skip to workspace content" });
    const workspaceContent = screen.getByRole("region", { name: "Current tax workspace" });
    const governanceButton = screen.getByRole("button", { name: "Rule governance" });

    expect(skipLink.getAttribute("href")).toBe("#workspace-content");
    expect(governanceButton.getAttribute("aria-pressed")).toBe("false");

    fireEvent.click(governanceButton);

    expect(governanceButton.getAttribute("aria-pressed")).toBe("true");
    expect(document.activeElement).toBe(workspaceContent);
  });

  it("exposes explicit locale, time-zone, and RTL display settings", () => {
    vi.stubGlobal("fetch", vi.fn(async () => new Response(null, { status: 200 })));
    render(<App />);

    fireEvent.click(screen.getByText("Display settings"));
    fireEvent.change(screen.getByRole("combobox", { name: "Language" }), { target: { value: "ar-EG" } });
    expect(document.querySelector(".app-shell")?.getAttribute("dir")).toBe("rtl");
    expect(document.documentElement.lang).toBe("ar");

    const timeZone = screen.getByRole("combobox", { name: "Time zone" });
    fireEvent.change(timeZone, { target: { value: "Not/AZone" } });
    expect(timeZone.getAttribute("aria-invalid")).toBe("true");
    expect(screen.getByText("Unknown time zone; UTC is used for rendering.")).toBeTruthy();
  });
});
