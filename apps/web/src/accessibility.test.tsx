// @vitest-environment happy-dom

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { run } from "axe-core";
import { afterEach, describe, expect, it, vi } from "vitest";
import App from "./App";
import CheckoutPanel from "./CheckoutPanel";
import DocumentCheckoutPanel from "./DocumentCheckoutPanel";
import DocumentQuotePanel from "./DocumentQuotePanel";
import GovernancePanel from "./GovernancePanel";
import ReplayOpsPanel from "./ReplayOpsPanel";
import VaultReconciliationPanel from "./VaultReconciliationPanel";
import VaultReleasePanel from "./VaultReleasePanel";
import SettlementAdjustmentPanel from "./SettlementAdjustmentPanel";
import RefundClaimPanel from "./RefundClaimPanel";
import RefundCancellationPanel from "./RefundCancellationPanel";
import RefundExecutePanel from "./RefundExecutePanel";
import EvidenceAccessPanel from "./EvidenceAccessPanel";

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
      <GovernancePanel key="governance" apiBase="/api" serviceOnline accessToken="" />,
      <ReplayOpsPanel key="replay" apiBase="/api" serviceOnline accessToken="" />,
      <VaultReconciliationPanel key="vault-reconciliation" apiBase="/api" serviceOnline accessToken="" locale="en-US" />,
      <VaultReleasePanel key="vault" apiBase="/api" serviceOnline accessToken="" />,
      <SettlementAdjustmentPanel key="adjustments" apiBase="/api" serviceOnline accessToken="" />,
      <RefundClaimPanel key="refund-claims" apiBase="/api" serviceOnline accessToken="" locale="en-US" />,
      <RefundCancellationPanel key="refund-cancellations" apiBase="/api" serviceOnline accessToken="" locale="en-US" />,
      <RefundExecutePanel key="refund-executions" apiBase="/api" serviceOnline accessToken="" locale="en-US" />,
      <EvidenceAccessPanel key="evidence-access" apiBase="/api" serviceOnline accessToken="" locale="en-US" />,
      <CheckoutPanel
        key="checkout"
        apiBase="/api"
        serviceOnline
        accessToken=""
        calculation={null}
        formatAtomic={(value, currency) => `${currency} ${value}`}
      />,
      <DocumentCheckoutPanel
        key="document-checkout"
        apiBase="/api"
        serviceOnline
        accessToken=""
      />,
      <DocumentQuotePanel
        key="document-quote"
        apiBase="/api"
        serviceOnline
        accessToken=""
        formatAtomic={(value, currency) => `${currency} ${value}`}
        onOpenCheckout={() => undefined}
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
    const governanceButton = screen.getByRole("button", { name: "Tax rules" });

    expect(skipLink.getAttribute("href")).toBe("#main-content");
    expect(governanceButton.getAttribute("aria-current")).toBeNull();

    fireEvent.click(governanceButton);

    expect(governanceButton.getAttribute("aria-current")).toBe("page");
    expect(document.activeElement).toBe(workspaceContent);
  });

  it("keeps browser history navigation aligned without pushing during popstate", async () => {
    window.history.replaceState(null, "", "/operations/tax-quotes");
    vi.stubGlobal("fetch", vi.fn(async () => new Response(null, { status: 200 })));
    const pushState = vi.spyOn(window.history, "pushState");

    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Tax rules" }));

    expect(window.location.pathname).toBe("/rules/tax-rules");
    expect(pushState).toHaveBeenCalledTimes(1);

    window.history.replaceState(null, "", "/money/balance-checks");
    window.dispatchEvent(new PopStateEvent("popstate"));

    await waitFor(() => expect(screen.getByRole("heading", { name: "Check tax balances" })).toBeTruthy());
    expect(window.location.pathname).toBe("/money/balance-checks");
    expect(pushState).toHaveBeenCalledTimes(1);
  });

  it("exposes explicit locale, time-zone, and RTL display settings", () => {
    vi.stubGlobal("fetch", vi.fn(async () => new Response(null, { status: 200 })));
    render(<App />);

    fireEvent.click(screen.getByText("Display settings"));
    fireEvent.change(screen.getByRole("combobox", { name: "Language" }), { target: { value: "ar-EG" } });
    expect(document.querySelector(".app-shell")?.getAttribute("dir")).toBe("rtl");
    expect(document.documentElement.lang).toBe("ar");

    const timeZone = screen.getByRole("combobox", { name: "المنطقة الزمنية" });
    fireEvent.change(timeZone, { target: { value: "Not/AZone" } });
    expect(timeZone.getAttribute("aria-invalid")).toBe("true");
    expect(screen.getByText("المنطقة الزمنية غير معروفة؛ يُستخدم UTC للعرض.")).toBeTruthy();
  });
});
