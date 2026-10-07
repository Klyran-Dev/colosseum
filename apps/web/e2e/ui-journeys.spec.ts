import { expect, test, type Page } from "@playwright/test";
import { Buffer } from "node:buffer";
import axe from "axe-core";
import { PublicKey } from "@solana/web3.js";
import { documentQuoteMessagesFor } from "../src/documentQuoteMessages";
import { attachStyleSnapshots, captureStyleSnapshot } from "./styleSnapshot";

attachStyleSnapshots(test);

async function expectNoBrowserAxeViolations(
  page: Page,
  surface: string,
): Promise<void> {
  await captureStyleSnapshot(page, test.info().title, surface);
  const violations = await page.evaluate(async (axeSource) => {
    if (!("axe" in window)) {
      const script = document.createElement("script");
      script.textContent = axeSource;
      document.head.appendChild(script);
    }
    const browserAxe = (window as unknown as {
      axe?: {
        run: (context: Document, options: { runOnly: { type: "tag"; values: string[] } }) => Promise<{
          violations: Array<{
            id: string;
            impact: string | null;
            help: string;
            nodes: Array<{ target: string[]; html: string }>;
          }>;
        }>;
      };
    }).axe;
    if (!browserAxe) throw new Error("axe-core did not initialize in the real browser");
    const result = await browserAxe.run(document, {
      runOnly: { type: "tag", values: ["wcag2a", "wcag2aa"] },
    });
    return result.violations.map(({ id, impact, help, nodes }) => ({
      id,
      impact,
      help,
      nodes: nodes.map(({ target, html }) => ({ target, html })),
    }));
  }, axe.source);

  expect(violations, `${surface} has real-browser axe violations`).toEqual([]);
}

const CAPABILITY_IDS = [
  "access.control",
  "evidence.applicability.read",
  "evidence.replay",
  "evidence.audit.read",
  "revenue_stream.claim.prepare",
  "revenue_stream.govern",
  "revenue_stream.propose",
  "revenue_stream.read",
  "refund.claim.cancel",
  "refund.claim.create",
  "refund.payment.release",
  "settlement.adjustment",
  "settlement.document.wallet",
  "settlement.wallet",
  "tax.document.quote.create",
  "tax.preview.run",
  "tax.quote.create",
  "tax.applicability.govern",
  "tax.rule.govern",
  "vault.reconciliation",
  "vault.release",
] as const;

function completeCapabilityCatalog(overrides: Record<string, { decision: "allow" | "deny" | "indeterminate"; reason_code: string }>) {
  return Object.fromEntries(CAPABILITY_IDS.map((id) => [
    id,
    overrides[id] ?? { decision: "deny", reason_code: "scope_missing" },
  ]));
}

async function fillLocalCredential(page: Page, value: string): Promise<void> {
  const field = page.getByLabel(/Sign-in credential \(held in memory\)|Anmeldeinformation \(im Speicher gehalten\)/);
  if (!(await field.isVisible())) {
    await page.getByRole("button", { name: /Open local sign-in|Lokale Anmeldung öffnen/ }).click();
  }
  await field.fill(value);
}

async function openLocationVerification(page: Page): Promise<void> {
  const details = page.locator("details.route-evidence").filter({ hasText: "Location verification" }).first();
  if (await details.getAttribute("open") === null) {
    await details.locator("summary").click();
  }
  await expect(details).toHaveAttribute("open", "");
}

async function openSystemStatus(page: Page): Promise<void> {
  const details = page.locator("details.system-status");
  if (await details.getAttribute("open") === null) {
    await details.locator("summary").click();
  }
  await expect(details).toHaveAttribute("open", "");
  await expect(details.getByText("All systems operational", { exact: true })).toBeVisible();
}

const NAVIGATION_GROUPS: Record<string, string> = {
  "Tax quotes": "Daily work",
  "Tax simulator": "Daily work",
  "Invoice tax": "Daily work",
  "Payment review": "Daily work",
  "Document payment": "Daily work",
  "Tax rules": "Rules & access",
  "Access & approvals": "Rules & access",
  "Treasury": "Money movement",
  "Balance checks": "Money movement",
  "Partner payouts": "Money movement",
  "Adjustments": "Money movement",
  "Refund requests": "Money movement",
  "Refund cancellations": "Money movement",
  "Refund payments": "Money movement",
  "Exceptions & recovery": "Records & issues",
  "Evidence & audit": "Records & issues",
};

async function openNavigationGroupFor(page: Page, workspace: string): Promise<void> {
  const item = page.getByRole("button", { name: workspace, exact: true });
  if (await item.isVisible()) return;
  const group = NAVIGATION_GROUPS[workspace];
  if (!group) throw new Error(`No navigation group is registered for ${workspace}`);
  const details = page.locator("details.nav-group-disclosure").filter({ hasText: group }).first();
  if (await details.getAttribute("open") === null) await details.locator("summary").click();
  await expect(item).toBeVisible();
}

