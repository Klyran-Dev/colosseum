import { expect, test } from "@playwright/test";

test.describe("admin control-plane browser contract", () => {
  test("the documented root enters the protected admin boundary", async ({ page }) => {
    await page.goto("/");
    await expect(page).toHaveURL(/\/auth\/signin\?next=%2Fadmin$/u);
    await expect(page.getByRole("heading", { name: "Sign in to Tax operations", exact: true })).toBeVisible();
    await expect(page.getByLabel("Work email")).toBeVisible();
    await expect(page.getByRole("button", { name: "Continue with email" })).toBeEnabled();
  });

  test("the signup boundary explains server-owned operator provisioning", async ({ page }) => {
    await page.goto("/auth/signup");
    await expect(page.getByRole("heading", { name: "Create your operator account", exact: true })).toBeVisible();
    await expect(page.getByText("Access is provisioned, never self-selected.", { exact: false })).toBeVisible();
    await expect(page.getByLabel("Work email")).toBeVisible();
    await expect(page.getByRole("link", { name: "Back to sign in" })).toBeVisible();
  });

  test("protected sections do not leak a dashboard before authentication", async ({ page }) => {
    await page.goto("/admin/evidence");
    await expect(page).toHaveURL(/\/auth\/signin\?next=%2Fadmin%2Fevidence$/u);
    await expect(page.getByRole("heading", { name: "Sign in to Tax operations", exact: true })).toBeVisible();
    await expect(page.getByText("Recent evidence access", { exact: true })).toHaveCount(0);
    await expect(page.getByText("Verified policy activity chart", { exact: true })).toHaveCount(0);
  });

  test("the auth surface remains usable at the narrow mobile contract width", async ({ page }) => {
    await page.setViewportSize({ width: 320, height: 800 });
    await page.goto("/auth/signup");
    await expect(page.getByRole("heading", { name: "Create your operator account", exact: true })).toBeVisible();
    const width = await page.evaluate(() => ({ scroll: document.documentElement.scrollWidth, viewport: document.documentElement.clientWidth }));
    expect(width.scroll, "auth page must not introduce horizontal scrolling").toBeLessThanOrEqual(width.viewport + 1);
  });
});
