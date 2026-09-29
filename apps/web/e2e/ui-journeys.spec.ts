import { expect, test, type Page } from "@playwright/test";
import axe from "axe-core";

async function expectNoBrowserAxeViolations(
  page: Page,
  surface: string,
): Promise<void> {
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

test.describe("real-browser SPA journeys", () => {
  test("supports keyboard context and real lazy workspace loading", async ({ page }) => {
    await page.goto("/");
    await expect(page.getByText("Calculation service online")).toBeVisible();

    await page.keyboard.press("Tab");
    const skipLink = page.getByRole("link", { name: "Skip to workspace content" });
    await expect(skipLink).toBeFocused();
    await page.keyboard.press("Enter");
    await expect(page.locator("#workspace-content")).toBeFocused();

    const workspaces = [
      ["Rule governance", "Review and promote a rule"],
      ["Replay operations", "Review replay gaps"],
      ["Vault releases", "Propose a vault release claim"],
      ["Vault reconciliation", "Vault reconciliation"],
      ["Settlement adjustments", "Propose an adjustment"],
      ["Refund claims", "Prepare a refund claim"],
      ["Refund executions", "Prepare execution"],
      ["Checkout authorization", "Review and authorize settlement"],
    ] as const;

    for (const [buttonName, headingName] of workspaces) {
      await page.getByRole("button", { name: buttonName, exact: true }).click();
      await expect(page.getByRole("heading", { name: headingName, exact: true })).toBeVisible();
      await expect(page.locator("#workspace-content")).toBeFocused();
    }
  });

  test("has no rendered WCAG A/AA violations across the real SPA workspaces", async ({ page }) => {
    await page.goto("/");
    await expect(page.getByText("Calculation service online")).toBeVisible();
    await expectNoBrowserAxeViolations(page, "production quote");

    const workspaces = [
      ["Rule governance", "Review and promote a rule"],
      ["Replay operations", "Review replay gaps"],
      ["Vault releases", "Propose a vault release claim"],
      ["Vault reconciliation", "Vault reconciliation"],
      ["Settlement adjustments", "Propose an adjustment"],
      ["Refund claims", "Prepare a refund claim"],
      ["Refund executions", "Prepare execution"],
      ["Checkout authorization", "Review and authorize settlement"],
    ] as const;

    for (const [buttonName, headingName] of workspaces) {
      await page.getByRole("button", { name: buttonName, exact: true }).click();
      await expect(page.getByRole("heading", { name: headingName, exact: true })).toBeVisible();
      await expectNoBrowserAxeViolations(page, buttonName);
    }
  });

  test("keeps narrow RTL and reduced-motion display behavior bounded", async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 800 });
    await page.goto("/");

    await page.getByText("Display settings", { exact: true }).click();
    await page.getByRole("combobox", { name: "Language" }).selectOption("ar-EG");

    await expect(page.locator("html")).toHaveAttribute("dir", "rtl");
    await expect(page.locator(".app-shell")).toHaveAttribute("dir", "rtl");
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);

    const transitionDuration = await page.getByRole("button", { name: "Production quote" }).evaluate(
      (element) => getComputedStyle(element).transitionDuration,
    );
    expect(transitionDuration).toBe("0s");
  });

  test("contains a real dynamic-import failure without exposing exception data", async ({ page }) => {
    await page.route("**/assets/GovernancePanel-*.js", (route) => route.abort("failed"));
    await page.goto("/");
    await page.getByRole("button", { name: "Rule governance", exact: true }).click();

    await expect(page.getByRole("heading", { name: "This workspace could not be loaded" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Reload application" })).toBeVisible();
    await expect(page.getByText(/secret|exception|stack|token/i)).toHaveCount(0);
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
    await expect(page.getByText("No calculation yet", { exact: true })).toBeVisible();
    await expect(page.getByRole("button", { name: "Create quote", exact: true })).toBeDisabled();
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
    await expect(page.getByText("Calculation service online")).toBeVisible();
    await page.getByLabel("OIDC access token (memory-only)").fill("synthetic-browser-token");
    await page.getByLabel("Provider-signed jurisdiction evidence (JSON array)").fill("[{}]");
    await page.getByRole("button", { name: "Create quote", exact: true }).click();
    await expect(page.getByText("Calculation ID 01992f1b-4a70-7e30-8ea2-b6e37a9d9d21", { exact: true })).toBeVisible();

    await page.getByRole("button", { name: "Checkout authorization", exact: true }).click();
    await expect(page.getByRole("heading", { name: "Review and authorize settlement", exact: true })).toBeVisible();
    await expect(page.getByText(/expired; create a new quote/)).toBeVisible();
    await expect(page.getByRole("button", { name: "Compile for review", exact: true })).toBeDisabled();
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
    await expect(page.getByText("Calculation service online")).toBeVisible();
    await page.getByRole("button", { name: "Checkout authorization", exact: true }).click();
    await page.getByRole("button", { name: "Connect wallet", exact: true }).click();
    await expect(page.getByRole("alert")).toContainText("User rejected wallet connection");
    await expect(page.getByText("No wallet connected", { exact: true })).toBeVisible();
    await expectNoBrowserAxeViolations(page, "wallet rejection state");
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
    await page.getByRole("button", { name: "Vault reconciliation", exact: true }).click();
    await page.getByLabel("OIDC access token (memory-only)").fill("synthetic-browser-token");
    await page.getByRole("button", { name: "Load reconciliation targets", exact: true }).click();

    await expect(page.getByText("Mismatch — stop and investigate", { exact: true }).first()).toBeVisible();
    await expect(page.getByText("The latest finalized observation does not match the evidence-derived expectation.", { exact: false })).toBeVisible();
    await expect(page.getByText("opening ceremony INC-42", { exact: true })).toBeVisible();

    await page.getByRole("button", { name: /vault-account.*Mismatch/, exact: false }).click();
    await expect(page.getByRole("heading", { name: "Recent runs", exact: true })).toBeVisible();
    await expect(page.getByText("matched", { exact: true })).toBeVisible();
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
    await page.getByRole("button", { name: "Vault reconciliation", exact: true }).click();
    await page.getByLabel("OIDC access token (memory-only)").fill("synthetic-browser-token");
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
    await page.getByRole("button", { name: "Vault reconciliation", exact: true }).click();
    await page.getByLabel("OIDC access token (memory-only)").fill("synthetic-browser-token");
    await page.getByRole("button", { name: "Load reconciliation targets", exact: true }).click();

    await expect(page.getByRole("alert")).toContainText("vault reconciliation access is not authorized");
    await expect(page.getByText("Evidence not loaded", { exact: true })).toBeVisible();
    await expectNoBrowserAxeViolations(page, "vault reconciliation unauthorized state");
  });
});