test.describe("real-browser SPA journeys", () => {
  test("supports direct workspace links and browser history", async ({ page }) => {
    await page.goto("/operations/tax-quotes");
    await expect(page.getByRole("status")).toHaveText("Tax quotes workspace");
    const aggregateLocationVerification = page.locator("details.route-evidence").filter({ hasText: "Location verification" }).first();
    await expect(aggregateLocationVerification).not.toHaveAttribute("open");

    await page.goto("/money/refund-payments");
    await expect(page).toHaveURL(/\/money\/refund-payments$/);
    await expect(page.getByRole("heading", { name: "Prepare refund release", exact: true })).toBeVisible();

    await openNavigationGroupFor(page, "Balance checks");
    await page.getByRole("button", { name: "Balance checks", exact: true }).click();
    await expect(page).toHaveURL(/\/money\/balance-checks$/);
    await expect(page.getByRole("heading", { name: "Check tax balances", exact: true })).toBeVisible();
    await expect(page.getByRole("status")).toHaveText("Balance checks workspace");

    await page.goBack();
    await expect(page).toHaveURL(/\/money\/refund-payments$/);
    await expect(page.getByRole("heading", { name: "Prepare refund release", exact: true })).toBeVisible();
    await page.goForward();
    await expect(page).toHaveURL(/\/money\/balance-checks$/);
    await expectNoBrowserAxeViolations(page, "addressable workspace navigation");
  });

  test("keeps one navigation group open around the current work area", async ({ page }) => {
    await page.goto("/");

    const dailyWork = page.locator("details.nav-group-disclosure").filter({ hasText: "Daily work" }).first();
    const rulesAndAccess = page.locator("details.nav-group-disclosure").filter({ hasText: "Rules & access" }).first();
    await expect(dailyWork).toHaveAttribute("open", "");
    await expect(rulesAndAccess).not.toHaveAttribute("open", "");

    await rulesAndAccess.locator("summary").click();
    await expect(rulesAndAccess).toHaveAttribute("open", "");
    await expect(dailyWork).not.toHaveAttribute("open", "");
    await expectNoBrowserAxeViolations(page, "single-open navigation group");
  });

  test("lets operators compact the desktop navigation without hiding workspace names from assistive technology", async ({ page }) => {
    await page.goto("/");

    const shell = page.locator(".product-shell");
    const collapse = page.getByRole("button", { name: "Collapse navigation", exact: true });
    await expect(collapse).toBeVisible();
    await collapse.click();

    await expect(shell).toHaveClass(/navigation-compact/);
    await expect(page.getByRole("button", { name: "Expand navigation", exact: true })).toBeVisible();
    await expect(page.getByRole("button", { name: "Tax quotes", exact: true })).toBeVisible();
    await expect(page.getByRole("heading", { name: "Tax quotes", exact: true })).toBeVisible();

    await page.getByRole("button", { name: "Expand navigation", exact: true }).click();
    await expect(shell).not.toHaveClass(/navigation-compact/);
    await expectNoBrowserAxeViolations(page, "compact navigation rail");
  });

  test("preserves semantic system roles in forced-colors mode", async ({ page }) => {
    await page.emulateMedia({ forcedColors: "active" });
    await page.goto("/");

    await expect.poll(() => page.evaluate(() => window.matchMedia("(forced-colors: active)").matches)).toBe(true);
    const roles = await page.locator(".product-shell").evaluate((element) => {
      const style = getComputedStyle(element);
      return {
        background: style.getPropertyValue("--app-bg").trim(),
        foreground: style.getPropertyValue("--app-on-surface").trim(),
        primary: style.getPropertyValue("--app-primary").trim(),
        onPrimary: style.getPropertyValue("--app-on-primary").trim(),
      };
    });
    expect(roles).toEqual({
      background: "Canvas",
      foreground: "CanvasText",
      primary: "Highlight",
      onPrimary: "HighlightText",
    });
    await page.getByRole("button", { name: "Tax quotes", exact: true }).focus();
    await expectNoBrowserAxeViolations(page, "forced-colors presentation");
  });

  test("supports keyboard context and real lazy workspace loading", async ({ page }) => {
    await page.goto("/");
    await openSystemStatus(page);
    await expect(page.getByText("Local development", { exact: true })).toBeVisible();
    await expect(page.getByText("Local Solana ledger", { exact: true })).toBeVisible();
    await expect(page.getByText("Acme Treasury", { exact: true })).toHaveCount(0);
    await expect(page.getByText("Test network", { exact: true })).toHaveCount(0);
    await page.locator("details.system-status summary").click();
    await expect(page.locator("details.system-status")).not.toHaveAttribute("open", "");

    await page.getByRole("button", { name: "Open display settings" }).click();
    await page.getByRole("combobox", { name: "Theme" }).selectOption("light");
    await expect(page.locator(".product-shell")).toHaveAttribute("data-theme", "light");
    await page.getByRole("combobox", { name: "Theme" }).selectOption("system");
    await expect(page.locator(".product-shell")).toHaveAttribute("data-theme", "system");
    await page.reload();

    await page.keyboard.press("Tab");
    const skipLink = page.getByRole("link", { name: "Skip to workspace content" });
    await expect(skipLink).toBeFocused();
    await page.keyboard.press("Enter");
    await expect(page.locator("#main-content")).toBeFocused();

    const workspaces = [
      ["Tax rules", "Review and publish a rule"],
      ["Exceptions & recovery", "Recovery queue"],
      ["Treasury", "Create a release request"],
      ["Balance checks", "Check tax balances"],
      ["Adjustments", "Create an adjustment request"],
      ["Refund requests", "Prepare a refund request"],
      ["Refund payments", "Prepare refund release"],
      ["Payment review", "Review payment before signing"],
      ["Invoice tax", "Calculate tax for an invoice"],
      ["Document payment", "Review and authorize this payment"],
      ["Evidence & audit", "Review evidence access"],
    ] as const;

    for (const [buttonName, headingName] of workspaces) {
      await openNavigationGroupFor(page, buttonName);
      await page.getByRole("button", { name: buttonName, exact: true }).click();
      await expect(page.getByRole("heading", { name: headingName, exact: true })).toBeVisible();
      await expect(page.locator("#workspace-content")).toBeFocused();
    }
  });

  test("keeps system status disclosure keyboard operable", async ({ page }) => {
    await page.goto("/");
    const details = page.locator("details.system-status");
    const summary = details.locator("summary");

    await summary.focus();
    await expect(summary).toBeFocused();
    await page.keyboard.press("Enter");
    await expect(details).toHaveAttribute("open", "");
    await expect(page.getByText("Deployment environment", { exact: true })).toBeVisible();

    await page.keyboard.press("Space");
    await expect(details).not.toHaveAttribute("open", "");
  });

  test("keeps expanded shell controls from widening or clipping the workspace", async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto("/");

    const localSignIn = page.getByRole("button", { name: "Open local sign-in", exact: true });
    await expect(localSignIn).toBeVisible();
    await localSignIn.click();

    const localPopoverGeometry = await page.evaluate(() => {
      const summary = document.querySelector<HTMLElement>(".session-control-local > summary")?.getBoundingClientRect();
      const fields = document.querySelector<HTMLElement>(".session-control-local-fields")?.getBoundingClientRect();
      return {
        summaryBottom: summary?.bottom ?? 0,
        fieldsTop: fields?.top ?? 0,
      };
    });
    expect(localPopoverGeometry.fieldsTop).toBeGreaterThanOrEqual(localPopoverGeometry.summaryBottom - 1);

    await page.getByRole("button", { name: "Open display settings", exact: true }).click();

    const geometry = await page.evaluate(() => {
      const viewportWidth = document.documentElement.clientWidth;
      const shell = document.querySelector<HTMLElement>(".product-shell")?.getBoundingClientRect();
      const mainColumn = document.querySelector<HTMLElement>(".app-main-column")?.getBoundingClientRect();
      const topbar = document.querySelector<HTMLElement>(".product-topbar")?.getBoundingClientRect();
      const breadcrumb = document.querySelector<HTMLElement>(".breadcrumb")?.getBoundingClientRect();
      return {
        viewportWidth,
        documentWidth: document.documentElement.scrollWidth,
        shellRight: shell?.right ?? 0,
        mainColumnRight: mainColumn?.right ?? 0,
        topbarRight: topbar?.right ?? 0,
        breadcrumbWidth: breadcrumb?.width ?? 0,
        localOpen: document.querySelector<HTMLElement>(".session-control-local")?.matches("[open]") ?? false,
      };
    });

    expect(geometry.documentWidth).toBeLessThanOrEqual(geometry.viewportWidth + 1);
    expect(geometry.shellRight).toBeLessThanOrEqual(geometry.viewportWidth + 1);
    expect(geometry.mainColumnRight).toBeLessThanOrEqual(geometry.viewportWidth + 1);
    expect(geometry.topbarRight).toBeLessThanOrEqual(geometry.mainColumnRight + 1);
    expect(geometry.breadcrumbWidth).toBeGreaterThan(80);
    expect(geometry.localOpen).toBe(false);
  });

  test("keeps mobile shell actions in a compact Material action row", async ({ page }) => {
    await page.setViewportSize({ width: 568, height: 900 });
    await page.goto("/");

    const geometry = await page.evaluate(() => {
      const topbar = document.querySelector<HTMLElement>(".product-topbar");
      const actions = document.querySelector<HTMLElement>(".product-topbar .topbar-actions");
      const breadcrumb = document.querySelector<HTMLElement>(".product-topbar .breadcrumb");
      const topbarRect = topbar?.getBoundingClientRect();
      const actionsRect = actions?.getBoundingClientRect();
      return {
        topbarDisplay: topbar ? getComputedStyle(topbar).display : "",
        actionsDirection: actions ? getComputedStyle(actions).flexDirection : "",
        actionsWrap: actions ? getComputedStyle(actions).flexWrap : "",
        topbarHeight: topbarRect?.height ?? 0,
        actionsHeight: actionsRect?.height ?? 0,
        breadcrumbHeight: breadcrumb?.getBoundingClientRect().height ?? 0,
      };
    });

    expect(geometry.topbarDisplay).toBe("grid");
    expect(geometry.actionsDirection).toBe("row");
    expect(geometry.actionsWrap).toBe("wrap");
    expect(geometry.breadcrumbHeight).toBeGreaterThan(0);
    expect(geometry.actionsHeight).toBeLessThan(96);
    expect(geometry.topbarHeight).toBeLessThan(180);
  });

  test("loads route-verification evidence only after an explicit operator action", async ({ page }) => {
    let evidenceRequests = 0;
    await page.route("**/api/v1/authorization/capabilities", (route) => route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        schema: "authorization-capabilities/v1",
        snapshot_id: "01900000-0000-7000-8000-000000000001",
        subject: "indexer-operator",
        tenant_id: "01900000-0000-4000-8000-000000000001",
        expires_at_unix: Math.floor(Date.now() / 1000) + 30,
        capabilities: completeCapabilityCatalog({
          "access.control": { decision: "allow", reason_code: "scope_and_role_satisfied" },
          "evidence.replay": { decision: "allow", reason_code: "scope_and_role_satisfied" },
        }),
      }),
    }));
    await page.route("**/api/v1/indexer/route-verification-runs*", async (route) => {
      evidenceRequests += 1;
      const authorized = (route.request().headers().authorization ?? "").includes("indexer-operator-session");
      await route.fulfill({
        status: authorized ? 200 : 403,
        contentType: "application/json",
        body: authorized ? JSON.stringify([{
          id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d25",
          run_kind: "backfill",
          status: "completed",
          batch_size: 8,
          examined_count: 2,
          activated_count: 1,
          already_active_count: 1,
          failed_count: 0,
          failure_code: null,
          operator_approval_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d26",
          started_at: "2026-10-05T10:00:00.000Z",
          completed_at: "2026-10-05T10:00:01.000Z",
        }]) : JSON.stringify({ code: "FORBIDDEN", message: "operator role required" }),
      });
    });

    await page.goto("/records/exceptions-recovery");
    await fillLocalCredential(page, "indexer-operator-session");
    await expect(page.getByRole("heading", { name: "Recovery queue", exact: true })).toBeVisible();
    const evidence = page.locator("details.replay-route-verification");
    await expect(evidence).not.toHaveAttribute("open", "");
    expect(evidenceRequests).toBe(0);

    await evidence.locator("summary").click();
    await evidence.getByRole("button", { name: "Load route-verification evidence", exact: true }).click();
    await expect(evidence.getByText("Verification completed", { exact: true })).toBeVisible();
    await expect(evidence.getByText("does not activate a route or confirm finalized settlement.", { exact: false })).toBeVisible();
    await evidence.getByText("Evidence reference", { exact: true }).click();
    await expect(evidence.getByText("01992f1b-4a70-7e30-8ea2-b6e37a9d9d26", { exact: true })).toBeVisible();
    expect(evidenceRequests).toBe(1);
    await expectNoBrowserAxeViolations(page, "route-verification operator evidence");
  });

  test("keeps protection assurances in one compact keyboard disclosure", async ({ page }) => {
    await page.goto("/");
    const details = page.locator("details.protection-details");
    const summary = details.locator("summary");

    await expect(details).not.toHaveAttribute("open", "");
    await summary.focus();
    await page.keyboard.press("Enter");
    await expect(details).toHaveAttribute("open", "");
    await expect(details.getByText("Account access", { exact: true })).toBeVisible();
    await expect(details.getByText("Every decision has a record.", { exact: true })).toBeVisible();
    await expect(details.getByText("Sensitive actions need approval.", { exact: true })).toBeVisible();
    await expect(details.getByText("Wallets stay with their owners.", { exact: true })).toBeVisible();
    await expect(page.locator(".security-strip")).toHaveCount(0);
    await expectNoBrowserAxeViolations(page, "compact protection disclosure");
  });

  test("provides account context and a safe sign-out path in the real shell", async ({ page }) => {
    await page.route("**/api/v1/authorization/capabilities", async (route) => {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          schema: "authorization-capabilities/v1",
          snapshot_id: "01900000-0000-7000-8000-000000000001",
          subject: "operator-a",
          tenant_id: "01900000-0000-4000-8000-000000000001",
          expires_at_unix: Math.floor(Date.now() / 1000) + 30,
          capabilities: completeCapabilityCatalog({
            "access.control": { decision: "allow", reason_code: "scope_and_role_satisfied" },
          }),
        }),
      });
    });

    await page.goto("/");
    const credential = page.getByLabel("Sign-in credential (held in memory)");
    await fillLocalCredential(page, "tenant-a-session");
    const accountMenu = page.locator("details.account-menu");
    await page.getByRole("button", { name: "Open account menu" }).click();
    await expect(accountMenu.getByText("Account 01900000", { exact: true })).toBeVisible();
    await expect(accountMenu.getByText("operator-a", { exact: true })).toBeVisible();
    await expect(accountMenu.getByRole("link", { name: "Review access & approvals" })).toBeVisible();
    await expect(accountMenu.getByRole("button", { name: "Sign out" })).toBeVisible();

    await accountMenu.getByRole("button", { name: "Sign out" }).click();
    await expect(credential).toHaveValue("");
    await page.getByRole("button", { name: "Open account menu" }).click();
    await expect(accountMenu.getByText("Sign in to manage this account.", { exact: true })).toBeVisible();
    await expect(accountMenu.getByRole("button", { name: "Sign out" })).toHaveCount(0);
    await expectNoBrowserAxeViolations(page, "account and session menu");
  });

  test("keeps protected actions unavailable when the server capability snapshot is expired", async ({ page }) => {
    await page.route("**/api/v1/authorization/capabilities", async (route) => {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          schema: "authorization-capabilities/v1",
          snapshot_id: "01900000-0000-7000-8000-000000000003",
          subject: "operator-expired",
          tenant_id: "01900000-0000-4000-8000-000000000003",
          expires_at_unix: Math.floor(Date.now() / 1000) - 1,
          capabilities: completeCapabilityCatalog({
            "access.control": { decision: "allow", reason_code: "scope_and_role_satisfied" },
          }),
        }),
      });
    });

    await page.goto("/");
    await fillLocalCredential(page, "expired-capability-session");
    const productNavigation = page.getByRole("complementary", { name: "Product navigation" });
    await expect(productNavigation.getByText("Access check unavailable", { exact: true })).toBeVisible();
    await expect(page.locator(".page-context-card").getByText("Your access check is out of date. We are refreshing access before you continue.", { exact: true })).toBeVisible();
    await openNavigationGroupFor(page, "Tax simulator");
    await expect(page.getByRole("button", { name: "Tax simulator", exact: true })).toHaveAttribute("title", /Needs access check/);
    await expectNoBrowserAxeViolations(page, "expired capability snapshot");
  });

  test("replaces the verified tenant context when the opaque session changes", async ({ page }) => {
    const tenantA = {
      token: "tenant-a-session",
      subject: "operator-a",
      tenantId: "01900000-0000-4000-8000-000000000001",
      snapshotId: "01900000-0000-7000-8000-000000000001",
      capabilities: completeCapabilityCatalog({
        "access.control": { decision: "allow", reason_code: "scope_and_role_satisfied" },
        "tax.preview.run": { decision: "allow", reason_code: "scope_and_role_satisfied" },
      }),
    };
    const tenantB = {
      token: "tenant-b-session",
      subject: "operator-b",
      tenantId: "02900000-0000-4000-8000-000000000002",
      snapshotId: "02900000-0000-7000-8000-000000000002",
      capabilities: completeCapabilityCatalog({
        "access.control": { decision: "allow", reason_code: "scope_and_role_satisfied" },
        "tax.preview.run": { decision: "deny", reason_code: "scope_missing" },
      }),
    };
    await page.route("**/api/v1/authorization/capabilities", async (route) => {
      const authorization = route.request().headers().authorization ?? "";
      const selected = authorization.includes(tenantB.token) ? tenantB : tenantA;
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          schema: "authorization-capabilities/v1",
          snapshot_id: selected.snapshotId,
          subject: selected.subject,
          tenant_id: selected.tenantId,
          expires_at_unix: Math.floor(Date.now() / 1000) + 30,
          capabilities: selected.capabilities,
        }),
      });
    });

    await page.goto("/");
    await openSystemStatus(page);
    await fillLocalCredential(page, tenantA.token);
    const accountMenu = page.getByRole("button", { name: "Open account menu" });
    await accountMenu.click();
    const accountPanel = page.locator(".account-menu-panel");
    await expect(accountPanel.getByText("Account 01900000", { exact: true })).toBeVisible();
    await openNavigationGroupFor(page, "Access & approvals");
    await page.getByRole("button", { name: "Access & approvals", exact: true }).click();
    await expect(page.getByRole("heading", { name: "Who can do what", exact: true })).toBeVisible();
    const workspaceContent = page.getByRole("region", { name: "Current tax workspace" });
    await expect(workspaceContent.getByText("operator-a", { exact: true })).toBeVisible();
    await openNavigationGroupFor(page, "Tax simulator");
    await expect(page.getByRole("button", { name: "Tax simulator", exact: true })).toHaveAttribute("title", /Available/);

    await fillLocalCredential(page, tenantB.token);
    await accountMenu.click();
    await expect(accountPanel.getByText("Account 02900000", { exact: true })).toBeVisible();
    await expect(accountPanel.getByText("Account 01900000", { exact: true })).toHaveCount(0);
    await expect(workspaceContent.getByText("operator-b", { exact: true })).toBeVisible();
    await expect(page.getByRole("button", { name: "Tax simulator", exact: true })).toHaveAttribute("title", /Not available/);
    await expectNoBrowserAxeViolations(page, "multi-tenant capability session reset");
  });

  test("supports the governed partner payout review flow without claiming a transfer", async ({ page }) => {
    const plan = {
      id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d23",
      merchant_id: "01900000-0000-4000-8000-000000000001",
      idempotency_key: "plan-fixture",
      request_digest: Array(32).fill(1),
      spec: {
        id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d23",
        merchant_id: "01900000-0000-4000-8000-000000000001",
        settlement_route_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d25",
        operator_subject: "payout-operator-a",
        asset: Array.from({ length: 32 }, (_, index) => index + 1),
        recipient: Array.from({ length: 32 }, (_, index) => 32 - index),
        plan_digest: Array(32).fill(3),
        policy_digest: Array(32).fill(4),
        start_slot: "100",
        end_slot: "200",
        max_total_atomic: "10000",
        max_claim_atomic: "1000",
      },
      status: "active",
      proposer_subject: "operator-a",
      approver_subject: "approver-a",
      accrued_total_atomic: "2500",
      released_total_atomic: "500",
      next_nonce: "2",
    };
    const capabilities = completeCapabilityCatalog({
      "revenue_stream.read": { decision: "allow", reason_code: "scope_and_role_satisfied" },
      "revenue_stream.propose": { decision: "allow", reason_code: "scope_and_role_satisfied" },
      "revenue_stream.govern": { decision: "allow", reason_code: "scope_and_role_satisfied" },
      "revenue_stream.claim.prepare": { decision: "allow", reason_code: "scope_and_role_satisfied" },
    });
    await page.route("**/api/v1/authorization/capabilities", (route) => route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        schema: "authorization-capabilities/v1",
        snapshot_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d24",
        subject: "operator-a",
        tenant_id: "01900000-0000-4000-8000-000000000001",
        expires_at_unix: Math.floor(Date.now() / 1000) + 30,
        capabilities,
      }),
    }));
    await page.route("**/api/v1/revenue-streaming/plans", (route) => route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify([plan]),
    }));

    await page.goto("/money/partner-payouts");
    await fillLocalCredential(page, "partner-payout-operator");
    await expect(page.getByRole("heading", { name: "Partner payouts", exact: true })).toBeVisible();
    await expect(page.getByText("Receiving accruals", { exact: true })).toBeVisible();
    await expect(page.getByText("Prepare a payout", { exact: true })).toBeVisible();
    await expect(page.getByText(/does not move money/i)).toBeVisible();
    await expect(page.getByText(/finalized Solana evidence/i)).toBeVisible();
    await expectNoBrowserAxeViolations(page, "partner payout review flow");
  });

  test("creates a partner payout plan from server-owned references", async ({ page }) => {
    const destinationId = "01992f1b-4a70-7e30-8ea2-b6e37a9d9d31";
    const splitPlanId = "01992f1b-4a70-7e30-8ea2-b6e37a9d9d32";
    const policyId = "01992f1b-4a70-7e30-8ea2-b6e37a9d9d33";
    const publicKey = "11111111111111111111111111111111";
    let proposalBody: Record<string, unknown> | null = null;
    const capabilities = completeCapabilityCatalog({
      "revenue_stream.read": { decision: "allow", reason_code: "scope_and_role_satisfied" },
      "revenue_stream.propose": { decision: "allow", reason_code: "scope_and_role_satisfied" },
    });

    await page.route("**/api/v1/authorization/capabilities", (route) => route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        schema: "authorization-capabilities/v1",
        snapshot_id: "01992f1b-0000-7000-8000-000000000031",
        subject: "payout-proposer",
        tenant_id: "01992f1b-0000-4000-8000-000000000031",
        expires_at_unix: Math.floor(Date.now() / 1000) + 30,
        capabilities,
      }),
    }));
    await page.route("**/api/v1/revenue-streaming/references", (route) => route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        destinations: [{
          id: destinationId,
          route_key: "partner-primary",
          version: 1,
          recipient_owner: publicKey,
          mint: publicKey,
          token_program: publicKey,
          verification_slot: "42",
        }],
        split_plans: [{ id: splitPlanId, plan_key: "partner-share", version: 2, mint: publicKey }],
        policies: [{
          id: policyId,
          rule_set_id: "01992f1b-0000-7000-8000-000000000034",
          rule_external_id: "india-goods",
          rule_version: 4,
          jurisdiction: "IN",
          regime: "GST",
          effective_from: "2026-01-01T00:00:00.000Z",
          effective_until: null,
        }],
      }),
    }));
    await page.route("**/api/v1/revenue-streaming/plans", async (route) => {
      if (route.request().method() === "GET") {
        await route.fulfill({ status: 200, contentType: "application/json", body: "[]" });
        return;
      }
      proposalBody = JSON.parse(route.request().postData() ?? "{}") as Record<string, unknown>;
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d34",
          status: "proposed",
          proposer_subject: "payout-proposer",
          approver_subject: null,
          accrued_total_atomic: "0",
          released_total_atomic: "0",
          next_nonce: "1",
          destination_route_id: destinationId,
          spec: {
            operator_subject: "payout-operator",
            start_slot: "100",
            end_slot: null,
            max_total_atomic: "1000",
            max_claim_atomic: "100",
            settlement_route_id: "01992f1b-0000-7000-8000-000000000035",
            asset: Array(32).fill(3),
            recipient: Array(32).fill(4),
            plan_digest: Array(32).fill(1),
            policy_digest: Array(32).fill(2),
          },
        }),
      });
    });

    await page.goto("/money/partner-payouts");
    await fillLocalCredential(page, "partner-proposer-session");
    await expect(page.getByRole("heading", { name: "Create a payout plan", exact: true })).toBeVisible();
    await expect(page.getByRole("combobox", { name: "Verified payout destination" })).toBeVisible();
    await page.getByRole("combobox", { name: "Verified payout destination" }).selectOption(destinationId);
    await page.getByRole("combobox", { name: "Settlement split" }).selectOption(splitPlanId);
    await page.getByRole("combobox", { name: "Published tax policy" }).selectOption(policyId);
    await page.getByLabel("Who prepares payouts?").fill("payout-operator");
    await page.getByLabel("Start after settlement").fill("100");
    await page.getByLabel("Maximum lifetime payout").fill("1000");
    await page.getByLabel("Maximum payout at one time").fill("100");
    await expect(page.getByLabel(/plan fingerprint|policy fingerprint|destination token account/i)).toHaveCount(0);
    await page.getByRole("button", { name: "Create plan for review", exact: true }).click();
    await expect(page.getByText("The payout plan is recorded and waiting for its required review.", { exact: true })).toBeVisible();
    expect(proposalBody).toMatchObject({
      destination_route_id: destinationId,
      split_plan_id: splitPlanId,
      policy_id: policyId,
      operator_subject: "payout-operator",
      start_slot: "100",
      max_total_atomic: "1000",
      max_claim_atomic: "100",
    });
    expect(proposalBody).not.toHaveProperty("plan_digest");
    expect(proposalBody).not.toHaveProperty("policy_digest");
    expect(proposalBody).not.toHaveProperty("recipient");
    expect(proposalBody).not.toHaveProperty("token_program");
    await expectNoBrowserAxeViolations(page, "partner payout reference selection flow");
  });

  test("localizes access-control explanations in German and Arabic RTL without translating policy contracts", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: "Open display settings" }).click();
    await page.getByLabel("Language").selectOption("de-DE");
    await page.goto("/rules/access-approvals");
    await expect(page.getByRole("heading", { name: "Wer darf was?", exact: true })).toBeVisible();
    await expect(page.getByRole("heading", { name: "Aktionen und Genehmigungen", exact: true })).toBeVisible();
    await expect(page.getByText("Steuerangebot erstellen", { exact: true }).first()).toBeVisible();
    await expect(page.getByText("RBAC checks responsibilities", { exact: false })).toHaveCount(0);
    await expectNoBrowserAxeViolations(page, "German access-control localization");

    await page.getByRole("button", { name: "Anzeigeeinstellungen öffnen" }).click();
    await page.getByLabel("Sprache").selectOption("ar-EG");
    await expect(page.locator("html")).toHaveAttribute("dir", "rtl");
    await expect(page.getByRole("heading", { name: "من يستطيع تنفيذ ماذا؟", exact: true })).toBeVisible();
    await expect(page.getByRole("heading", { name: "الإجراءات والموافقات", exact: true })).toBeVisible();
    await expectNoBrowserAxeViolations(page, "Arabic RTL access-control localization");
  });

  test("localizes partner payout governance in German and Arabic RTL without translating values", async ({ page }) => {
    const capabilities = completeCapabilityCatalog({
      "revenue_stream.read": { decision: "allow", reason_code: "scope_and_role_satisfied" },
      "revenue_stream.propose": { decision: "allow", reason_code: "scope_and_role_satisfied" },
    });
    await page.route("**/api/v1/authorization/capabilities", (route) => route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({ schema: "authorization-capabilities/v1", snapshot_id: "01992f1b-0000-7000-8000-000000000041", subject: "payout-proposer", tenant_id: "01992f1b-0000-4000-8000-000000000041", expires_at_unix: Math.floor(Date.now() / 1000) + 30, capabilities }),
    }));
    await page.route("**/api/v1/revenue-streaming/plans", (route) => route.fulfill({ status: 200, contentType: "application/json", body: "[]" }));
    await page.route("**/api/v1/revenue-streaming/references", (route) => route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify({ destinations: [], split_plans: [], policies: [] }) }));

    await page.goto("/");
    await page.getByRole("button", { name: "Open display settings" }).click();
    await page.getByLabel("Language").selectOption("de-DE");
    await page.goto("/money/partner-payouts");
    await fillLocalCredential(page, "localized-payout-session");
    await expect(page.getByRole("heading", { name: "Partnerauszahlungspläne", exact: true })).toBeVisible();
    await expect(page.getByRole("heading", { name: "Auszahlungsplan erstellen", exact: true })).toBeVisible();
    await expect(page.getByText("Genehmigte Einrichtung ist nicht bereit", { exact: true })).toBeVisible();
    await expectNoBrowserAxeViolations(page, "German partner payout localization");

    await page.getByRole("button", { name: "Anzeigeeinstellungen öffnen" }).click();
    await page.getByLabel("Sprache").selectOption("ar-EG");
    await expect(page.locator("html")).toHaveAttribute("dir", "rtl");
    await expect(page.getByRole("heading", { name: "خطط مدفوعات الشركاء", exact: true })).toBeVisible();
    await expect(page.getByRole("heading", { name: "إنشاء خطة دفع", exact: true })).toBeVisible();
    await expectNoBrowserAxeViolations(page, "Arabic RTL partner payout localization");
  });

  test("fails closed for partner payouts when the server denies the capability", async ({ page }) => {
    const capabilities = completeCapabilityCatalog({
      "revenue_stream.read": { decision: "deny", reason_code: "role_missing" },
    });
    await page.route("**/api/v1/authorization/capabilities", (route) => route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        schema: "authorization-capabilities/v1",
        snapshot_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d25",
        subject: "viewer-a",
        tenant_id: "01900000-0000-4000-8000-000000000001",
        expires_at_unix: Math.floor(Date.now() / 1000) + 30,
        capabilities,
      }),
    }));
    await page.goto("/money/partner-payouts");
    await fillLocalCredential(page, "viewer-session");
    await expect(page.getByText("This workspace is not available to your account", { exact: true })).toBeVisible();
    await expect(page.getByText(/did not grant access to partner payout plans/i)).toBeVisible();
    await expect(page.getByText("Prepare a payout release", { exact: true })).toHaveCount(0);
    await expectNoBrowserAxeViolations(page, "partner payout denied state");
  });

  test("has no rendered WCAG A/AA violations across the real SPA workspaces", async ({ page }) => {
    await page.goto("/");
    await openSystemStatus(page);
    await expectNoBrowserAxeViolations(page, "production quote");

    const workspaces = [
      ["Tax rules", "Review and publish a rule"],
      ["Exceptions & recovery", "Recovery queue"],
      ["Treasury", "Create a release request"],
      ["Balance checks", "Check tax balances"],
      ["Adjustments", "Create an adjustment request"],
      ["Refund requests", "Prepare a refund request"],
      ["Refund payments", "Prepare refund release"],
      ["Payment review", "Review payment before signing"],
      ["Invoice tax", "Calculate tax for an invoice"],
      ["Document payment", "Review and authorize this payment"],
    ] as const;

    for (const [buttonName, headingName] of workspaces) {
      await openNavigationGroupFor(page, buttonName);
      await page.getByRole("button", { name: buttonName, exact: true }).click();
      await expect(page.getByRole("heading", { name: headingName, exact: true })).toBeVisible();
      await expectNoBrowserAxeViolations(page, buttonName);
    }
  });

  test("keeps the selected semantic theme across every workspace", async ({ page }) => {
    await page.goto("/");
    const darkThemeMetadata = page.locator('meta[name="theme-color"][data-theme-color="dark"]');
    const lightThemeMetadata = page.locator('meta[name="theme-color"][data-theme-color="light"]');
    await expect(darkThemeMetadata).toHaveAttribute("media", "(prefers-color-scheme: dark)");
    await expect(lightThemeMetadata).toHaveAttribute("media", "(prefers-color-scheme: light)");
    await page.getByRole("button", { name: "Open display settings" }).click();
    const theme = page.getByRole("combobox", { name: "Theme" });

    await theme.selectOption("light");
    await expect(darkThemeMetadata).toHaveAttribute("media", "not all");
    await expect(lightThemeMetadata).toHaveAttribute("media", "all");
    const lightShellBackground = await page.locator(".product-shell").evaluate(
      (element) => getComputedStyle(element).backgroundColor,
    );
    await theme.selectOption("dark");
    await expect(darkThemeMetadata).toHaveAttribute("media", "all");
    await expect(lightThemeMetadata).toHaveAttribute("media", "not all");
    const darkShellBackground = await page.locator(".product-shell").evaluate(
      (element) => getComputedStyle(element).backgroundColor,
    );

    expect(lightShellBackground).not.toBe(darkShellBackground);

    const workspaces = [
      "Tax quotes", "Tax simulator", "Invoice tax", "Payment review",
      "Document payment", "Tax rules", "Access & approvals", "Treasury",
      "Balance checks", "Partner payouts", "Adjustments", "Refund requests",
      "Refund cancellations", "Refund payments", "Exceptions & recovery",
    ] as const;

    for (const workspace of workspaces) {
      await openNavigationGroupFor(page, workspace);
      await page.getByRole("button", { name: workspace, exact: true }).click();
      await expect(page.locator(".product-shell")).toHaveAttribute("data-theme", "dark");
      await expect.poll(() => page.locator(".product-shell").evaluate(
        (element) => getComputedStyle(element).backgroundColor,
      )).toBe(darkShellBackground);
    }
  });

  test("localizes the shell navigation without changing the workspace route", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: "Open display settings" }).click();
    await page.getByRole("combobox", { name: "Language" }).selectOption("de-DE");
    await expect(page.locator("html")).toHaveAttribute("dir", "ltr");
    await expect(page.getByRole("button", { name: "Steuerangebote", exact: true })).toBeVisible();
    await expect(page.getByRole("heading", { name: "Steuerangebote", exact: true })).toBeVisible();
    await expect(page.getByRole("status")).toHaveText("Arbeitsbereich: Steuerangebote");
    await expectNoBrowserAxeViolations(page, "German shell navigation");

    await page.getByRole("combobox", { name: "Sprache" }).selectOption("ar-EG");
    await expect(page.locator("html")).toHaveAttribute("dir", "rtl");
    await expect(page.getByRole("button", { name: "عروض ضريبية", exact: true })).toBeVisible();
    await expect(page.getByRole("heading", { name: "عروض ضريبية", exact: true })).toBeVisible();
    await expect(page.getByRole("status")).toHaveText("مساحة العمل: عروض ضريبية");
    await expectNoBrowserAxeViolations(page, "Arabic shell navigation");
  });

  test("localizes the quote workspace without changing its financial input semantics", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: "Open display settings" }).click();
    await page.getByRole("combobox", { name: "Language" }).selectOption("de-DE");
    await expect(page.getByRole("heading", { name: "Steuerangebot erstellen", exact: true })).toBeVisible();
    await expect(page.getByLabel("Gesamt vor Steuer", { exact: true })).toBeVisible();
    await expect(page.getByPlaceholder("Währungscode eingeben", { exact: true })).toBeVisible();
    await expectNoBrowserAxeViolations(page, "German quote workspace");

    await page.getByRole("combobox", { name: "Sprache" }).selectOption("ar-EG");
    await expect(page.locator("html")).toHaveAttribute("dir", "rtl");
    await expect(page.getByRole("heading", { name: "إنشاء عرض ضريبي", exact: true })).toBeVisible();
    await expect(page.getByLabel("الإجمالي قبل الضريبة", { exact: true })).toBeVisible();
    await expect(page.getByPlaceholder("أدخل رمز العملة", { exact: true })).toBeVisible();
    await expectNoBrowserAxeViolations(page, "Arabic quote workspace");
  });

  test("localizes the invoice quote workspace without translating document values", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: "Open display settings" }).click();
    await page.getByRole("combobox", { name: "Language" }).selectOption("de-DE");
    await page.goto("/operations/invoice-tax");
    await expect(page.getByRole("heading", { name: "Steuer für eine Rechnung berechnen", exact: true })).toBeVisible();
    await expect(page.getByLabel("Dokument-ID", { exact: true })).toBeVisible();
    await expect(page.getByRole("button", { name: "Position hinzufügen", exact: true })).toBeVisible();
    await expectNoBrowserAxeViolations(page, "German invoice quote workspace");

    await page.getByRole("button", { name: "Anzeigeeinstellungen öffnen" }).click();
    await page.getByRole("combobox", { name: "Sprache" }).selectOption("ar-EG");
    await expect(page.locator("html")).toHaveAttribute("dir", "rtl");
    await expect(page.getByRole("heading", { name: "حساب الضريبة لفاتورة", exact: true })).toBeVisible();
    await expect(page.getByLabel("معرّف المستند", { exact: true })).toBeVisible();
    await expect(page.getByRole("button", { name: "إضافة بند", exact: true })).toBeVisible();
    await expectNoBrowserAxeViolations(page, "Arabic invoice quote workspace");
  });

  test("localizes aggregate payment review in German and Arabic RTL", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: "Open display settings" }).click();
    await page.getByRole("combobox", { name: "Language" }).selectOption("de-DE");
    await page.goto("/operations/payment-review");

    await expect(page.locator(".app-shell")).toHaveAttribute("dir", "ltr");
    await expect(page.getByRole("heading", { name: "Zahlung vor der Signatur prüfen", exact: true })).toBeVisible();
    await expect(page.locator(".checkout-review").getByRole("heading", { name: "Zahlungsprüfung", exact: true })).toBeVisible();
    await expect(page.getByRole("button", { name: "Zahlung zur Prüfung vorbereiten", exact: true })).toBeVisible();
    await expect(page.getByText("Derzeit ist kein kompatibles Solana-Wallet verfügbar.", { exact: true })).toBeVisible();
    await expectNoBrowserAxeViolations(page, "German aggregate checkout localization");

    await page.getByRole("button", { name: "Anzeigeeinstellungen öffnen" }).click();
    await page.getByRole("combobox", { name: "Sprache" }).selectOption("ar-EG");
    await expect(page.locator(".app-shell")).toHaveAttribute("dir", "rtl");
    await expect(page.getByRole("heading", { name: "راجع الدفع قبل التوقيع", exact: true })).toBeVisible();
    await expect(page.locator(".checkout-review").getByRole("heading", { name: "مراجعة الدفع", exact: true })).toBeVisible();
    await expect(page.getByRole("button", { name: "إعداد الدفع للمراجعة", exact: true })).toBeVisible();
    await expectNoBrowserAxeViolations(page, "Arabic RTL aggregate checkout localization");
  });

  test("localizes governed Treasury release review in German and Arabic RTL", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: "Open display settings" }).click();
    await page.getByRole("combobox", { name: "Language" }).selectOption("de-DE");
    await page.goto("/money/treasury");

    const treasury = page.locator(".vault-layout");
    await expect(treasury).toHaveAttribute("aria-label", "Verwalteter Tresor-Freigabebereich");
    await expect(treasury.getByRole("heading", { name: "Freigabeantrag erstellen", exact: true })).toBeVisible();
    await expect(treasury.getByRole("heading", { name: "Antrag prüfen oder weiterführen", exact: true })).toBeVisible();
    await expect(treasury.getByRole("button", { name: "Freigabeantrag erstellen", exact: true })).toBeDisabled();
    await expect(treasury.getByText("Vier-Augen-Kontrolle", { exact: true })).toBeVisible();
    await expectNoBrowserAxeViolations(page, "German Treasury release localization");

    await page.getByRole("button", { name: "Anzeigeeinstellungen öffnen" }).click();
    await page.getByRole("combobox", { name: "Sprache" }).selectOption("ar-EG");
    await expect(page.locator(".app-shell")).toHaveAttribute("dir", "rtl");
    await expect(treasury).toHaveAttribute("aria-label", "مساحة إصدار الخزينة الخاضعة للحوكمة");
    await expect(treasury.getByRole("heading", { name: "إنشاء طلب إصدار", exact: true })).toBeVisible();
    await expect(treasury.getByRole("heading", { name: "مراجعة طلب أو تغيير حالته", exact: true })).toBeVisible();
    await expect(treasury.getByRole("button", { name: "إنشاء طلب إصدار", exact: true })).toBeDisabled();
    await expectNoBrowserAxeViolations(page, "Arabic RTL Treasury release localization");
  });

  test("localizes refund-request wallet authorization in German and Arabic RTL", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: "Open display settings" }).click();
    await page.getByRole("combobox", { name: "Language" }).selectOption("de-DE");
    await page.goto("/money/refund-requests");

    const refund = page.locator("section.adjustment-layout").first();
    await expect(refund).toHaveAttribute("aria-label", "Wallet-Arbeitsbereich für Rückerstattungsanträge");
    await expect(refund.getByRole("heading", { name: "Rückerstattungsantrag vorbereiten", exact: true })).toBeVisible();
    await expect(refund.getByRole("heading", { name: "Wallet-Autorisierung", exact: true })).toBeVisible();
    await expect(refund.getByRole("button", { name: "Rückerstattungsantrag vorbereiten", exact: true })).toBeDisabled();
    await expect(refund.getByText("Sie behalten die Verwahrung", { exact: true })).toBeVisible();
    await expectNoBrowserAxeViolations(page, "German refund-request localization");

    await page.getByRole("button", { name: "Anzeigeeinstellungen öffnen" }).click();
    await page.getByRole("combobox", { name: "Sprache" }).selectOption("ar-EG");
    await expect(page.locator(".app-shell")).toHaveAttribute("dir", "rtl");
    await expect(refund).toHaveAttribute("aria-label", "مساحة محفظة طلبات استرداد الأموال");
    await expect(refund.getByRole("heading", { name: "إعداد طلب استرداد", exact: true })).toBeVisible();
    await expect(refund.getByRole("heading", { name: "تفويض المحفظة", exact: true })).toBeVisible();
    await expect(refund.getByRole("button", { name: "إعداد طلب استرداد", exact: true })).toBeDisabled();
    await expectNoBrowserAxeViolations(page, "Arabic RTL refund-request localization");
  });

  test("localizes refund cancellation authorization in German and Arabic RTL", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: "Open display settings" }).click();
    await page.getByRole("combobox", { name: "Language" }).selectOption("de-DE");
    await page.goto("/money/refund-cancellations");

    const cancellation = page.locator("section.adjustment-layout").first();
    await expect(cancellation).toHaveAttribute("aria-label", "Wallet-Arbeitsbereich für die Stornierung von Rückerstattungsanträgen");
    await expect(cancellation.getByRole("heading", { name: "Rückerstattungsantrag stornieren", exact: true })).toBeVisible();
    await expect(cancellation.getByRole("heading", { name: "Autorisierung der Governance-Wallet", exact: true })).toBeVisible();
    await expect(cancellation.getByRole("button", { name: "Stornierung vorbereiten", exact: true })).toBeDisabled();
    await expect(cancellation.getByText("Keine Geldbewegung", { exact: true })).toBeVisible();
    await expectNoBrowserAxeViolations(page, "German refund-cancellation localization");

    await page.getByRole("button", { name: "Anzeigeeinstellungen öffnen" }).click();
    await page.getByRole("combobox", { name: "Sprache" }).selectOption("ar-EG");
    await expect(page.locator(".app-shell")).toHaveAttribute("dir", "rtl");
    await expect(cancellation).toHaveAttribute("aria-label", "مساحة محفظة إلغاء طلبات الاسترداد");
    await expect(cancellation.getByRole("heading", { name: "إلغاء طلب استرداد", exact: true })).toBeVisible();
    await expect(cancellation.getByRole("heading", { name: "تفويض محفظة الحوكمة", exact: true })).toBeVisible();
    await expect(cancellation.getByRole("button", { name: "إعداد الإلغاء", exact: true })).toBeDisabled();
    await expect(cancellation.getByText("لم تتحرك أموال", { exact: true })).toBeVisible();
    await expectNoBrowserAxeViolations(page, "Arabic RTL refund-cancellation localization");
  });

  test("localizes replay recovery controls in German and Arabic RTL", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: "Open display settings" }).click();
    await page.getByRole("combobox", { name: "Language" }).selectOption("de-DE");
    await page.goto("/records/exceptions-recovery");

    const replay = page.locator("section.replay-layout").first();
    await expect(replay).toHaveAttribute("aria-label", "Arbeitsbereich für Indexer-Wiederherstellung");
    await expect(replay.getByRole("heading", { name: "Wiederherstellungslücken prüfen", exact: true })).toBeVisible();
    await expect(replay.getByRole("heading", { name: "Wiederherstellungswarteschlange", exact: true })).toBeVisible();
    await expect(replay.getByText("Routenprüfungsnachweise", { exact: true })).toBeVisible();
    await expect(replay.getByRole("button", { name: "Wiederherstellungselemente laden", exact: true })).toBeDisabled();
    await expectNoBrowserAxeViolations(page, "German replay-recovery localization");

    await page.getByRole("button", { name: "Anzeigeeinstellungen öffnen" }).click();
    await page.getByRole("combobox", { name: "Sprache" }).selectOption("ar-EG");
    await expect(page.locator(".app-shell")).toHaveAttribute("dir", "rtl");
    await expect(replay).toHaveAttribute("aria-label", "مساحة عمليات استعادة الفهرس");
    await expect(replay.getByRole("heading", { name: "مراجعة فجوات الاستعادة", exact: true })).toBeVisible();
    await expect(replay.getByRole("heading", { name: "قائمة انتظار الاستعادة", exact: true })).toBeVisible();
    await expect(replay.getByText("أدلة التحقق من المسار", { exact: true })).toBeVisible();
    await expect(replay.getByRole("button", { name: "تحميل عناصر الاستعادة", exact: true })).toBeDisabled();
    await expectNoBrowserAxeViolations(page, "Arabic RTL replay-recovery localization");
  });

  test("localizes vault reconciliation release-stop states in German and Arabic RTL", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: "Open display settings" }).click();
    await page.getByRole("combobox", { name: "Language" }).selectOption("de-DE");
    await page.goto("/money/balance-checks");

    const reconciliation = page.locator("section.vault-layout").first();
    await expect(reconciliation).toHaveAttribute("aria-label", "Arbeitsbereich für Tresorabgleich");
    await expect(reconciliation.getByRole("heading", { name: "Steuerguthaben prüfen", exact: true })).toBeVisible();
    await expect(reconciliation.getByRole("heading", { name: "Aktive Steuerkonten", exact: true })).toBeVisible();
    await expect(reconciliation.getByRole("button", { name: "Abgleichsziele laden", exact: true })).toBeDisabled();
    await expect(reconciliation.getByText(/Guthaben nicht ändern, Gelder nicht freigeben/i)).toBeVisible();
    await expectNoBrowserAxeViolations(page, "German vault-reconciliation localization");

    await page.getByRole("button", { name: "Anzeigeeinstellungen öffnen" }).click();
    await page.getByRole("combobox", { name: "Sprache" }).selectOption("ar-EG");
    await expect(page.locator(".app-shell")).toHaveAttribute("dir", "rtl");
    await expect(reconciliation).toHaveAttribute("aria-label", "مساحة مطابقة خزينة الضرائب");
    await expect(reconciliation.getByRole("heading", { name: "فحص أرصدة الضرائب", exact: true })).toBeVisible();
    await expect(reconciliation.getByRole("heading", { name: "حسابات الضرائب النشطة", exact: true })).toBeVisible();
    await expect(reconciliation.getByRole("button", { name: "تحميل أهداف المطابقة", exact: true })).toBeDisabled();
    await expectNoBrowserAxeViolations(page, "Arabic RTL vault-reconciliation localization");
  });

  test("localizes settlement adjustment governance in German and Arabic RTL", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: "Open display settings" }).click();
    await page.getByRole("combobox", { name: "Language" }).selectOption("de-DE");
    await page.goto("/money/adjustments");

    const adjustment = page.locator("section.adjustment-layout").first();
    await expect(adjustment).toHaveAttribute("aria-label", "Arbeitsbereich für Zahlungsanpassungen");
    await expect(adjustment.getByRole("heading", { name: "Korrekturantrag erstellen", exact: true })).toBeVisible();
    await expect(adjustment.getByRole("heading", { name: "Korrektur prüfen", exact: true })).toBeVisible();
    await expect(adjustment.getByRole("button", { name: "Korrekturantrag erstellen", exact: true })).toBeDisabled();
    await expect(adjustment.getByText("Keine Geldbewegung", { exact: true })).toHaveCount(2);
    await expectNoBrowserAxeViolations(page, "German settlement-adjustment localization");

    await page.getByRole("button", { name: "Anzeigeeinstellungen öffnen" }).click();
    await page.getByRole("combobox", { name: "Sprache" }).selectOption("ar-EG");
    await expect(page.locator(".app-shell")).toHaveAttribute("dir", "rtl");
    await expect(adjustment).toHaveAttribute("aria-label", "مساحة تعديلات التسوية");
    await expect(adjustment.getByRole("heading", { name: "إنشاء طلب تصحيح", exact: true })).toBeVisible();
    await expect(adjustment.getByRole("heading", { name: "مراجعة التصحيح", exact: true })).toBeVisible();
    await expect(adjustment.getByRole("button", { name: "إنشاء طلب تصحيح", exact: true })).toBeDisabled();
    await expect(adjustment.getByText("لم تتحرك أموال", { exact: true })).toHaveCount(2);
    await expectNoBrowserAxeViolations(page, "Arabic RTL settlement-adjustment localization");
  });

  test("localizes refund execution authorization in German and Arabic RTL", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: "Open display settings" }).click();
    await page.getByRole("combobox", { name: "Language" }).selectOption("de-DE");
    await page.goto("/money/refund-payments");

    const refund = page.locator("section.adjustment-layout").first();
    await expect(refund).toHaveAttribute("aria-label", "Wallet-Arbeitsbereich für die Rückerstattungsfreigabe");
    await expect(refund.getByRole("heading", { name: "Rückerstattung freigeben", exact: true })).toBeVisible();
    await expect(refund.getByRole("heading", { name: "Ausführungsautorisierung", exact: true })).toBeVisible();
    await expect(refund.getByRole("button", { name: "Rückerstattung freigeben", exact: true })).toBeDisabled();
    await expect(refund.getByText("Nachweis erforderlich", { exact: true })).toBeVisible();
    await expectNoBrowserAxeViolations(page, "German refund-execution localization");

    await page.getByRole("button", { name: "Anzeigeeinstellungen öffnen" }).click();
    await page.getByRole("combobox", { name: "Sprache" }).selectOption("ar-EG");
    await expect(page.locator(".app-shell")).toHaveAttribute("dir", "rtl");
    await expect(refund).toHaveAttribute("aria-label", "مساحة محفظة إصدار الاسترداد");
    await expect(refund.getByRole("heading", { name: "إعداد إصدار الاسترداد", exact: true })).toBeVisible();
    await expect(refund.getByRole("heading", { name: "تفويض التنفيذ", exact: true })).toBeVisible();
    await expect(refund.getByRole("button", { name: "إعداد إصدار الاسترداد", exact: true })).toBeDisabled();
    await expect(refund.getByText("الدليل مطلوب", { exact: true })).toBeVisible();
    await expectNoBrowserAxeViolations(page, "Arabic RTL refund-execution localization");
  });

  test("restores display preferences without restoring local credentials", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: "Open display settings" }).click();
    await fillLocalCredential(page, "never-persist-this-credential");
    // The shell keeps adjacent popovers mutually exclusive so a credential
    // surface cannot overlap the display-settings menu. Reopen the settings
    // disclosure before changing the persisted presentation preferences.
    await page.getByRole("button", { name: "Open display settings" }).click();
    await page.getByRole("combobox", { name: "Language" }).selectOption("ar-EG");
    await page.getByRole("combobox", { name: "المنطقة الزمنية" }).fill("Asia/Kolkata");
    await page.getByRole("combobox", { name: "السمة" }).selectOption("light");

    await page.reload();

    await expect(page.locator(".product-shell")).toHaveAttribute("data-theme", "light");
    await expect(page.locator("html")).toHaveAttribute("lang", "ar");
    await expect(page.locator("html")).toHaveAttribute("dir", "rtl");
    await page.getByRole("button", { name: "فتح إعدادات العرض" }).click();
    await expect(page.getByRole("combobox", { name: "اللغة" })).toHaveValue("ar-EG");
    await expect(page.getByRole("combobox", { name: "المنطقة الزمنية" })).toHaveValue("Asia/Kolkata");

    await page.getByRole("button", { name: "فتح تسجيل الدخول المحلي" }).click();
    await expect(page.getByLabel("بيانات تسجيل الدخول (محفوظة في الذاكرة)")).toHaveValue("");
    await expectNoBrowserAxeViolations(page, "persisted display preferences");
  });

  test("keeps narrow RTL and reduced-motion display behavior bounded", async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 800 });
    await page.goto("/");

    await page.getByText("Display settings", { exact: true }).click();
    await page.getByRole("combobox", { name: "Language" }).selectOption("ar-EG");
    await page.getByRole("button", { name: "فتح التنقل", exact: true }).click();
    const arabicRulesGroup = page.locator("details.nav-group-disclosure").filter({ hasText: "القواعد والوصول" }).first();
    await arabicRulesGroup.locator("summary").click();
    await expect(page.getByRole("button", { name: "القواعد الضريبية", exact: true })).toBeVisible();
    const arabicMoneyGroup = page.locator("details.nav-group-disclosure").filter({ hasText: "حركة الأموال" }).first();
    await arabicMoneyGroup.locator("summary").click();
    await page.getByRole("button", { name: "فحوصات الأرصدة", exact: true }).click();
    await expect(page.getByRole("button", { name: "فتح التنقل", exact: true })).toHaveAttribute("aria-expanded", "false");
    await page.getByRole("button", { name: "فتح التنقل", exact: true }).click();

    await expect(page.locator("html")).toHaveAttribute("dir", "rtl");
    await expect(page.locator(".app-shell")).toHaveAttribute("dir", "rtl");
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);

    const arabicDailyGroup = page.locator("details.nav-group-disclosure").filter({ hasText: "العمل اليومي" }).first();
    await arabicDailyGroup.locator("summary").click();
    const transitionDuration = await page.getByRole("button", { name: "عروض ضريبية" }).evaluate(
      (element) => getComputedStyle(element).transitionDuration,
    );
    expect(transitionDuration).toBe("0s");
  });

  test("contains a real dynamic-import failure without exposing exception data", async ({ page }) => {
    await page.route("**/assets/GovernancePanel-*.js", (route) => route.abort("failed"));
    await page.goto("/");
    await openNavigationGroupFor(page, "Tax rules");
    await page.getByRole("button", { name: "Tax rules", exact: true }).click();

    await expect(page.getByRole("heading", { name: "This workspace could not be loaded" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Reload application" })).toBeVisible();
    await expect(page.locator(".workspace-error").getByText(/secret|exception|stack|token/i)).toHaveCount(0);
  });

  test("fails closed when the API health boundary is unavailable", async ({ page }) => {
    await page.route("**/*healthz*", (route) => route.fulfill({
      status: 503,
      contentType: "application/json",
      body: JSON.stringify({ error: { code: "SERVICE_UNAVAILABLE", message: "temporarily unavailable" } }),
    }));
    await page.goto("/");

    const outageAlert = page.getByRole("alert");
    await expect(outageAlert).toHaveText("The service is unavailable. No client-side fallback is used.");
    await expect(page.getByText("Your result will appear here", { exact: true })).toBeVisible();
    await expect(page.getByRole("button", { name: "Prepare quote", exact: true })).toBeDisabled();
    await expectNoBrowserAxeViolations(page, "API unavailable state");
  });

  test("shows an expired quote and blocks wallet compilation before signing", async ({ page }) => {
    const now = Math.floor(Date.now() / 1000);
    await page.route("**/api/v1/tax/quote", (route) => route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        calculation_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d21",
        status: "calculated_unattested",
        expires_at_unix: now - 1,
        result: {
          rule_set_id: "published-sales-tax",
          rule_version: 3,
          engine_version: "tax-engine/v1",
          atomic_decimals: 2,
          currency: "USD",
          jurisdiction: "US-CA",
          facts_digest: Array(32).fill(1),
          rule_digest: Array(32).fill(2),
          gross_atomic: "1000",
          total_tax_atomic: "100",
          total_net_atomic: "900",
          components: [{
            id: "sales-tax",
            label: "Sales tax",
            base_atomic: "1000",
            effective_base_atomic: "1000",
            threshold_atomic: null,
            exemption_applied: false,
            raw_tax: { numerator: "100", denominator: "1" },
            credit_atomic: "0",
            clamped_tax_atomic: "100",
            tax_atomic: "100",
            explanation: "Published rule evidence",
          }],
        },
      }),
    }));
    await page.route("**/api/v1/settlement/routes", (route) => route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify([{
        id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d22",
        route_key: "local-usdc",
        version: 1,
        merchant_account: "8qbHbw2BbbTHBW1sbeqakYXVKRQM8Ne7pLK7m6CVfeR",
        mint: "CktRuQ2mttgRGkXJtyksdKHjUdc2C4TgDzyB98oEzy8",
        mint_decimals: 6,
        tax_vault: "8nGRRJuGoubpUKHVa5paPCvGLQU39nnu3zFWYREbSeNN",
        net_destination: "QWmroo4YnnMqYW3cnxWkFdaTxGD3P7vMSzwMHGbUzwF",
        token_program: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
      }]),
    }));
    await page.route("**/api/v1/settlement/split-plans", (route) => route.fulfill({
      status: 200,
      contentType: "application/json",
      body: "[]",
    }));

    await page.goto("/");
    await openSystemStatus(page);
    await fillLocalCredential(page, "synthetic-browser-token");
    await openLocationVerification(page);
    await page.getByLabel("Provider-signed location record (advanced)").fill("[{}]");
    await page.getByLabel("Total before tax").fill("1100");
    await page.getByLabel("Amount subject to tax").fill("1000");
    await page.getByLabel("Currency").fill("USD");
    await page.getByRole("textbox", { name: /Asset precision/ }).fill("2");
    await page.getByLabel("Approved tax rule").fill("operator-simulator-flat-rate");
    await page.getByLabel("Rule version").fill("1");
    await page.getByLabel("Jurisdiction").fill("US-CA");
    await page.getByLabel("Tax category").fill("sales-tax");
    await page.getByRole("button", { name: "Prepare quote", exact: true }).click();
    await expect(page.getByText("Reference 01992f1b-4a70-7e30-8ea2-b6e37a9d9d21", { exact: true })).toBeVisible();

    await page.getByRole("button", { name: "Payment review", exact: true }).click();
    await expect(page.getByRole("heading", { name: "Review payment before signing", exact: true })).toBeVisible();
    await expect(page.getByText(/expired; create a new quote/)).toBeVisible();
    await expect(page.getByRole("button", { name: "Prepare payment for review", exact: true })).toBeDisabled();
    await expectNoBrowserAxeViolations(page, "expired quote checkout state");
  });

  test("renders an accessible wallet-rejection state without claiming authorization", async ({ page }) => {
    await page.addInitScript(() => {
      Object.defineProperty(window, "solana", {
        configurable: true,
        value: {
          connect: async () => { throw new Error("User rejected wallet connection"); },
        },
      });
    });

    await page.goto("/");
    await openSystemStatus(page);
    await page.getByRole("button", { name: "Payment review", exact: true }).click();
    await page.getByRole("button", { name: "Connect wallet", exact: true }).click();
    await expect(page.getByRole("alert")).toContainText("Wallet connection was rejected.");
    await expect(page.getByRole("alert")).not.toContainText("User rejected wallet connection");
    await expect(page.getByText("No wallet connected", { exact: true })).toBeVisible();
    await expectNoBrowserAxeViolations(page, "wallet rejection state");
  });

  test("requires explicit Wallet Standard account selection across a browser reload", async ({ page }) => {
    await page.addInitScript(() => {
      const alphabet = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
      const decodeBase58 = (value: string): Uint8Array => {
        const bytes = [0];
        for (const character of value) {
          const index = alphabet.indexOf(character);
          if (index < 0) throw new Error("invalid test public key");
          let carry = index;
          for (let offset = 0; offset < bytes.length; offset += 1) {
            const next = bytes[offset] * 58 + carry;
            bytes[offset] = next & 0xff;
            carry = next >> 8;
          }
          while (carry > 0) {
            bytes.push(carry & 0xff);
            carry >>= 8;
          }
        }
        for (let index = 0; value[index] === "1"; index += 1) bytes.push(0);
        return Uint8Array.from(bytes.reverse());
      };

      const accounts = [
        "11111111111111111111111111111111",
        "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
      ].map((address) => ({
        address,
        publicKey: decodeBase58(address),
        chains: ["solana:localnet"],
        features: ["solana:signTransaction"],
      }));
      const wallet = {
        version: "1.0.0",
        name: "Browser Standard Test Wallet",
        icon: "data:image/svg+xml;base64,PHN2Zy8+",
        chains: ["solana:localnet"],
        accounts,
        features: {
          "standard:connect": {
            version: "1.0.0",
            connect: async () => ({ accounts }),
          },
          "solana:signTransaction": {
            version: "1.0.0",
            signTransaction: async () => [],
          },
        },
      };
      window.addEventListener("wallet-standard:app-ready", (event) => {
        const detail = (event as CustomEvent<{ register: (candidate: typeof wallet) => void }>).detail;
        detail.register(wallet);
      });
    });

    await page.goto("/");
    await openSystemStatus(page);
    await page.getByRole("button", { name: "Payment review", exact: true }).click();
    await page.getByRole("button", { name: "Connect wallet", exact: true }).click();

    const accountPicker = page.getByRole("combobox", { name: "Wallet account" });
    await expect(accountPicker).toBeVisible();
    await expect(page.getByRole("alert")).toContainText("multiple Solana accounts");
    await expect(accountPicker).toHaveValue("");
    await accountPicker.selectOption("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
    await page.getByRole("button", { name: "Connect selected account", exact: true }).click();
    await expect(page.getByText("Selected account: TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA", { exact: true })).toBeVisible();
    await expectNoBrowserAxeViolations(page, "Wallet Standard account selection");

    await page.reload();
    await openSystemStatus(page);
    await page.getByRole("button", { name: "Payment review", exact: true }).click();
    await page.getByRole("button", { name: "Connect wallet", exact: true }).click();
    await expect(page.getByRole("combobox", { name: "Wallet account" })).toHaveValue("");
    await expectNoBrowserAxeViolations(page, "Wallet Standard account selection after reload");
  });

  test("renders and reloads server-bound finalized refund evidence", async ({ page }) => {
    const response = finalizedRefundClaimResponse();
    await page.route("**/api/v1/settlement/refund-claims**", async (route) => {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(response),
      });
    });

    await page.goto("/");
    await openSystemStatus(page);
    await fillLocalCredential(page, "synthetic-browser-token");
    await openNavigationGroupFor(page, "Refund requests");
    await page.getByRole("button", { name: "Refund requests", exact: true }).click();
    await page.getByLabel("Approved correction reference").fill("01992f1b-4a70-7e30-8ea2-b6e37a9d9d12");
    await page.getByRole("button", { name: "Prepare refund request", exact: true }).click();

    await expect(page.getByText("Finalized claim event evidence matched the durable artifact.", { exact: true })).toBeVisible();
    await page.getByText("Finalized event proof", { exact: true }).click();
    await expect(page.getByText("AdjustmentRefundClaimCreated · v1", { exact: true })).toBeVisible();
    await expect(page.getByText("Gross / tax / refund", { exact: true }).locator("..").getByText("100 / 25 / 75", { exact: true })).toBeVisible();
    await expectNoBrowserAxeViolations(page, "finalized refund proof");

    await page.reload();
    await openSystemStatus(page);
    await fillLocalCredential(page, "synthetic-browser-token");
    await openNavigationGroupFor(page, "Refund requests");
    await page.getByRole("button", { name: "Refund requests", exact: true }).click();
    await page.getByLabel("Refund request reference").fill("01992f1b-4a70-7e30-8ea2-b6e37a9d9d14");
    await page.getByRole("button", { name: "Load saved request", exact: true }).click();
    await expect(page.getByText("Finalized claim event evidence matched the durable artifact.", { exact: true })).toBeVisible();
    await page.getByText("Finalized event proof", { exact: true }).click();
    await expect(page.getByText("77", { exact: true })).toBeVisible();
    await expectNoBrowserAxeViolations(page, "finalized refund proof after reload");
  });

  test("fails closed when the authenticated document quote API is unavailable", async ({ page }) => {
    await page.route("**/api/v1/tax/document-quote", (route) => route.fulfill({
      status: 503,
      contentType: "application/json",
      headers: { "x-request-id": "01992f1b-4a70-7e30-8ea2-b6e37a9d9d31" },
      body: JSON.stringify({ error: { code: "SERVICE_UNAVAILABLE", message: "document quote service is unavailable" } }),
    }));

    await page.goto("/");
    await openSystemStatus(page);
    await fillLocalCredential(page, "synthetic-browser-token");
    await page.getByRole("button", { name: "Invoice tax", exact: true }).click();
    await page.getByLabel("Document ID").fill("invoice-1");
    await page.getByLabel("Currency").fill("USD");
    await page.getByRole("textbox", { name: /Asset precision/ }).fill("2");
    await page.getByLabel("Jurisdiction").fill("US-CA");
    await page.getByLabel("Tax category").fill("sales-tax");
    await page.getByLabel("Approved tax rule").fill("operator-document-flat-rate");
    await page.getByLabel("Rule version").fill("1");
    await page.getByLabel("Line ID").fill("line-a");
    await page.getByLabel("Amount before tax").fill("3000");
    await page.getByLabel("Amount after tax").fill("3000");
    await openLocationVerification(page);
    await page.getByLabel("Provider-signed location record (advanced)").fill("[{}]");
    await page.getByRole("button", { name: "Calculate invoice tax", exact: true }).click();

    await expect(page.getByRole("alert")).toContainText(documentQuoteMessagesFor("en-US").errors.quoteFailed);
    await expect(page.getByRole("alert")).not.toContainText("document quote service is unavailable");
    await expect(page.getByText("No invoice calculation yet", { exact: true })).toBeVisible();
    await expect(page.getByRole("button", { name: "Open document checkout", exact: true })).toHaveCount(0);
    await expectNoBrowserAxeViolations(page, "document quote API unavailable state");
  });

  test("renders document checkout route failure without enabling signing", async ({ page }) => {
    await page.route("**/api/v1/settlement/routes*", (route) => route.fulfill({
      status: 503,
      contentType: "application/json",
      headers: { "x-request-id": "01992f1b-4a70-7e30-8ea2-b6e37a9d9d32" },
      body: JSON.stringify({ error: { code: "SERVICE_UNAVAILABLE", message: "active settlement routes are unavailable" } }),
    }));

    await page.goto("/");
    await openSystemStatus(page);
    await fillLocalCredential(page, "synthetic-browser-token");
    await page.getByRole("button", { name: "Document payment", exact: true }).click();

    await expect(page.getByRole("alert")).toContainText("Routes unavailable");
    await expect(page.getByRole("alert")).not.toContainText("active settlement routes are unavailable");
    await expect(page.getByRole("combobox", { name: "Payment route" })).toHaveValue("");
    await expect(page.getByRole("button", { name: "Prepare payment for review", exact: true })).toBeDisabled();
    await expect(page.getByRole("button", { name: "Sign and submit", exact: true })).toHaveCount(0);
    await expectNoBrowserAxeViolations(page, "document checkout route failure state");
  });

  test("localizes the document checkout review surface in German and Arabic RTL", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: "Open display settings" }).click();
    await page.getByLabel("Language").selectOption("de-DE");
    await page.goto("/operations/document-payment");

    await expect(page.locator(".app-shell")).toHaveAttribute("dir", "ltr");
    await expect(page.getByRole("heading", { name: "Diese Zahlung prüfen und autorisieren", exact: true })).toBeVisible();
    await expect(page.getByRole("heading", { name: "Betrag und Empfänger bestätigen", exact: true })).toBeVisible();
    await expect(page.getByRole("button", { name: "Zahlung zur Prüfung vorbereiten", exact: true })).toBeVisible();
    await expect(page.getByText("Derzeit ist kein kompatibles Solana-Wallet verfügbar.", { exact: true })).toBeVisible();
    await expectNoBrowserAxeViolations(page, "German document checkout localization");

    await page.getByRole("button", { name: "Anzeigeeinstellungen öffnen" }).click();
    await page.getByLabel("Sprache").selectOption("ar-EG");
    await expect(page.locator(".app-shell")).toHaveAttribute("dir", "rtl");
    await expect(page.getByRole("heading", { name: "مراجعة هذه الدفعة وتفويضها", exact: true })).toBeVisible();
    await expect(page.getByRole("heading", { name: "تأكيد المبلغ والمستلم", exact: true })).toBeVisible();
    await expect(page.getByRole("button", { name: "إعداد الدفعة للمراجعة", exact: true })).toBeVisible();
    await expectNoBrowserAxeViolations(page, "Arabic RTL document checkout localization");
  });

  test("localizes tax-rule governance and structured authoring without changing protocol values", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: "Open display settings" }).click();
    await page.getByLabel("Language").selectOption("de-DE");
    await page.goto("/rules/tax-rules");

    await expect(page.getByRole("heading", { name: "Regel prüfen und veröffentlichen", exact: true })).toBeVisible();
    await expect(page.getByRole("heading", { name: "So wird die Steuer berechnet", exact: true })).toBeVisible();
    await expect(page.getByLabel("Quelldatensatz", { exact: true })).toBeVisible();
    await expect(page.locator("label").filter({ hasText: "Wie wird der Preis angezeigt?" }).locator("select")).toHaveValue("Exclusive");
    await expect(page.locator("label").filter({ hasText: "Wie soll die Regel eingegeben werden?" }).locator("select")).toHaveValue("structured");
    await expect(page.getByRole("option", { name: "Erweitertes JSON", exact: true })).toHaveAttribute("value", "json");
    await expectNoBrowserAxeViolations(page, "German tax-rule governance localization");

    await page.getByRole("button", { name: "Anzeigeeinstellungen öffnen" }).click();
    await page.getByLabel("Sprache").selectOption("ar-EG");
    await expect(page.locator("html")).toHaveAttribute("dir", "rtl");
    await expect(page.getByRole("heading", { name: "مراجعة قاعدة ونشرها", exact: true })).toBeVisible();
    await expect(page.getByRole("heading", { name: "كيفية حساب الضريبة", exact: true })).toBeVisible();
    await expectNoBrowserAxeViolations(page, "Arabic RTL tax-rule governance localization");
  });

  test("keeps saved-rule simulation progressively disclosed until requested", async ({ page }) => {
    await page.goto("/rules/tax-rules");
    await expect(page.getByRole("heading", { name: "Review and publish a rule", exact: true })).toBeVisible();

    const simulation = page.locator("details.simulation-section").filter({ hasText: "Try the saved rule" }).first();
    await expect(simulation).not.toHaveAttribute("open", "");
    await simulation.locator("summary").click();
    await expect(simulation).toHaveAttribute("open", "");
    await expect(simulation.getByLabel("Amount before tax")).toBeVisible();
    await expectNoBrowserAxeViolations(page, "progressively disclosed rule simulation");
  });

  test("proves document evidence handoff and keeps signing behind a reviewed artifact", async ({ page }) => {
    const calculationId = "01992f1b-4a70-7e30-8ea2-b6e37a9d9d41";
    const routeId = "01992f1b-4a70-7e30-8ea2-b6e37a9d9d42";
    const intentId = "01992f1b-4a70-7e30-8ea2-b6e37a9d9d43";
    const payer = "cGfHiC6Kgg3FpFZvgwGcswsCRtp4aBP2fzuXRQPizuN";
    const programId = "4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi";
    const tokenProgram = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
    const route = {
      id: routeId,
      route_key: "local-usdc",
      version: 1,
      merchant_account: "8qbHbw2BbbTHBW1sbeqakYXVKRQM8Ne7pLK7m6CVfeR",
      mint: "GgBaCs3NCBuZN12kCJgAW63ydqohFkHEdfdEXBPzLHq",
      mint_decimals: 6,
      tax_vault: "LbUiWL3xVV8hTFYBVdbTNrpDo41NKS6o3LHHuDzjfcY",
      net_destination: "QWmroo4YnnMqYW3cnxWkFdaTxGD3P7vMSzwMHGbUzwF",
      token_program: tokenProgram,
    };
    const expiresAt = Math.floor(Date.now() / 1000) + 600;
    const instructionData = Buffer.concat([
      Buffer.from("XAF7GP/zWDU=", "base64"),
      Buffer.alloc(40 + 32),
      Buffer.alloc(32, 8),
      Buffer.alloc(32, 9),
    ]).toString("base64");
    const accounts = [
      { pubkey: payer, is_signer: true, is_writable: true },
      { pubkey: "qcfm6jd37nhqmbY6JomQWtdD9S4HieqnmRm49Hum6ir", is_signer: false, is_writable: false },
      { pubkey: route.merchant_account, is_signer: false, is_writable: false },
      { pubkey: "CktRuQ2mttgRGkXJtyksdKHjUdc2C4TgDzyB98oEzy8", is_signer: false, is_writable: true },
      { pubkey: route.mint, is_signer: false, is_writable: false },
      { pubkey: route.tax_vault, is_signer: false, is_writable: true },
      { pubkey: "CsNjEE9EUZUJeAVqSWmbZxT9WB9h4xYvuLNRRPymrGcs", is_signer: false, is_writable: false },
      { pubkey: "3MUvGdb23Wh2N42NKSiQE1wcRbR3RXSXZde2MPiihHAq", is_signer: false, is_writable: false },
      { pubkey: route.net_destination, is_signer: false, is_writable: true },
      { pubkey: "8nGRRJuGoubpUKHVa5paPCvGLQU39nnu3zFWYREbSeNN", is_signer: false, is_writable: true },
      { pubkey: "2nqeGnn1BrXoKjG6H5cSFReSEuDxpb18GH3g8bkt3YVy", is_signer: false, is_writable: false },
      { pubkey: tokenProgram, is_signer: false, is_writable: false },
      { pubkey: "11111111111111111111111111111111", is_signer: false, is_writable: false },
    ];

    await page.addInitScript(({ address }) => {
      const alphabet = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
      const decodeBase58 = (value: string): Uint8Array => {
        const bytes = [0];
        for (const character of value) {
          const index = alphabet.indexOf(character);
          if (index < 0) throw new Error("invalid test public key");
          let carry = index;
          for (let offset = 0; offset < bytes.length; offset += 1) {
            const next = bytes[offset] * 58 + carry;
            bytes[offset] = next & 0xff;
            carry = next >> 8;
          }
          while (carry > 0) {
            bytes.push(carry & 0xff);
            carry >>= 8;
          }
        }
        for (let index = 0; value[index] === "1"; index += 1) bytes.push(0);
        return Uint8Array.from(bytes.reverse());
      };
      const account = {
        address,
        publicKey: decodeBase58(address),
        chains: ["solana:localnet"],
        features: ["solana:signTransaction"],
      };
      const wallet = {
        version: "1.0.0",
        name: "Browser Document Test Wallet",
        icon: "data:image/svg+xml;base64,PHN2Zy8+",
        chains: ["solana:localnet"],
        accounts: [account],
        features: {
          "standard:connect": { version: "1.0.0", connect: async () => ({ accounts: [account] }) },
          "solana:signTransaction": { version: "1.0.0", signTransaction: async () => [] },
        },
      };
      window.addEventListener("wallet-standard:app-ready", (event) => {
        const detail = (event as CustomEvent<{ register: (candidate: typeof wallet) => void }>).detail;
        detail.register(wallet);
      });
    }, { address: payer });

    await page.route("**/api/v1/tax/document-quote", (routeRequest) => routeRequest.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        calculation_id: calculationId,
        status: "attested",
        expires_at_unix: expiresAt,
        schema: "solana-tax-protocol/tax-document/v1",
        result: {
          document_id: "invoice-1",
          rule_set_id: "operator-document-flat-rate",
          rule_version: 1,
          engine_version: "tax-engine/v1",
          atomic_decimals: 2,
          currency: "USD",
          jurisdiction: "US-CA",
          facts_digest: new Array(32).fill(1),
          rule_digest: new Array(32).fill(2),
          gross_atomic: "3000",
          total_tax_atomic: "247",
          total_net_atomic: "2753",
          lines: [{
            id: "line-a",
            gross_atomic: "3000",
            total_tax_atomic: "247",
            total_net_atomic: "2753",
            components: [{ id: "sales-tax", label: "Sales tax", tax_atomic: "247", explanation: "Server-owned document trace" }],
          }],
        },
      }),
    }));
    await page.route("**/api/v1/settlement/routes", (routeRequest) => routeRequest.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify([route]),
    }));
    await page.route(`**/api/v1/tax/document-quotes/${calculationId}/attestation/status**`, (routeRequest) => routeRequest.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        status: "verified",
        calculation_id: calculationId,
        payload_digest: new Array(32).fill(8),
        issuer_public_key: new Array(32).fill(9),
        expires_at_unix: expiresAt,
        checked_at_unix: expiresAt - 100,
      }),
    }));
    await page.route("**/api/v1/settlement/document/compile-from-quote", (routeRequest) => routeRequest.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        status: "prepared_unsubmitted",
        program_id: programId,
        instruction_data_base64: instructionData,
        accounts,
        protocol_config: accounts[1].pubkey,
        vault_authority: accounts[6].pubkey,
        policy_attestation: accounts[7].pubkey,
        receipt: accounts[9].pubkey,
        asset_config: accounts[10].pubkey,
        compute_budget_policy: { policy_version: 1, compute_unit_limit: 200000, micro_lamports_per_compute_unit: 1000 },
        document_attestation_digest: new Array(32).fill(8),
        payment_scope_digest: new Array(32).fill(9),
        intent_id: intentId,
        request_digest: new Array(32).fill(1),
        instruction_digest: new Array(32).fill(2),
        route_id: routeId,
        authorization: {
          calculation_id: calculationId,
          route_id: routeId,
          source_attestation_digest: new Array(32).fill(8),
          payment_scope_digest: new Array(32).fill(9),
          scope: { kind: "full_document" },
          gross_atomic: "3000",
          tax_atomic: "247",
          net_atomic: "2753",
        },
      }),
    }));

    await page.goto("/");
    await openSystemStatus(page);
    await fillLocalCredential(page, "synthetic-browser-token");
    await page.getByRole("button", { name: "Invoice tax", exact: true }).click();
    await page.getByLabel("Document ID").fill("invoice-1");
    await page.getByLabel("Currency").fill("USD");
    await page.getByRole("textbox", { name: /Asset precision/ }).fill("2");
    await page.getByLabel("Jurisdiction").fill("US-CA");
    await page.getByLabel("Tax category").fill("sales-tax");
    await page.getByLabel("Approved tax rule").fill("operator-document-flat-rate");
    await page.getByLabel("Rule version").fill("1");
    await page.getByLabel("Line ID").fill("line-a");
    await page.getByLabel("Amount before tax").fill("3000");
    await page.getByLabel("Amount after tax").fill("3000");
    await openLocationVerification(page);
    await page.getByLabel("Provider-signed location record (advanced)").fill("[{}]");
    await page.getByRole("button", { name: "Calculate invoice tax", exact: true }).click();
    await expect(page.getByText(`Calculation ID ${calculationId}`, { exact: true })).toBeVisible();
    await page.getByRole("button", { name: "Open document checkout", exact: true }).click();
    await expect(page).toHaveURL(/\/operations\/document-payment$/);
    await expect(page.getByRole("heading", { name: "Review and authorize this payment", exact: true })).toBeVisible();

    await page.getByRole("button", { name: "Check invoice evidence", exact: true }).click();
    await expect(page.locator(".route-evidence strong").filter({ hasText: "Invoice evidence checked" }).first()).toBeVisible();
    await expect(page.getByRole("button", { name: "Prepare payment for review", exact: true })).toBeDisabled();

    await page.getByRole("button", { name: "Connect wallet", exact: true }).click();
    await page.getByRole("button", { name: "Prepare payment for review", exact: true }).click();
    await expect(page.getByText("Payment ready for your approval", { exact: true })).toBeVisible();
    await expect(page.getByText("3000", { exact: true })).toBeVisible();
    await expect(page.getByText("247", { exact: true })).toBeVisible();
    await expect(page.getByRole("button", { name: "Approve and send payment", exact: true })).toBeEnabled();
    await expectNoBrowserAxeViolations(page, "document evidence handoff and reviewed artifact");

    await page.reload();
    await expect(page).toHaveURL(/\/operations\/document-payment$/);
    await expect(page.getByRole("heading", { name: "Review and authorize this payment", exact: true })).toBeVisible();
    await expect(page.getByRole("button", { name: "Check invoice evidence", exact: true })).toBeDisabled();
    await expect(page.getByRole("button", { name: "Prepare payment for review", exact: true })).toBeDisabled();
  });

  test("renders server-owned reconciliation evidence and stops on mismatch", async ({ page }) => {
    const target = {
      id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d12",
      merchant_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d11",
      route_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d13",
      stream_key: "mainnet/settlement/v1",
      mint: "mint-account",
      token_program: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
      tax_vault: "vault-account",
      opening_balance_atomic: "1000",
      opening_balance_slot: "20",
      opening_reference: "opening ceremony INC-42",
      receipt_tax_atomic: "250",
      release_atomic: "100",
    };
    const latestRun = {
      id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d14",
      status: "mismatch",
      opening_balance_atomic: "1000",
      receipt_tax_atomic: "250",
      release_atomic: "100",
      expected_balance_atomic: "1150",
      observed_balance_atomic: "1149",
      observed_slot: "99",
      error_code: null,
      created_at: "2026-09-27T00:00:00.000Z",
      stale: false,
    };
    const historyRun = { ...latestRun, id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d15", status: "matched", observed_balance_atomic: "1150" };

    await page.route("**/v1/vault/reconciliation/targets/*/runs*", async (route) => {
      await route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify([historyRun, latestRun]) });
    });
    await page.route("**/v1/vault/reconciliation/targets?*", async (route) => {
      await route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify([{ target, latest_run: latestRun }]) });
    });

    await page.goto("/");
    await openNavigationGroupFor(page, "Balance checks");
    await page.getByRole("button", { name: "Balance checks", exact: true }).click();
    await fillLocalCredential(page, "synthetic-browser-token");
    await page.getByRole("button", { name: "Load reconciliation targets", exact: true }).click();

    await expect(page.getByText("Mismatch — stop and investigate", { exact: true }).first()).toBeVisible();
    await expect(page.getByText("The latest finalized observation does not match the evidence-derived expectation.", { exact: false })).toBeVisible();
    await expect(page.getByText("opening ceremony INC-42", { exact: true })).toBeVisible();

    await page.getByRole("button", { name: /vault-account.*Mismatch/, exact: false }).click();
    await expect(page.getByRole("heading", { name: "Recent runs", exact: true })).toBeVisible();
    await expect(page.getByText("Matched", { exact: true })).toBeVisible();
    await expectNoBrowserAxeViolations(page, "vault reconciliation mismatch state");
  });

  test("does not render an old matched observation as current evidence", async ({ page }) => {
    const staleRun = {
      id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d14",
      status: "matched",
      opening_balance_atomic: "1000",
      receipt_tax_atomic: "250",
      release_atomic: "100",
      expected_balance_atomic: "1150",
      observed_balance_atomic: "1150",
      observed_slot: "99",
      error_code: null,
      created_at: "2026-09-27T00:00:00.000Z",
      stale: true,
    };
    const target = {
      id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d12",
      merchant_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d11",
      route_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d13",
      stream_key: "mainnet/settlement/v1",
      mint: "mint-account",
      token_program: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
      tax_vault: "vault-account",
      opening_balance_atomic: "1000",
      opening_balance_slot: "20",
      opening_reference: "opening ceremony INC-42",
      receipt_tax_atomic: "250",
      release_atomic: "100",
    };
    await page.route("**/v1/vault/reconciliation/targets*", (route) => route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify([{ target, latest_run: staleRun }]),
    }));

    await page.goto("/");
    await openNavigationGroupFor(page, "Balance checks");
    await page.getByRole("button", { name: "Balance checks", exact: true }).click();
    await fillLocalCredential(page, "synthetic-browser-token");
    await page.getByRole("button", { name: "Load reconciliation targets", exact: true }).click();

    await expect(page.getByText("Stale — stop and investigate", { exact: true }).first()).toBeVisible();
    await expect(page.getByText("The latest finalized observation is stale and cannot prove the current vault balance.", { exact: false })).toBeVisible();
    await expect(page.getByText("The latest finalized observation does not match the evidence-derived expectation.", { exact: false })).toHaveCount(0);
    await expectNoBrowserAxeViolations(page, "vault reconciliation stale state");
  });

  test("fails closed on unauthorized reconciliation reads", async ({ page }) => {
    await page.route("**/v1/vault/reconciliation/targets*", (route) => route.fulfill({
      status: 403,
      contentType: "application/json",
      headers: { "x-request-id": "01992f1b-4a70-7e30-8ea2-b6e37a9d9d16" },
      body: JSON.stringify({ error: { code: "FORBIDDEN", message: "vault reconciliation access is not authorized" } }),
    }));

    await page.goto("/");
    await openNavigationGroupFor(page, "Balance checks");
    await page.getByRole("button", { name: "Balance checks", exact: true }).click();
    await fillLocalCredential(page, "synthetic-browser-token");
    await page.getByRole("button", { name: "Load reconciliation targets", exact: true }).click();

    await expect(page.getByRole("alert")).toContainText("Vault reconciliation lookup failed");
    await expect(page.getByRole("alert")).not.toContainText("vault reconciliation access is not authorized");
    await expect(page.getByText("Balance evidence not loaded", { exact: true })).toBeVisible();
    await expectNoBrowserAxeViolations(page, "vault reconciliation unauthorized state");
  });

  test("fails closed when a reconciliation read exceeds its client deadline", async ({ page }) => {
    await page.route("**/v1/vault/reconciliation/targets*", async (route) => {
      await new Promise((resolve) => setTimeout(resolve, 6_000));
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify([]),
      });
    });

    await page.goto("/");
    await openNavigationGroupFor(page, "Balance checks");
    await page.getByRole("button", { name: "Balance checks", exact: true }).click();
    await fillLocalCredential(page, "synthetic-browser-token");
    await page.getByRole("button", { name: "Load reconciliation targets", exact: true }).click();

    await expect(page.getByRole("alert")).toContainText("Vault reconciliation lookup failed; retry or contact an operator.", { timeout: 7_000 });
    await expect(page.getByRole("alert")).not.toContainText("Request timed out");
    await expect(page.getByText("Balance evidence not loaded", { exact: true })).toBeVisible();
    await expectNoBrowserAxeViolations(page, "vault reconciliation timeout state");
  });
});

