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
});