function finalizedRefundClaimResponse(): Record<string, unknown> {
  const instruction = {
    status: "prepared_unsubmitted",
    program_id: "4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi",
    instruction_data_base64: "eYjG9eQLZXMDAAAAAAAAAAkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJANJJawAAAABY1ElrAAAAAA==",
    accounts: [
      { pubkey: "11157t3sqMV725NVRLrVQbAu98Jjfk1uCKehJnXXQs", is_signer: true, is_writable: true },
      { pubkey: "1117mWrzzrZr312ebPDHu8tbfMwFNvCvMbr6WepCNG", is_signer: true, is_writable: false },
      { pubkey: "1119DWteoLSdjvrT6g6L8C2PfDD2faiTQUpsjY2RiF", is_signer: false, is_writable: false },
      { pubkey: "111BuZ6b86gm7XhxjvTakhRvxSMjXp2GqgifkNUmDK", is_signer: false, is_writable: false },
      { pubkey: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA", is_signer: false, is_writable: false },
      { pubkey: "qcfm6jd37nhqmbY6JomQWtdD9S4HieqnmRm49Hum6ir", is_signer: false, is_writable: false },
      { pubkey: "4fuGSLS5aka44BPDqoH7RSg1HLvy4bMU4zY551u2Dnts", is_signer: false, is_writable: false },
      { pubkey: "Fk3Z5sqgXWRYu3dFuQqshGmgGJoSxQSnxTHSFt9CKM5T", is_signer: false, is_writable: false },
      { pubkey: "111FJo4zLAGU9nzTWa6EnbV4VAmtG4FR8kcokrtZYr", is_signer: false, is_writable: false },
      { pubkey: "111A4R8Vf7xP4Rz7ZkYd8H7f4Q5J4h4Y4V4w4U4T4S", is_signer: false, is_writable: false },
      { pubkey: "HL3Hhyp81PHGgfQQXmAbJy7ZmEi9MJX3ex4UFAunH2g5", is_signer: false, is_writable: false },
      { pubkey: "111B4R8Vf7xP4Rz7ZkYd8H7f4Q5J4h4Y4V4w4U4T4S", is_signer: false, is_writable: false },
      { pubkey: "111C4R8Vf7xP4Rz7ZkYd8H7f4Q5J4h4Y4V4w4U4T4S", is_signer: false, is_writable: false },
      { pubkey: "4DqbTd6EtCebWaXDpCRmoSRVjeHB1k6B3CFYQ89dunAK", is_signer: false, is_writable: true },
      { pubkey: "11111111111111111111111111111111", is_signer: false, is_writable: false },
    ],
    registry: "4fuGSLS5aka44BPDqoH7RSg1HLvy4bMU4zY551u2Dnts",
    adjustment: "Fk3Z5sqgXWRYu3dFuQqshGmgGJoSxQSnxTHSFt9CKM5T",
    original_receipt: "111FJo4zLAGU9nzTWa6EnbV4VAmtG4FR8kcokrtZYr",
    claim: "4DqbTd6EtCebWaXDpCRmoSRVjeHB1k6B3CFYQ89dunAK",
    claim_nonce: "3",
  };
  return {
    status: "finalized",
    intent_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d14",
    request_digest: Array(32).fill(3),
    instruction_digest: Array(32).fill(1),
    artifact_digest: Array(32).fill(2),
    claim: instruction.claim,
    artifact: {
      schema: "refund-movement-artifact/v1",
      instruction,
      instruction_digest: Array(32).fill(1),
      artifact_digest: Array(32).fill(2),
      refund_destination: instruction.accounts[12].pubkey,
      merchant_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d11",
      adjustment_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d12",
      funding_configuration_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d13",
      receipt_signature: "5VfY7WmYcR7h9o9E9nP7pQ8rS",
      receipt_slot: "42",
      proposer_subject: "proposer",
      refund_funder_subject: "refund-funder",
      proposer: instruction.accounts[0].pubkey,
      refund_funder: instruction.accounts[1].pubkey,
      merchant: instruction.accounts[2].pubkey,
      mint: instruction.accounts[3].pubkey,
      token_program: instruction.accounts[4].pubkey,
      tax_vault: instruction.accounts[9].pubkey,
      net_source: instruction.accounts[11].pubkey,
      gross_atomic: "100",
      tax_atomic: "25",
      net_atomic: "75",
      execute_after_unix: 1800000000,
      expires_at_unix: 1800000600,
    },
    signed_transaction_digest: null,
    signature: "5VfY7WmYcR7h9o9E9nP7pQ8rS",
    submitted_slot: "77",
    commitment: "finalized",
    last_error_code: null,
    finalized_evidence: {
      event_type: "AdjustmentRefundClaimCreated",
      event_version: "v1",
      event_discriminator: Array(8).fill(4),
      commitment: "finalized",
      signature: "5VfY7WmYcR7h9o9E9nP7pQ8rS",
      slot: "77",
      event_index: 2,
      claim: Array.from(new PublicKey(instruction.claim).toBytes()),
      adjustment: Array(32).fill(1),
      original_receipt: Array(32).fill(2),
      merchant: Array(32).fill(3),
      mint: Array(32).fill(4),
      nonce: "3",
      gross_magnitude: "100",
      tax_magnitude: "25",
      net_magnitude: "75",
      refund_funder: Array(32).fill(5),
      refund_destination: Array(32).fill(6),
      proposer: Array(32).fill(7),
      reference_digest: Array(32).fill(8),
      execute_after_unix: 1800000000,
      expires_at_unix: 1800000600,
      evidence_digest: Array(32).fill(1),
    },
  };
}
